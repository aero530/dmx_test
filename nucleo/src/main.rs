#![no_std]
#![no_main]

use cfg_if::cfg_if;
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Input, Level, Output, OutputOpenDrain, Pull, Speed};
use embassy_stm32::i2c::{Config as I2cConfig, I2c, Master};
use embassy_stm32::time::Hertz;
use embassy_stm32::usb::Driver;
// use embassy_stm32::usart::{Config as UsartConfig, DataBits, StopBits, Uart};
use embassy_stm32::spi::{Config as SpiConfig, Mode as SpiMode, Phase, Polarity, Spi};
use embassy_stm32::{bind_interrupts, i2c, peripherals, usb, Config};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Duration;

use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

// Eth
cfg_if! {
    if #[cfg(feature = "ethernet")] {
        use embassy_net::StackResources;
        use embassy_stm32::eth::generic_smi::GenericSMI;
        use embassy_stm32::eth::{Ethernet, PacketQueue};
        use embassy_stm32::rng::Rng;
        use embassy_stm32::{eth, rng};

        mod artnet;
        use crate::artnet::artnet_task;
    }
}

mod pwm_i2c;
use pwm_i2c::pwm_i2c_task;

mod constants;
pub use constants::*;

// -----

// PWM Chip
// PCA9685

// https://www.st.com/en/microcontrollers-microprocessors/stm32f746zg.html
// https://www.st.com/en/evaluation-tools/nucleo-f746zg.html

// https://www.st.com/en/evaluation-tools/nucleo-h563zi.html
// https://www.st.com/en/microcontrollers-microprocessors/stm32h563zi.html

// https://www.st.com/en/evaluation-tools/nucleo-h533re.html
// https://www.st.com/en/microcontrollers-microprocessors/stm32h533re.html

mod button;
use button::button_task;

mod button_array;
use button_array::button_row_task;

mod usb_io;
use usb_io::usb_task;

mod event_router;
use event_router::{event_router, Router};

// mod led;
// use led::led_task;

mod logger;
use logger::log_task;

mod channels;
use channels::*;

mod ansi;

// mod dmx;
// use dmx::dmx_task;

mod dmx_i2c;
use dmx_i2c::dmx_task;

mod smart_led;
use smart_led::smart_led_task;

mod ui;
use ui::ui_task_spi;

type I2c1Bus = Mutex<NoopRawMutex, I2c<'static, embassy_stm32::mode::Async, Master>>;

/// Display I2C / Smbus - I2C2
/// SCL: PF1
/// SDA: PF0
/// Alert#: PF2
/// Reset: PF3
static I2C_BUS_DISPLAY: StaticCell<I2c1Bus> = StaticCell::new();

/// DMX I2C / Smbus - I2C1
/// SCL: PB8
/// SDA: PB9
/// Alert#: PB5
/// Reset: PA3
static I2C_BUS_DMX: StaticCell<I2c1Bus> = StaticCell::new();

/// LED (output) I2C / Smbus - I2C4
static I2C_BUS_LED: StaticCell<I2c1Bus> = StaticCell::new();

