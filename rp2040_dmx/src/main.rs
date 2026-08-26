//! DMX-512 to I2C bridge firmware for the RP2040.
//!
//! Rust/Embassy reimplementation of the `DMX_on_pico` Arduino sketch,
//! extended with DMX output:
//!
//! * Core 0 runs the DMX port. In **input** mode it receives DMX frames on
//!   GPIO2 through a PIO state machine + DMA and forwards each complete
//!   frame to core 1; "no data!" is logged over USB serial when the signal
//!   disappears, just like the sketch. In **output** mode it continuously
//!   retransmits the most recently written output frame on GPIO4 (~43
//!   packets/s) with the RS-485 driver enabled.
//! * Core 1 acts as an I2C slave (address 0x33, SDA=GPIO6, SCL=GPIO7) for
//!   the STM32 (`nucleo/src/dmx_i2c.rs`):
//!     - block reads 0x01/0x02/0x03 return the received frame in chunks;
//!     - block writes 0x11/0x12/0x13 fill the output frame in the same
//!       chunk layout;
//!     - write 0x20 + mode byte selects the port direction (0 = input,
//!       1 = output).
//!
//! A WS2812 status pixel on GPIO23 shows green while DMX data is flowing,
//! red when the input signal times out, and blue while transmitting.

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU8, Ordering};

use embassy_executor::Executor;
use embassy_futures::select::{Either, select};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::multicore::{Stack, spawn_core1};
use embassy_rp::peripherals::{I2C1, PIO0, PIO1, USB};
use embassy_rp::pio::Pio;
use embassy_rp::pio_programs::ws2812::{Grb, PioWs2812, PioWs2812Program};
use embassy_rp::usb::Driver;
use embassy_rp::{bind_interrupts, dma, i2c, i2c_slave, pio, usb};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::watch::Watch;
use embassy_time::Timer;
use smart_leds::RGB8;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

mod dmx_pio;
use dmx_pio::{DMX_FRAME_SIZE, PioDmxRx, PioDmxRxProgram, PioDmxTx, PioDmxTxProgram};

/// One DMX packet: slot 0 is the start code, slots 1..=512 the channel data.
type DmxFrame = [u8; DMX_FRAME_SIZE];

/// I2C slave address of this device (matches the STM32 master).
const DEV_ADDR: u8 = 0x33;

/// If no complete DMX packet arrives within this window, report signal loss.
/// Matches the 100ms freshness check in the Arduino sketch.
const DMX_TIMEOUT_MS: u64 = 100;

// Block-read commands issued by the STM32 as `write_read(0x33, [cmd], buf)`
// (see nucleo/src/dmx_i2c.rs). The 513-byte frame is served in three chunks
// because the master reads at most 200 bytes at a time.
const CMD_READ_BLOCK_1: u8 = 0x01; // slots [0, 200)
const CMD_READ_BLOCK_2: u8 = 0x02; // slots [200, 400)
const CMD_READ_BLOCK_3: u8 = 0x03; // slots [400, 513)

// Block-write commands, issued as `write(0x33, [cmd, data...])`, filling the
// output frame in the same chunk layout as the reads.
const CMD_WRITE_BLOCK_1: u8 = 0x11; // slots [0, 200), slot 0 = start code
const CMD_WRITE_BLOCK_2: u8 = 0x12; // slots [200, 400)
const CMD_WRITE_BLOCK_3: u8 = 0x13; // slots [400, 513)

/// Direction command: `write(0x33, [0x20, mode])`.
const CMD_SET_DIRECTION: u8 = 0x20;
const DIRECTION_INPUT: u8 = 0x00; // receive DMX on GPIO2 (RS-485 driver disabled)
const DIRECTION_OUTPUT: u8 = 0x01; // transmit DMX on GPIO4 (RS-485 driver enabled)

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

/// Latest complete received DMX frame, core 0 -> core 1. Capacity 1 plus
/// `try_send` keeps the DMX receiver from ever blocking on a busy consumer,
/// replacing the `newDataReady` / inter-core FIFO handshake of the sketch.
static FRAMES: Channel<CriticalSectionRawMutex, DmxFrame, 1> = Channel::new();

/// Latest output frame written by the STM32, core 1 -> core 0 (latest wins).
static OUT_FRAME: Watch<CriticalSectionRawMutex, DmxFrame, 1> = Watch::new();

/// Current port direction (`DIRECTION_INPUT` / `DIRECTION_OUTPUT`), set by
/// core 1 from I2C commands and polled by core 0 between packets. Plain
/// load/store only — thumbv6m has no CAS, and none is needed here.
static DMX_DIRECTION: AtomicU8 = AtomicU8::new(DIRECTION_INPUT);

