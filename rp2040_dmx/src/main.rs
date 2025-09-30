#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Executor;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::{I2C1, PIO0, PIO1, DMA_CH0};
use embassy_rp::{bind_interrupts, i2c, i2c_slave};
use embassy_rp::{Peri, pio, pio::Pio};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};
use embassy_futures::select::{select, select3, Either, Either3};
use embassy_time::Timer;
use smart_leds::RGB8;

mod dmx_pio;
use dmx_pio::{PioDmxRx, PioDmxRxProgram};

const DMX_SIZE: usize = 512;
type DmxBuffer = [u8; DMX_SIZE];

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static CHANNEL: Channel<CriticalSectionRawMutex, DmxBuffer, 1> = Channel::new();

static DMX_BUF: StaticCell<DmxBuffer> = StaticCell::new();

bind_interrupts!(struct Irqs {
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    PIO1_IRQ_0 => pio::InterruptHandler<PIO1>;
});

const DEV_ADDR: u8 = 0x33;

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());
    let led = Output::new(p.PIN_25, Level::Low);

    // Neopixel setup
    let Pio {mut common, sm0, ..} = Pio::new(p.PIO1, Irqs);
    let program = PioWs2812Program::new(&mut common);
    let ws2812 = PioWs2812::new(&mut common, sm0, p.DMA_CH2, p.PIN_23, &program);
    
    // I2C setup
    let d_sda = p.PIN_6;
    let d_scl = p.PIN_7;
    let mut config = i2c_slave::Config::default();
    config.addr = DEV_ADDR as u16;
    let i2c_device = i2c_slave::I2cSlave::new(p.I2C1, d_scl, d_sda, Irqs, config);

    // DMX PIO setup
    let Pio {mut common,  sm1, ..} = Pio::new(p.PIO0, Irqs);
    let rx_program = PioDmxRxProgram::new(&mut common);
    let _dma_out_ref: Peri<'static, DMA_CH0> = p.DMA_CH0;
    let dmx_rx: PioDmxRx<'_, PIO0, 1> = PioDmxRx::new(
        // dma_out_ref,
        &mut common,
        sm1,
        p.PIN_2,
        &rx_program
    );

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| spawner.spawn(core1_task(led, i2c_device)).unwrap());
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        spawner.spawn(core0_task(dmx_rx, ws2812)).unwrap();
    });
}

// Read DMX data via PIO
#[embassy_executor::task]
async fn core0_task(
    mut dmx_rx: PioDmxRx<'static, PIO0, 1>,
    mut neo: PioWs2812<'static, PIO1, 0, 1>
) {
    let dmx_data: DmxBuffer = [0_u8; DMX_SIZE];
    let dmx_buf = DMX_BUF.init(dmx_data);
    
    loop {
        // try to read DMX data with timout of 100ms
        match select(dmx_rx.read(dmx_buf), Timer::after_millis(100)).await {
            Either::First(_) => {
                // set LED green if data received
                neo.write(&[RGB8::new(0,15,0)]).await;
                CHANNEL.send(*dmx_buf).await;
                // set LED to blue when data is sent to i2c task
                neo.write(&[RGB8::new(0,0,15)]).await;
            }
            Either::Second(_) => {
                // set LED to red if timeout reached
                neo.write(&[RGB8::new(15,0,0)]).await;
            }
        }
    }
}

// Reply to STM32 data requests
#[embassy_executor::task]
async fn core1_task(mut led: Output<'static>, mut dev: i2c_slave::I2cSlave<'static, I2C1>) {
    let mut dmx_data: DmxBuffer = [0_u8; DMX_SIZE];
    let mut buf = [0u8; 128];

    loop {
        // wait for either new DMX data on CHANNEL or a request on I2C dev
        match select3(CHANNEL.receive(), dev.listen(&mut buf), Timer::after_millis(500)).await {
            Either3::First(incoming_dmx) => {
                dmx_data = incoming_dmx;
            }
            Either3::Second(i2c_request) => {
                match i2c_request {
                    Ok(i2c_slave::Command::GeneralCall(len)) => {
                        info!("Device received general call write: {}", buf[..len]);
                        buf = [0; 128];
                    },
                    Ok(i2c_slave::Command::Read) => loop {
                        match dev.respond_to_read(&dmx_data).await {
                            Ok(x) => match x {
                                i2c_slave::ReadStatus::Done => break,
                                i2c_slave::ReadStatus::NeedMoreBytes => (),
                                i2c_slave::ReadStatus::LeftoverBytes(x) => {
                                    info!("tried to write {} extra bytes", x);
                                    break;
                                }
                            },
                            Err(e) => error!("error while responding {}", e),
                        }
                    },
                    Ok(i2c_slave::Command::Write(len)) => {
                        info!("Device received write: {}", buf[..len]);
                        buf = [0; 128];
                    },
                    Ok(i2c_slave::Command::WriteRead(len)) => {
                        info!("device received write read: {:x}", buf[..len]);
                        match buf[0] {
                            // Set the state
                            0xC2 => {
                                // state = buf[1];
                                match dev.respond_and_fill(&dmx_data, 0x00).await {
                                    Ok(read_status) => info!("response read status {}", read_status),
                                    Err(e) => error!("error while responding {}", e),
                                }
                            }
                            // Reset State
                            0xC8 => {
                                // state = 0;
                                match dev.respond_and_fill(&dmx_data, 0x00).await {
                                    Ok(read_status) => info!("response read status {}", read_status),
                                    Err(e) => error!("error while responding {}", e),
                                }
                            }
                            x => error!("Invalid Write Read {:x}", x),
                        }
                        buf = [0; 128];
                    }
                    Err(e) => error!("{}", e),
                }
            }
            Either3::Third(_) => {
                led.toggle();
            }
        }
    }
}