bind_interrupts!(struct Irqs {
    USB_DRD_FS => usb::InterruptHandler<peripherals::USB>;
    // USART6 => usart::InterruptHandler<peripherals::USART6>;
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
    I2C2_EV => i2c::EventInterruptHandler<peripherals::I2C2>;
    I2C2_ER => i2c::ErrorInterruptHandler<peripherals::I2C2>;
    I2C4_EV => i2c::EventInterruptHandler<peripherals::I2C4>;
    I2C4_ER => i2c::ErrorInterruptHandler<peripherals::I2C4>;

    #[cfg(feature = "ethernet")]
    RNG => rng::InterruptHandler<peripherals::RNG>;
    #[cfg(feature = "ethernet")]
    ETH => eth::InterruptHandler;

});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hsi = None;
        // config.rcc.hsi48 = Some(Default::default()); // needed for RNG
        config.rcc.hsi48 = Some(Hsi48Config { sync_from_usb: true }); // needed for USB
        config.rcc.hse = Some(Hse {
            // High speed external clock
            freq: Hertz(8_000_000), // 4 - 26MHz
            mode: HseMode::BypassDigital,
        });

        config.rcc.csi = true; // enable CSI clock

        // System
        // PLL1Q -> Ethernet
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV2,
            mul: PllMul::MUL125,
            divp: Some(PllDiv::DIV2), // PLL P divisor => pll_src / prediv * mul / divp = 8mhz / 2 * 125 / 2 = 250Mhz
            divq: Some(PllDiv::DIV2), // PLL Q divisor => pll_src / prediv * mul / divp = 8mhz / 2 * 215 / 2 = 250Mhz
            divr: None,
        });
        // SPI
        config.rcc.pll2 = Some(Pll {
            source: PllSource::CSI, // 4 MHz
            prediv: PllPreDiv::DIV2,
            mul: PllMul::MUL100,
            divp: Some(PllDiv::DIV2), // PLL P divisor => pll_src / prediv * mul / divp = 4mhz / 2 * 100 / 2 = 100Mhz
            divq: Some(PllDiv::DIV2), // PLL Q divisor => pll_src / prediv * mul / divp = 4mhz / 2 * 100 / 2 = 100Mhz
            divr: None,
        });
        // I2C
        config.rcc.pll3 = Some(Pll {
            source: PllSource::CSI, // 4 MHz
            prediv: PllPreDiv::DIV2,
            mul: PllMul::MUL100,
            divp: Some(PllDiv::DIV2), // PLL P divisor => pll_src / prediv * mul / divp = 4mhz / 2 * 100 / 2 = 100Mhz
            divq: Some(PllDiv::DIV2), // PLL Q divisor => pll_src / prediv * mul / divp = 4mhz / 2 * 100 / 2 = 100Mhz
            divr: Some(PllDiv::DIV2), // PLL R divisor => pll_src / prediv * mul / divp = 4mhz / 2 * 100 / 2 = 100Mhz
        });

        config.rcc.ahb_pre = AHBPrescaler::DIV2;
        config.rcc.apb1_pre = APBPrescaler::DIV4;
        config.rcc.apb2_pre = APBPrescaler::DIV2;
        config.rcc.apb3_pre = APBPrescaler::DIV4;
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.voltage_scale = VoltageScale::Scale0;

        // config.rcc.mux.rtcsel = mux::Rtcsel::LSI;
        // config.rcc.mux.uart4sel = mux::Usartsel;
        // config.rcc.mux.uart5sel = mux::Usartsel;
        // config.rcc.mux.uart7sel = mux::Usartsel;
        // config.rcc.mux.uart8sel = mux::Usartsel;
        // config.rcc.mux.uart9sel = mux::Usartsel;
        // config.rcc.mux.usart10sel = mux::Usartsel;
        // config.rcc.mux.usart1sel = mux::Usart1sel;
        // config.rcc.mux.usart2sel = mux::Usartsel;
        // config.rcc.mux.usart3sel = mux::Usartsel;
        // config.rcc.mux.usart6sel = mux::Usartsel;
        // config.rcc.mux.lptim1sel = mux::Lptimsel;
        // config.rcc.mux.lptim2sel = mux::Lptim2sel;
        // config.rcc.mux.lptim3sel = mux::Lptimsel;
        // config.rcc.mux.lptim4sel = mux::Lptimsel;
        // config.rcc.mux.lptim5sel = mux::Lptimsel;
        // config.rcc.mux.lptim6sel = mux::Lptimsel;
        // config.rcc.mux.usart11sel = mux::Usartsel;
        // config.rcc.mux.lpuart1sel = mux::Lpusartsel;
        config.rcc.mux.spi1sel = mux::Spi1sel::PLL2_P;
        config.rcc.mux.spi2sel = mux::Spi2sel::PLL2_P;
        config.rcc.mux.spi3sel = mux::Spi3sel::PLL2_P;
        config.rcc.mux.spi4sel = mux::Spi4sel::PLL2_Q;
        config.rcc.mux.spi5sel = mux::Spi5sel::PLL2_Q;
        config.rcc.mux.spi6sel = mux::Spi6sel::PLL2_Q;
        config.rcc.mux.i2c1sel = mux::I2csel::PLL3_R;
        config.rcc.mux.i2c2sel = mux::I2csel::PLL3_R;
        // config.rcc.mux.i2c3sel = mux::I2c34sel;
        config.rcc.mux.i2c4sel = mux::I2c34sel::PLL3_R;
        // config.rcc.mux.i3c1sel = mux::I2csel;
        // config.rcc.mux.octospi1sel = mux::Octospisel;
        // config.rcc.mux.sdmmc1sel = mux::Sdmmcsel;
        // config.rcc.mux.sdmmc2sel = mux::Sdmmcsel;
        config.rcc.mux.usbsel = mux::Usbsel::HSI48;
        // config.rcc.mux.adcdacsel = mux::Adcdacsel;
        // config.rcc.mux.cecsel = mux::Cecsel;
        // config.rcc.mux.fdcan12sel = mux::Fdcansel;
        config.rcc.mux.persel = mux::Persel::HSI;
        config.rcc.mux.rngsel = mux::Rngsel::HSI48;
        // config.rcc.mux.sai1sel = mux::Saisel;
        // config.rcc.mux.sai2sel = mux::Saisel;
    }
    let p = embassy_stm32::init(config);

    // -----------------------------------
    // Configure I2C for module led board devices
    // -----------------------------------
    // LED (output) I2C / Smbus - I2C4
    // SCL: PF5
    // SDA: PF15
    // Alert#: PF13
    // Reset: PF14
    let mut cfg: I2cConfig = I2cConfig::default();
    cfg.frequency = Hertz(1_000_000);
    let i2c_led = I2c::new(p.I2C4, p.PF5, p.PF15, Irqs, p.GPDMA1_CH0, p.GPDMA1_CH1, cfg);
    let i2c_led_bus = Mutex::new(i2c_led);
    let i2c_led_bus_manager = I2C_BUS_LED.init(i2c_led_bus);
    spawner.spawn(pwm_i2c_task(i2c_led_bus_manager, PWM_ADDRESS, CHANNEL_PWM.receiver())).unwrap();

    // -----------------------------------
    // Configure I2C for display
    // -----------------------------------
    // Display I2C / Smbus - I2C2
    // SCL: PF1
    // SDA: PF0
    // Alert#: PF2
    // Reset: PF3

    // let mut cfg: I2cConfig = I2cConfig::default();
    // cfg.timeout = Duration::from_millis(200);
    // cfg.frequency = Hertz(100_000);
    // let i2c_display = I2c::new(p.I2C2, p.PF1, p.PF0, Irqs, p.GPDMA1_CH2, p.GPDMA1_CH3, cfg);
    // let i2c_display_bus = Mutex::new(i2c_display);
    // let i2c_display_bus_manager = I2C_BUS_DISPLAY.init(i2c_display_bus);
    // spawner
    //     .spawn(ui_task(i2c_display_bus_manager, CHANNEL_UI.receiver()))
    //     .unwrap();

    // -----------------------------------
    // Configure I2C for DMX
    // -----------------------------------
    // DMX I2C / Smbus - I2C1
    // SCL: PB8
    // SDA: PB9
    // Alert#: PB5
    // Reset: PA3
    let mut cfg: I2cConfig = I2cConfig::default();
    cfg.timeout = Duration::from_millis(200);
    cfg.frequency = Hertz(4_000_000);
    let i2c_dmx = I2c::new(p.I2C1, p.PB8, p.PB9, Irqs, p.GPDMA1_CH4, p.GPDMA1_CH5, cfg);

    let i2c_dmx_bus = Mutex::new(i2c_dmx);
    let i2c_dmx_bus_manager = I2C_BUS_DMX.init(i2c_dmx_bus);
    spawner.spawn(dmx_task(i2c_dmx_bus_manager, DMX_ADDRESS, CHANNEL_DMX.sender())).unwrap();

    // -----------------------------------
    // Button (header)
    // -----------------------------------
    let button = Input::new(p.PC13, Pull::Up);
    spawner.spawn(button_task(button, CHANNEL.sender())).unwrap();

    // -----------------------------------
    // Display board buttons
    // -----------------------------------
    // spawner
    //     .spawn(button_array_task(
    //         [
    //             Input::new(p.PE7, Pull::Up),
    //             Input::new(p.PE8, Pull::Up),
    //             Input::new(p.PE9, Pull::Up),
    //             Input::new(p.PE10, Pull::Up),
    //         ],
    //         [
    //             OutputOpenDrain::new(p.PD10, Level::High, Speed::Medium),
    //             OutputOpenDrain::new(p.PD11, Level::High, Speed::Medium),
    //             OutputOpenDrain::new(p.PD12, Level::High, Speed::Medium),
    //             OutputOpenDrain::new(p.PD13, Level::High, Speed::Medium),
    //         ],
    //         CHANNEL.sender(),
    //     ))
    //     .unwrap();
    spawner
        .spawn(button_row_task(
            [Input::new(p.PD10, Pull::Up), Input::new(p.PD11, Pull::Up), Input::new(p.PD12, Pull::Up), Input::new(p.PD13, Pull::Up)],
            CHANNEL.sender(),
        ))
        .unwrap();

    // -----------------------------------
    // USB
    // -----------------------------------

    // Create the driver, from the HAL.
    let driver = {
        let d = Driver::new(p.USB, Irqs, p.PA12, p.PA11);
        d
    };

    spawner.spawn(usb_task(driver, CHANNEL_USB.receiver(), CHANNEL.sender())).unwrap();

    spawner.spawn(log_task(CHANNEL.sender(), CHANNEL_LOG.receiver().unwrap(), CHANNEL_USB.sender())).unwrap();

    // // -----------------------------------
    // // Setup USART for RS485 / DMX
    // // https://ww1.microchip.com/downloads/aemDocuments/documents/OTH/ApplicationNotes/ApplicationNotes/00001659A.pdf
    // // -----------------------------------

    // //A data byte is a Start bit, eight data bits and two Stop bits with LSB sent first
    // let mut usart_config = UsartConfig::default();

    // usart_config.baudrate = 250000;
    // usart_config.data_bits = DataBits::DataBits9; // set to 9 data bits but we will ignore the start bit
    // usart_config.stop_bits = StopBits::STOP2; //StopBits::STOP2;

    // // CN10 pin 14 (D1) = p.PG14, CN10 pin 16 (D0) = p.PG9
    // let usart = Uart::new(p.USART6, p.PG9, p.PG14, Irqs, p.DMA2_CH7, p.DMA2_CH2, usart_config).unwrap();

    // // Connect this pin to RX pin so we can detect DMX BREAK and MAB independent of the USART peripheral
    // // let dmx_break_pin = ExtiInput::new(p.PD7, p.EXTI7, Pull::None);
    // let dmx_break_pin = ExtiInput::new(p.PE8, p.EXTI8, Pull::None);
    // spawner
    //     .spawn(dmx_task(usart, dmx_break_pin, CHANNEL_DMX.sender()))
    //     .unwrap();

    // -----------------------------------
    // Config SPI for WS2812B
    // -----------------------------------

    // LED1 CLK:  A5, DAT: D7 - SPI1
    // LED2 CLK: B10, DAT: C3 - SPI2
    // LED3 CLK: C10, DAT: B2 - SPI3
    // LED4 CLK: E12, DAT: E14 - SPI4

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(3_000_000);
    spi_config.mode = SpiMode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    };

    let spi_1 = Spi::new_txonly(p.SPI1, p.PA5, p.PD7, p.GPDMA2_CH0, spi_config);
    let spi_2 = Spi::new_txonly(p.SPI2, p.PB10, p.PC3, p.GPDMA2_CH1, spi_config);
    let spi_3 = Spi::new_txonly(p.SPI3, p.PC10, p.PB2, p.GPDMA2_CH2, spi_config);
    let spi_4 = Spi::new_txonly(p.SPI4, p.PE12, p.PE14, p.GPDMA2_CH3, spi_config);
    spawner.spawn(smart_led_task(spi_1, spi_2, spi_3, spi_4, CHANNEL_SMART_LED.receiver())).unwrap();

    // -----------------------------------
    // Config SPI for Display
    // -----------------------------------

    // SCK: F7, MISO: F8, MOSI: F9, CS1: F6, CS2: F10, CS3: F11 - SPI5

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(100_000_000);
    spi_config.mode = SpiMode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    };

    let spi_display: Spi<'_, embassy_stm32::mode::Async> = Spi::new(p.SPI5, p.PF7, p.PF9, p.PF8, p.GPDMA2_CH4, p.GPDMA2_CH5, spi_config);
    let display_cs = Output::new(p.PF6, Level::High, Speed::Low);
    let display_dc = Output::new(p.PF11, Level::High, Speed::Low);
    let display_reset = Output::new(p.PF10, Level::High, Speed::Low);
    let display_backlight = OutputOpenDrain::new(p.PF3, Level::Low, Speed::Low); // using display reset from i2c which is pulled high

    spawner
        .spawn(ui_task_spi(spi_display, display_cs, display_dc, display_reset, display_backlight, CHANNEL_UI.receiver()))
        .unwrap();

    // -----------------------------------
    // Config SPI for DMX
    // -----------------------------------

    // SCK: C12, MISO: A6, MOSI: G14, CS1: A0, CS2: C8, CS3: B4 - SPI6

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(3_000_000);
    spi_config.mode = SpiMode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    };

    let _spi_dmx = Spi::new(p.SPI6, p.PC12, p.PG14, p.PA6, p.GPDMA1_CH6, p.GPDMA1_CH7, spi_config);
    // spawner
    //     .spawn(dmx_task(spi_dmx, CHANNEL_UI.receiver()))
    //     .unwrap();

    // -----------------------------------
    // Config ethernet for ArtNet
    // -----------------------------------
    cfg_if! {
        if #[cfg(feature = "ethernet")] {
            // Generate random seed.
            let mut rng = Rng::new(p.RNG, Irqs);
            let mut seed = [0; 8];
            rng.async_fill_bytes(&mut seed).await.unwrap();
            let seed = u64::from_le_bytes(seed);

            let mac_addr = [0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];

            static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();
            let ethernet_device = Ethernet::new(
                PACKETS.init(PacketQueue::<4, 4>::new()),
                p.ETH,
                Irqs,
                p.PA1,
                p.PA2,
                p.PC1,
                p.PA7,
                p.PC4,
                p.PC5,
                p.PG13,
                p.PB15,
                p.PG11,
                GenericSMI::new(0),
                mac_addr,
            );


            let config = embassy_net::Config::dhcpv4(Default::default());
            //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
            //    address: Ipv4Cidr::new(Ipv4Address::new(10, 42, 0, 61), 24),
            //    dns_servers: Vec::new(),
            //    gateway: Some(Ipv4Address::new(10, 42, 0, 1)),
            //});

            // Init network stack
            static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
            let (stack, runner) = embassy_net::new(ethernet_device, config, RESOURCES.init(StackResources::new()), seed);

            spawner
                .spawn(artnet_task(stack, runner, spawner.clone(), CHANNEL_DMX.sender()))
                .unwrap();

        }
    }

    // -----------------------------------
    // Initialize event router
    // -----------------------------------

    info!("Initializing event router.");

    let router = Router::new(
        CHANNEL.receiver(),
        CHANNEL_DMX.receiver(),
        // CHANNEL_LED.sender(),
        // CHANNEL_PWM.sender(),
        CHANNEL_PWM_I2C.sender(),
        CHANNEL_SMART_LED.sender(),
        CHANNEL_UI.sender(),
        CHANNEL_LOG.sender(),
    );

    spawner.spawn(event_router(router)).unwrap();
}

// fn draw_smiley<T: DrawTarget<Color = Rgb565>>(display: &mut T) -> Result<(), T::Error> {
//     // Draw the left eye as a circle located at (50, 100), with a diameter of 40, filled with white
//     Circle::new(Point::new(50, 100), 40)
//         .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
//         .draw(display)?;

//     // Draw the right eye as a circle located at (50, 200), with a diameter of 40, filled with white
//     Circle::new(Point::new(50, 200), 40)
//         .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
//         .draw(display)?;

//     // Draw an upside down red triangle to represent a smiling mouth
//     Triangle::new(
//         Point::new(130, 140),
//         Point::new(130, 200),
//         Point::new(160, 170),
//     )
//     .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
//     .draw(display)?;

//     // Cover the top part of the mouth with a black triangle so it looks closed instead of open
//     Triangle::new(
//         Point::new(130, 150),
//         Point::new(130, 190),
//         Point::new(150, 170),
//     )
//     .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
//     .draw(display)?;

//     Ok(())
// }
