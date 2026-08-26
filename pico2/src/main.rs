//! DMX interface firmware — W6300-EVB-Pico2 target (RP2350 + W6300).
//!
//! Replaces the STM32H563 Nucleo build. Everything target-agnostic lives in the
//! `common` crate and is shared with `nucleo/` and `host_tests/`; this crate is
//! only the peripheral setup, the PIO programs and the executor wiring.
//!
//! See `../REV2_PLAN.md` for the design and `../docs/` for the diagrams.
//!
//! # Core split
//!
//! * **Core 0** — W6300 + smoltcp, Art-Net / sACN, the event router.
//! * **Core 1** — OLED UI, buttons, wired DMX, LED render.
//!
//! The split exists so network load cannot make the UI laggy: a console
//! broadcasting universes we do not care about still has to be ingested and
//! discarded, and that work stays off the core driving the display.

#![no_std]
#![no_main]

extern crate alloc;

// The ported sources (artnet, event_router) use `crate::`-rooted paths. Mirror
// nucleo's re-export shape so they compile without edits.
pub use common::constants::*;
pub use common::{channels, enttec_protocol, ui, DMX_BUFFER, LED_COLORS};

use defmt::*;
use defmt_rtt as _;
use embassy_executor::{Executor, Spawner};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_net::StackResources;
use embassy_rp::i2c::{self, I2c, InterruptHandler as I2cInterruptHandler};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_rp::clocks::RoscRng;
use embassy_rp::dma::InterruptHandler as DmaInterruptHandler;
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::peripherals::{
    DMA_CH0, DMA_CH1, DMA_CH10, DMA_CH11, DMA_CH2, DMA_CH3, DMA_CH4, DMA_CH5, DMA_CH6,
    DMA_CH7, DMA_CH8, DMA_CH9, I2C1, PIO0, PIO1, PIO2, USB,
};
use embassy_rp::pio_programs::spi::Spi as PioSpi;
use embassy_rp::spi::{Config as SpiConfig, Spi as HwSpi};
use embedded_hal_bus::spi::ExclusiveDevice;
use static_cell::StaticCell;
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio};
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_rp::multicore::{spawn_core1, Stack as CoreStack};
use embassy_time::{Delay, Timer};
use panic_probe as _;

mod artnet;
mod buttons;
mod console_usb;
mod dmx;
mod dmx_pio;
mod eeprom;
mod event_router;
mod m24x02;
mod smart_led;
mod tca9555;
mod oled_ui;
mod sacn_rx;
mod usb_device;
mod w6300;
use smart_led::Ws2812Outputs;