bind_interrupts!(struct Irqs {
    // 0.10 requires each DMA channel in use to bind a handler on DMA_IRQ_0.
    DMA_IRQ_0 => dma::InterruptHandler<embassy_rp::peripherals::DMA_CH0>,
                 dma::InterruptHandler<embassy_rp::peripherals::DMA_CH1>,
                 dma::InterruptHandler<embassy_rp::peripherals::DMA_CH3>;
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    PIO1_IRQ_0 => pio::InterruptHandler<PIO1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    // Onboard LED, blinks as DMX packets arrive (LED_BUILTIN in the sketch).
    let led = Output::new(p.PIN_25, Level::Low);

    // RS-485 transceiver driver-enable, held low = receive-only.
    let dmx_en = Output::new(p.PIN_3, Level::Low);

    // USB serial logging (the sketch's `Serial.begin(115200)`).
    let usb_driver = Driver::new(p.USB, Irqs);

    // WS2812 status pixel.
    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO1, Irqs);
    let ws_program = PioWs2812Program::new(&mut common);
    let status_pixel = PioWs2812::new(&mut common, sm0, p.DMA_CH1, Irqs, p.PIN_23, &ws_program);

    // I2C1 slave for the STM32. GPIO6 = SDA, GPIO7 = SCL.
    let mut config = i2c_slave::Config::default();
    config.addr = DEV_ADDR as u16;
    let i2c_device = i2c_slave::I2cSlave::new(p.I2C1, p.PIN_7, p.PIN_6, Irqs, config);

    // DMX receiver on GPIO2, transmitter on GPIO4.
    let Pio {
        mut common,
        sm0,
        sm1,
        ..
    } = Pio::new(p.PIO0, Irqs);
    let rx_program = PioDmxRxProgram::new(&mut common);
    let dmx_rx = PioDmxRx::new(&mut common, sm0, p.DMA_CH0, Irqs, p.PIN_2, &rx_program);
    let tx_program = PioDmxTxProgram::new(&mut common);
    let dmx_tx = PioDmxTx::new(&mut common, sm1, p.DMA_CH3, Irqs, p.PIN_4, &tx_program);

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| spawner.spawn(defmt::unwrap!(i2c_task(i2c_device))));
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        spawner.spawn(defmt::unwrap!(logger_task(usb_driver)));
        spawner.spawn(defmt::unwrap!(dmx_task(dmx_rx, dmx_tx, status_pixel, led, dmx_en)));
    });
}

/// Route `log` messages to a USB CDC-ACM serial port.
#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

/// Core 0: run the DMX port in the direction selected over I2C — receive
/// packets and hand them to core 1, or continuously retransmit the latest
/// output frame written by the STM32.
#[embassy_executor::task]
async fn dmx_task(
    mut dmx_rx: PioDmxRx<'static, PIO0, 0>,
    mut dmx_tx: PioDmxTx<'static, PIO0, 1>,
    mut status_pixel: PioWs2812<'static, PIO1, 0, 1, Grb>,
    mut led: Output<'static>,
    mut dmx_en: Output<'static>, // RS-485 driver enable: low = receive, high = transmit
) {
    let mut frame: DmxFrame = [0; DMX_FRAME_SIZE];
    // All channels dark until the STM32 writes an output frame.
    let mut out_frame: DmxFrame = [0; DMX_FRAME_SIZE];
    let mut out_rx = OUT_FRAME.receiver().unwrap();

    loop {
        // Direction changes take effect between packets: within one 100ms
        // read timeout, or one ~23ms transmitted frame.
        if DMX_DIRECTION.load(Ordering::Relaxed) == DIRECTION_OUTPUT {
            if let Some(new_frame) = out_rx.try_changed() {
                out_frame = new_frame;
            }

            dmx_en.set_high();
            dmx_tx.write(&out_frame).await;
            led.toggle();
            status_pixel.write(&[RGB8::new(0, 0, 15)]).await;
        } else {
            dmx_en.set_low();
            match select(dmx_rx.read(&mut frame), Timer::after_millis(DMX_TIMEOUT_MS)).await {
                Either::First(()) => {
                    // Forward the packet, dropping it if core 1 still holds
                    // the previous one (latest data wins in DMX).
                    let _ = FRAMES.try_send(frame);
                    led.toggle();
                    status_pixel.write(&[RGB8::new(0, 15, 0)]).await;
                }
                Either::Second(()) => {
                    // Cancelling the read aborted its DMA transfer; the next
                    // read re-arms the state machine from the BREAK detector.
                    log::info!("no data!");
                    led.set_low();
                    status_pixel.write(&[RGB8::new(15, 0, 0)]).await;
                }
            }
        }
    }
}