/// Program metadata for `picotool info`.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"DMX Interface"),
    embassy_rp::binary_info::rp_program_description!(
        c"Art-Net / sACN / DMX to 8x WS2812, W6300-EVB-Pico2"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

/// Heap for `common::ui::fields`, which formats menu strings, and for the
/// Ratatui cell buffers behind the OLED.
///
/// 16 KB against nucleo's 160 KB: the mono 128x64 framebuffer is 1 KB where the
/// 320x172 TFT needed ~108 KB, and the 21x8 character grid is a fraction of the
/// old 35x11. Revisit once the Phase 3 UI is real.
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

const HEAP_SIZE: usize = 16 * 1024;

bind_interrupts!(struct Irqs {
    I2C1_IRQ => I2cInterruptHandler<I2C1>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    PIO1_IRQ_0 => PioInterruptHandler<PIO1>;
    PIO2_IRQ_0 => PioInterruptHandler<PIO2>;
    // Every DMA channel shares DMA_IRQ_0, so each one used needs its own
    // handler bound to it. embassy-rp 0.10 made the DMA irq binding an explicit
    // argument to the PIO drivers.
    DMA_IRQ_0 => DmaInterruptHandler<DMA_CH0>, DmaInterruptHandler<DMA_CH1>,
                 DmaInterruptHandler<DMA_CH2>, DmaInterruptHandler<DMA_CH3>,
                 DmaInterruptHandler<DMA_CH4>, DmaInterruptHandler<DMA_CH5>,
                 DmaInterruptHandler<DMA_CH6>, DmaInterruptHandler<DMA_CH7>,
                 DmaInterruptHandler<DMA_CH8>, DmaInterruptHandler<DMA_CH9>,
                 DmaInterruptHandler<DMA_CH10>, DmaInterruptHandler<DMA_CH11>;
});

/// Pin allocation — see `docs/rev2-pinmap.svg`.
///
/// GP15–22 are consumed by the W6300 inside the module and cannot be reused.
/// GP23/24/25/29 are not on the header at all: the Pico 2 keeps four GPIOs
/// internal (SMPS `PS`, VBUS sense, user LED, VSYS/3 ADC). That leaves GP13 as
/// the only spare, broken out to a test point.
pub mod pins {
    /// WS2812 data out, one per string, GP0–GP7.
    pub const WS2812: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

    pub const DMX_RX: u8 = 8;
    pub const DMX_TX: u8 = 9;
    /// Driver enable / receiver enable, tied together. Must stay on a real
    /// GPIO — it switches per frame against PIO output timing.
    pub const DMX_DE: u8 = 10;

    /// SSD1306 on SPI1. `CS` ties low (sole device on the bus) and `RES` is on
    /// the I/O expander, which is what keeps GP13 free.
    pub const OLED_MOSI: u8 = 11; // SPI1 TX
    pub const OLED_DC: u8 = 12; // plain GPIO, toggles per transaction
    pub const OLED_SCK: u8 = 14; // SPI1 SCK

    /// The only spare GPIO. Also SPI1 CSn if a second SPI device appears.
    pub const SPARE: u8 = 13;

    /// Shared I²C1: M24C02 EEPROM at 0x56, TCA9555 expander at 0x20.
    pub const I2C_SDA: u8 = 26;
    pub const I2C_SCL: u8 = 27;
    /// TCA9555 interrupt, open-drain with a 10 kΩ pull-up.
    pub const EXPANDER_INT: u8 = 28;
}

/// MAC used when the EEPROM has none programmed.
///
/// Locally-administered (bit 1 of the first octet set), so it is valid on a
/// private network. **Two unprogrammed boards on one bench would collide** —
/// which is exactly why boot prefers the EEPROM copy at 0x02–0x07 and only
/// falls back to this.
const MAC_FALLBACK: [u8; 6] = [0x02, 0x44, 0x4d, 0x58, 0x00, 0x01];

/// Shared I²C1 bus: the TCA9555 at 0x20 and the M24C02 at 0x56.
type I2cBus = Mutex<CriticalSectionRawMutex, I2c<'static, I2C1, i2c::Async>>;
type I2cDev = I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, I2C1, i2c::Async>>;

/// Released-OLED-reset handshake: signalled by the button task (which owns the
/// expander that drives the panel's RES line) and awaited by the UI task
/// before it sends the SSD1306 init sequence. Without it, the blocking init
/// runs while the panel is still held in reset and the display stays dark.
static OLED_READY: embassy_sync::signal::Signal<CriticalSectionRawMutex, ()> =
    embassy_sync::signal::Signal::new();

/// Concrete wrapper — embassy tasks cannot be generic, so the generic driver in
/// `buttons` is instantiated here.
#[embassy_executor::task]
async fn button_task(
    expander: tca9555::Tca9555<I2cDev>,
    int: Input<'static>,
    tx: common::channels::RouterChannelTx,
) -> ! {
    buttons::run(expander, int, tx, &OLED_READY).await
}

/// Heartbeat on the module's user LED, so a board that is alive but not
/// networking is distinguishable from one that is not running at all.
#[embassy_executor::task]
async fn heartbeat(mut led: Output<'static>) {
    loop {
        led.set_high();
        Timer::after_millis(50).await;
        led.set_low();
        Timer::after_millis(950).await;
    }
}

/// Core 1's stack. Task futures live in static task storage, not here, so this
/// only has to cover the executor poll loop and ordinary call depth.
static mut CORE1_STACK: CoreStack<8192> = CoreStack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

/// Boot orchestration on core 0 — the ladder the STM32 build ran from `main`.
///
/// 1. Load persisted state **through the EEPROM task**, so the results flow
///    to the router — which forwards settings to the UI and broadcasts the
///    operating mode to every input task. (Without this the router never sees
///    `BootStatus::Success` and deliberately renders nothing.)
/// 2. Arm the boot flag (write `Failed`): if this boot wedges before the
///    `Success` write at the end, the NEXT boot sees `Failed` and skips the
///    network — the lockout guard, unchanged from the STM32 build. The first
///    boot after flashing a blank EEPROM therefore runs without Ethernet;
///    it comes up on the following boot.
/// 3. Bring the network up, gated on the guard and on the `ethernet_enabled`
///    setting ("Ethernet (reboot)" on the System page).
/// 4. Record the successful boot, retried and confirmed via the router.
#[embassy_executor::task]
async fn boot_task(
    spawner: Spawner,
    ee: m24x02::M24x02<I2cDev>,
    spi_dev: w6300::SpiDev,
    int: Input<'static>,
    reset: Output<'static>,
) {
    use common::event_router::{MainEvent, ReturnChannel, RouterEvent};
    use common::events::EepromEvent;
    use common::ui::BootStatus;
    use embassy_time::{with_timeout, Duration};

    channels::CHANNEL_EEPROM.send(EepromEvent::ReadBootStatus).await;
    Timer::after_millis(50).await;
    channels::CHANNEL_EEPROM.send(EepromEvent::ReadModuleType).await;
    Timer::after_millis(50).await;
    channels::CHANNEL_EEPROM.send(EepromEvent::ReadSettings).await;
    Timer::after_millis(50).await;

    let _ = channels::CHANNEL.try_send(RouterEvent::GetBootStatus(ReturnChannel::Main));
    let prev_boot = match with_timeout(Duration::from_millis(250), channels::CHANNEL_MAIN.receiver().receive()).await {
        Ok(MainEvent::ReturnBootStatus(Some(s))) => s,
        _ => BootStatus::Failed,
    };
    let _ = channels::CHANNEL.try_send(RouterEvent::GetSettings(ReturnChannel::Main));
    let settings = match with_timeout(Duration::from_millis(250), channels::CHANNEL_MAIN.receiver().receive()).await {
        Ok(MainEvent::ReturnSettings(s)) => s,
        _ => Default::default(),
    };
    info!("boot: previous boot {:?}", prev_boot);

    channels::CHANNEL_EEPROM.send(EepromEvent::WriteBootStatus(BootStatus::Failed)).await;
    Timer::after_millis(50).await;

    if prev_boot != BootStatus::Success {
        warn!("boot: previous boot incomplete - Ethernet skipped this boot (lockout guard)");
    } else if !settings.ethernet_enabled {
        info!("boot: Ethernet disabled in settings");
    } else {
        net_bringup(spawner, ee, spi_dev, int, reset).await;
    }

    // DMX -> LED rendering is gated on the router seeing Success; retry the
    // write and confirm it round-tripped (the EEPROM task echoes a successful
    // write as StoreBootStatus).
    for attempt in 1..=3u8 {
        channels::CHANNEL_EEPROM.send(EepromEvent::WriteBootStatus(BootStatus::Success)).await;
        Timer::after_millis(50).await;
        let _ = channels::CHANNEL.try_send(RouterEvent::GetBootStatus(ReturnChannel::Main));
        if let Ok(MainEvent::ReturnBootStatus(Some(BootStatus::Success))) =
            with_timeout(Duration::from_millis(250), channels::CHANNEL_MAIN.receiver().receive()).await
        {
            info!("boot: complete");
            return;
        }
        error!("boot: success flag write attempt {} not confirmed", attempt);
    }
    error!("boot: flag could not be stored - EEPROM unreachable? Output stays blocked.");
}

/// Network bring-up: W6300 reset, MAC from the EEPROM, DHCP, then the
/// Art-Net and sACN receivers.
async fn net_bringup(
    spawner: Spawner,
    mut ee: m24x02::M24x02<I2cDev>,
    spi_dev: w6300::SpiDev,
    int: Input<'static>,
    reset: Output<'static>,
) {
    let mac = eeprom::read_mac(&mut ee).await.unwrap_or_else(|| {
        warn!("using fallback MAC - program the EEPROM before shipping");
        MAC_FALLBACK
    });

    static W6300_STATE: StaticCell<w6300::W6300State> = StaticCell::new();
    let Some((device, eth_runner)) =
        w6300::init(mac, W6300_STATE.init(w6300::W6300State::new()), spi_dev, int, reset).await
    else {
        // Deliberately not fatal: DMX, USB and the LEDs all still work without a
        // network, and the boot-flag / lockout logic depends on reaching this
        // point. The cause has already been logged specifically.
        warn!("continuing without Ethernet");
        return;
    };

    spawner.spawn(unwrap!(w6300::w6300_task(eth_runner)));

    static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
    let (stack, net_runner) = embassy_net::new(
        device,
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        RoscRng.next_u64(),
    );
    spawner.spawn(unwrap!(w6300::net_task(net_runner)));
    spawner.spawn(unwrap!(artnet::artnet_task(
        stack,
        channels::CHANNEL_DMX.sender(),
        channels::CHANNEL.sender(),
        unwrap!(channels::CHANNEL_DMX_FEEDBACK.receiver()),
    )));
    spawner.spawn(unwrap!(sacn_rx::sacn_task(
        stack,
        channels::CHANNEL_DMX.sender(),
        channels::CHANNEL.sender(),
        unwrap!(channels::CHANNEL_DMX_FEEDBACK.receiver()),
    )));
}

#[cortex_m_rt::entry]
fn main() -> ! {
    // Before anything can allocate.
    {
        use core::mem::MaybeUninit;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

    let p = embassy_rp::init(Default::default());
    info!("DMX interface starting on RP2350");

    // Everything is constructed here, on core 0, and then moved into whichever
    // executor owns it. Peripherals cannot be split after the cores start.

    // ---- I2C1: EEPROM + expander, shared across both cores ---------------
    // The mutex is CriticalSectionRawMutex precisely because core 0 (EEPROM)
    // and core 1 (expander) both reach this bus.
    let mut i2c_cfg = i2c::Config::default();
    i2c_cfg.frequency = 400_000;
    let i2c = I2c::new_async(p.I2C1, p.PIN_27, p.PIN_26, Irqs, i2c_cfg);
    static I2C_BUS: StaticCell<I2cBus> = StaticCell::new();
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));

    let eeprom_dev = m24x02::M24x02::new(I2cDevice::new(i2c_bus), eeprom::ADDR);
    let expander = tca9555::Tca9555::new(I2cDevice::new(i2c_bus), tca9555::ADDR);

    // ---- PIO -------------------------------------------------------------
    // PIO0 SM0 is the W6300 transport; PIO2 SM0/SM1 are wired DMX. Everything
    // else is WS2812. See smart_led.rs for the allocation table.
    let Pio { common: mut pio0, sm0: p0sm0, sm1: p0sm1, sm2: p0sm2, sm3: p0sm3, .. } =
        Pio::new(p.PIO0, Irqs);
    let Pio { common: mut pio1, sm0: p1sm0, sm1: p1sm1, sm2: p1sm2, sm3: p1sm3, .. } =
        Pio::new(p.PIO1, Irqs);
    let Pio { common: mut pio2, sm0: p2sm0, sm1: p2sm1, sm2: p2sm2, .. } =
        Pio::new(p.PIO2, Irqs);

    // One copy of the 4-instruction WS2812 program per block, shared by that
    // block's state machines.
    let ws_p0 = PioWs2812Program::new(&mut pio0);
    let ws_p1 = PioWs2812Program::new(&mut pio1);
    let ws_p2 = PioWs2812Program::new(&mut pio2);

    let outputs = Ws2812Outputs {
        s1: PioWs2812::new(&mut pio0, p0sm1, p.DMA_CH0, Irqs, p.PIN_0, &ws_p0),
        s2: PioWs2812::new(&mut pio0, p0sm2, p.DMA_CH1, Irqs, p.PIN_1, &ws_p0),
        s3: PioWs2812::new(&mut pio0, p0sm3, p.DMA_CH2, Irqs, p.PIN_2, &ws_p0),
        s4: PioWs2812::new(&mut pio1, p1sm0, p.DMA_CH3, Irqs, p.PIN_3, &ws_p1),
        s5: PioWs2812::new(&mut pio1, p1sm1, p.DMA_CH4, Irqs, p.PIN_4, &ws_p1),
        s6: PioWs2812::new(&mut pio1, p1sm2, p.DMA_CH5, Irqs, p.PIN_5, &ws_p1),
        s7: PioWs2812::new(&mut pio1, p1sm3, p.DMA_CH6, Irqs, p.PIN_6, &ws_p1),
        s8: PioWs2812::new(&mut pio2, p2sm2, p.DMA_CH7, Irqs, p.PIN_7, &ws_p2),
    };

    // ---- Wired DMX on PIO2 ------------------------------------------------
    let dmx_rx_prog = dmx_pio::PioDmxRxProgram::new(&mut pio2);
    let dmx_rx = dmx_pio::PioDmxRx::new(&mut pio2, p2sm0, p.DMA_CH10, Irqs, p.PIN_8, &dmx_rx_prog);
    let dmx_tx_prog = dmx_pio::PioDmxTxProgram::new(&mut pio2);
    let dmx_tx = dmx_pio::PioDmxTx::new(&mut pio2, p2sm1, p.DMA_CH11, Irqs, p.PIN_9, &dmx_tx_prog);

    // ---- W6300 transport on PIO0 SM0 -------------------------------------
    // SCLK is on GP17, a CSn mux position, so hardware SPI0 cannot reach it.
    let mut spi_cfg = SpiConfig::default();
    spi_cfg.frequency = w6300::SPI_FREQ_HZ;
    let spi_bus = PioSpi::new(
        &mut pio0, p0sm0, p.PIN_17, // SCLK
        p.PIN_18,                   // IO0 / MOSI
        p.PIN_19,                   // IO1 / MISO
        p.DMA_CH8, p.DMA_CH9, Irqs, spi_cfg,
    );
    let spi_dev = ExclusiveDevice::new(spi_bus, Output::new(p.PIN_16, Level::High), Delay)
        .expect("W6300 chip select");

    // ---- SSD1306 on SPI1 --------------------------------------------------
    // Blocking, because display-interface-spi has no async variant. CS is tied
    // low on the carrier (sole device on the bus), which is what keeps GP13 free.
    let mut oled_cfg = SpiConfig::default();
    oled_cfg.frequency = 8_000_000;
    let oled = oled_ui::build(
        HwSpi::new_blocking_txonly(p.SPI1, p.PIN_14, p.PIN_11, oled_cfg),
        Output::new(p.PIN_12, Level::Low), // DC
    );

    // ---- Core 1: UI, buttons, DMX, LED render ----------------------------
    // Everything here is periodic or human-paced. Keeping it off core 0 is what
    // stops a console broadcasting universes we do not want from making the
    // panel laggy — the claim the whole split exists to make good on.
    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|s| {
                s.spawn(unwrap!(button_task(
                    expander,
                    Input::new(p.PIN_28, Pull::Up), // INT, open-drain + 10k
                    channels::CHANNEL.sender(),
                )));
                s.spawn(unwrap!(dmx::dmx_task(
                    dmx_rx,
                    dmx_tx,
                    Output::new(p.PIN_10, Level::Low), // DE//RE, low = receive
                    unwrap!(channels::CHANNEL_DMX_FEEDBACK.receiver()),
                )));
                s.spawn(unwrap!(smart_led::smart_led_task(
                    outputs,
                    channels::CHANNEL_SMART_LED.receiver(),
                )));
                s.spawn(unwrap!(oled_ui::ui_task(
                    oled,
                    channels::CHANNEL_UI.receiver(),
                    channels::CHANNEL.sender(),
                    &OLED_READY,
                )));
            });
        },
    );

    // ---- Core 0: network and the event router ----------------------------
    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|s| {
        s.spawn(unwrap!(heartbeat(Output::new(p.PIN_25, Level::Low))));

        // The hub: routes settings, button and EEPROM events, and maps
        // DMX_BUFFER into LED_COLORS on every packet before pinging the LED
        // task on core 1.
        s.spawn(unwrap!(event_router::event_router(event_router::Router::new(
            channels::CHANNEL.receiver(),
            channels::CHANNEL_DMX.receiver(),
            channels::CHANNEL_DMX_FEEDBACK.sender(),
            channels::CHANNEL_PWM_I2C.sender(),
            channels::CHANNEL_SMART_LED.sender(),
            channels::CHANNEL_UI.sender(),
            channels::CHANNEL_EEPROM.sender(),
            channels::CHANNEL_MAIN.sender(),
        ))));

        // USB composite device: the Enttec DMX USB Pro widget plus the console
        // line protocol `dmx_console` talks to. An input like the network, so it
        // sits alongside it rather than competing with the UI on core 1.
        s.spawn(unwrap!(usb_device::usb_device_task(
            UsbDriver::new(p.USB, Irqs),
            channels::CHANNEL_DMX.sender(),
            unwrap!(channels::CHANNEL_DMX_FEEDBACK.receiver()),
            channels::CHANNEL.sender(),
            unwrap!(channels::CHANNEL_LOG.receiver()),
        )));

        // Settings persistence. Shares I2C1 with the expander on core 1 — which
        // is what the CriticalSectionRawMutex on that bus is for.
        s.spawn(unwrap!(eeprom::eeprom_task(
            I2cDevice::new(i2c_bus),
            eeprom::ADDR,
            channels::CHANNEL_EEPROM.receiver(),
            channels::CHANNEL.sender(),
        )));

        s.spawn(unwrap!(boot_task(
            s,
            eeprom_dev,
            spi_dev,
            Input::new(p.PIN_15, Pull::Up), // W6300 INT
            Output::new(p.PIN_22, Level::High), // W6300 RSTn
        )));
    });

    // Remaining Phase 3 work:
    //   - byte-budget enforcement surfaced in the UI (over_budget() exists)
    //   - RGBW output format (the PIO drivers are 24-bit GRB; the menu option
    //     currently renders the RGB portion and warns once)
    //   - DMX TX task: ArtNet>DMX / USB>DMX modes drive PioDmxTx + GP10 high
}