/// Core 1: I2C slave serving the latest received DMX frame to the STM32 and
/// accepting output-frame data and direction commands from it.
#[embassy_executor::task]
async fn i2c_task(mut dev: i2c_slave::I2cSlave<'static, I2C1>) {
    let mut frame: DmxFrame = [0; DMX_FRAME_SIZE];
    // Output frame staging area, pushed to core 0 after each block write.
    let mut out_frame: DmxFrame = [0; DMX_FRAME_SIZE];
    let out_tx = OUT_FRAME.sender();
    // Command byte remembered from a plain write, in case the master splits
    // the command write and the block read into two transactions (the
    // Arduino sketch's onReceive/onRequest flow supported both framings).
    let mut pending_cmd: u8 = 0;
    // Large enough for the biggest write: block command + 200 data bytes.
    let mut buf = [0u8; 256];

    loop {
        // Pick up the latest received frame without cancelling listen():
        // embassy-rp's listen() keeps its progress in a local, so racing it
        // against the channel in a select() could garble a transaction that
        // collides with a frame arrival. listen() completes on every master
        // transaction (~100/s while the STM32 polls), so draining the channel
        // between transactions keeps the served frame <= ~30ms stale.
        while let Ok(new_frame) = FRAMES.try_receive() {
            frame = new_frame;
        }

        match dev.listen(&mut buf).await {
            Ok(cmd) => match cmd {
                i2c_slave::Command::WriteRead(_) => {
                    pending_cmd = buf[0];
                    respond_block(&mut dev, pending_cmd, &frame).await;
                }
                i2c_slave::Command::Write(0) => {} // address-only probe (e.g. bus scan)
                i2c_slave::Command::Write(1) => {
                    pending_cmd = buf[0]; // answered by a following read
                }
                i2c_slave::Command::Write(len) => {
                    handle_data_write(buf[0], &buf[1..len], &mut out_frame, &out_tx);
                }
                i2c_slave::Command::Read => {
                    respond_block(&mut dev, pending_cmd, &frame).await;
                }
                i2c_slave::Command::GeneralCall(len) => {
                    log::warn!("unexpected I2C general call ({len} bytes)");
                }
            },
            Err(e) => log::error!("I2C listen error: {e:?}"),
        }
    }
}

/// Apply a multi-byte I2C write: output-frame block data or a direction change.
fn handle_data_write(
    cmd: u8,
    data: &[u8],
    out_frame: &mut DmxFrame,
    out_tx: &embassy_sync::watch::Sender<'static, CriticalSectionRawMutex, DmxFrame, 1>,
) {
    let dest: &mut [u8] = match cmd {
        CMD_WRITE_BLOCK_1 => &mut out_frame[0..200],
        CMD_WRITE_BLOCK_2 => &mut out_frame[200..400],
        CMD_WRITE_BLOCK_3 => &mut out_frame[400..513],
        CMD_SET_DIRECTION => {
            match data[0] {
                DIRECTION_INPUT | DIRECTION_OUTPUT => {
                    DMX_DIRECTION.store(data[0], Ordering::Relaxed);
                    log::info!("DMX port direction set to {}", if data[0] == DIRECTION_OUTPUT { "output" } else { "input" });
                }
                other => log::warn!("invalid DMX direction {other:#04x}"),
            }
            return;
        }
        other => {
            log::warn!("unknown I2C write command {other:#04x} ({} bytes)", data.len());
            return;
        }
    };

    // Tolerate short writes (partial block update); ignore excess bytes.
    let n = data.len().min(dest.len());
    dest[..n].copy_from_slice(&data[..n]);
    // Publish the whole staged frame; core 0 picks up the latest between packets.
    out_tx.send(*out_frame);
}

/// Answer a block-read command with the matching chunk of the DMX frame.
async fn respond_block(dev: &mut i2c_slave::I2cSlave<'static, I2C1>, cmd: u8, frame: &DmxFrame) {
    let block: &[u8] = match cmd {
        CMD_READ_BLOCK_1 => &frame[0..200],
        CMD_READ_BLOCK_2 => &frame[200..400],
        CMD_READ_BLOCK_3 => &frame[400..513],
        other => {
            // Still answer (with zeroes) so the master is never left
            // clock-stretched waiting on an empty TX FIFO.
            log::warn!("unknown I2C read command {other:#04x}");
            &[0]
        }
    };

    // respond_and_fill pads with zeroes if the master reads past the end of
    // the block, so a length mismatch can't stall the bus.
    match dev.respond_and_fill(block, 0x00).await {
        Ok(i2c_slave::ReadStatus::LeftoverBytes(n)) => {
            log::warn!("I2C master stopped reading {n} bytes early");
        }
        Ok(_) => {}
        Err(e) => log::error!("I2C respond error: {e:?}"),
    }
}
