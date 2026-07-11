//! DMX & ArtNet Device
//!
//! Firmware to process incoming DMX or ArtNet data and update attached LEDs.
//! Multiple output module types are supported including SmartLED (WS2812),
//! PWM output (low power LEDs), and high power LED drivers. A user interface
//! on the fixture allows for DMX / ArtNet configuration and for output LEDs
//! to be addressed individually or in groups.
#![no_std]
#![no_main]




// add enable_ethernet setting on the menu
// add boot_successful flag bit in the eeprom.
// on boot check boot_successful flag. if not set then clear enable_ethernet flag.  if set then boot normally.
// clear boot_successful flag at the start of each boot sequence.  set boot_successful flag at the end of a successful boot sequence.
// during boot only try to enable ethernet if 'enable ethernet' setting is set in the eeprom.
// after boot sequence set boot_successful flag in eeprom.



use cfg_if::cfg_if;
// use defmt::*;

// use defmt::Format;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{info, error};
    } else {
        use defmt::{info, error};
    }
}

use {defmt_rtt as _, panic_probe as _};

use core::cell::RefCell;

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Input, Level, Output, OutputOpenDrain, Pull, Speed};
use embassy_stm32::i2c::{Config as I2cConfig, I2c};
use embassy_stm32::time::Hertz;
// use embassy_stm32::usb::Driver;
// use embassy_stm32::usart::{Config as UsartConfig, DataBits, StopBits, Uart};
use embassy_stm32::rcc::{mux, AHBPrescaler, APBPrescaler, Hse, HseMode, Hsi48Config, Pll, PllDiv, PllMul, PllPreDiv, PllSource, Sysclk, VoltageScale};
use embassy_stm32::spi::{Config as SpiConfig, Mode as SpiMode, Phase, Polarity, Spi};
use embassy_stm32::{bind_interrupts, i2c, peripherals, usb, Config};
use embassy_sync::blocking_mutex::NoopMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{with_timeout, Duration, Timer};

use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;

use crate::eeprom::EepromEvent;
use crate::event_router::{MainEvent, ReturnChannel, RouterEvent};
use crate::ui::{MenuData, ModuleType};

// Eth
cfg_if! {
    if #[cfg(feature = "ethernet")] {
        use crate::ui::EthernetIPMode;
        use embassy_net::StackResources;
        use embassy_net::{Ipv4Cidr, Ipv4Address};
        use embassy_stm32::eth::{Ethernet, PacketQueue, GenericPhy};
        use embassy_stm32::rng::Rng;
        use embassy_stm32::{eth, rng};
        use static_cell::StaticCell;
    }
}

mod artnet;
cfg_if! {
    if #[cfg(feature = "ethernet")] {
        use crate::artnet::{artnet_task, net_task};
    }
}

mod pwm_i2c;
pub use pwm_i2c::pwm_i2c_task;

mod constants;
pub use constants::*;

mod statics;
pub use statics::*;

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

// USB composite device: Enttec DMX widget, plus a CDC logger interface
// when the `usb` logging feature is enabled.
mod enttec_usb;
use embassy_stm32::usb::Driver;
use enttec_usb::enttec_usb_task;

mod event_router;
use event_router::{event_router, Router};

mod led;
use led::led_task;


mod channels;
use channels::*;

mod ansi;

mod dmx_i2c;
use dmx_i2c::dmx_task;

mod smart_led;
use smart_led::smart_led_task;

mod ui;
use ui::ui_task_spi;
use ui::BootStatus;

mod eeprom;
use eeprom::eeprom_i2c_task;

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

    cfg_if! {
        if #[cfg(feature = "clock_stlink")] {
            // HSE on the Nucleo board defaults to an output from the STLink micro.
            // There is also an on board 25MHz clock that could be used by changing some jumpers.
            // The config here is for the STLink source at 8MHz.
            config.rcc.hse = Some(Hse {
                freq: Hertz(8_000_000),
                mode: HseMode::BypassDigital,
            });

            config.rcc.hsi = None;

            // System
            // PLL1Q -> Ethernet
            config.rcc.pll1 = Some(Pll {
                source: PllSource::HSE, // use HSE as the clock source
                prediv: PllPreDiv::DIV2,
                mul: PllMul::MUL125,
                divp: Some(PllDiv::DIV2), // PLL P divisor => pll_src / prediv * mul / divp = 8mhz / 2 * 125 / 2 = 250Mhz
                divq: Some(PllDiv::DIV2), // PLL Q divisor => pll_src / prediv * mul / divp = 8mhz / 2 * 125 / 2 = 250Mhz
                divr: None,
            });

        } else if #[cfg(feature = "clock_25MHz_osc")] {
            // The Nucleo board also has an on board 25MHz clock that could be used by changing some jumpers.
            config.rcc.hse = Some(Hse {
                freq: Hertz(25_000_000),
                mode: HseMode::Oscillator,
            });

            config.rcc.hsi = None;


            // System
            // PLL1Q -> Ethernet
            config.rcc.pll1 = Some(Pll {
                source: PllSource::HSE, // use HSE as the clock source
                prediv: PllPreDiv::DIV2,
                mul: PllMul::MUL40,
                divp: Some(PllDiv::DIV2), // PLL P divisor => pll_src / prediv * mul / divp = 25mhz / 2 * 40 / 2 = 250Mhz
                divq: Some(PllDiv::DIV2), // PLL Q divisor => pll_src / prediv * mul / divp = 25mhz / 2 * 40 / 2 = 250Mhz
                divr: None,
            });

        } else {
            // This option defaults to using the high speed internal clock as the main PLL source.
            // This clock is less accurate than using an external clock.
            // The internal clock is 64MHz.  Divide that clock by 8 to get an input of 8MHz to the
            // rest of the clock chain.
            // config.rcc.hsi = Some(HSIPrescaler::DIV8);
            config.rcc.hsi = None;

            // System
            // PLL1Q -> Ethernet
            config.rcc.pll1 = Some(Pll {
                // source: PllSource::HSI, // use HSI as the clock source
                // prediv: PllPreDiv::DIV2,
                // mul: PllMul::MUL125,
                // divp: Some(PllDiv::DIV2), // PLL P divisor => pll_src / prediv * mul / divp = 8mhz / 2 * 125 / 2 = 250Mhz
                // divq: Some(PllDiv::DIV2), // PLL Q divisor => pll_src / prediv * mul / divp = 8mhz / 2 * 215 / 2 = 250Mhz
                // divr: None,
                source: PllSource::CSI,
                prediv: PllPreDiv::DIV1,
                mul: PllMul::MUL125,
                divp: Some(PllDiv::DIV2), // PLL P divisor => pll_src / prediv * mul / divp = 8mhz / 2 * 125 / 2 = 250Mhz
                divq: Some(PllDiv::DIV4), // PLL Q divisor => pll_src / prediv * mul / divp = 8mhz / 2 * 215 / 2 = 250Mhz
                divr: None,
            });
        }
    }

    // config.rcc.hsi48 = Some(Default::default());
    // config.rcc.hsi48 = Some(Hsi48Config { sync_from_usb: true }); // needed for USB


    config.rcc.hsi48 = Some(Hsi48Config { sync_from_usb: true }); // needed for USB (logger or Enttec CDC)


    config.rcc.csi = true; // enable CSI clock

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

    config.rcc.ahb_pre = AHBPrescaler::DIV1; // AHBPrescaler::DIV2;
    config.rcc.apb1_pre = APBPrescaler::DIV1; // APBPrescaler::DIV4;
    config.rcc.apb2_pre = APBPrescaler::DIV1; // APBPrescaler::DIV2;
    config.rcc.apb3_pre = APBPrescaler::DIV1; // APBPrescaler::DIV4;
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.voltage_scale = VoltageScale::Scale0;

    config.rcc.mux.spi1sel = mux::Spi1sel::PLL2_P;
    config.rcc.mux.spi2sel = mux::Spi2sel::PLL2_P;
    config.rcc.mux.spi3sel = mux::Spi3sel::PLL2_P;
    config.rcc.mux.spi4sel = mux::Spi4sel::PLL2_Q;
    config.rcc.mux.spi5sel = mux::Spi5sel::PLL2_Q;
    config.rcc.mux.spi6sel = mux::Spi6sel::PLL2_Q;
    config.rcc.mux.i2c1sel = mux::I2csel::PLL3_R;
    config.rcc.mux.i2c2sel = mux::I2csel::PLL3_R;
    config.rcc.mux.i2c4sel = mux::I2c34sel::PLL3_R;
    config.rcc.mux.usbsel = mux::Usbsel::HSI48;
    // config.rcc.mux.persel = mux::Persel::HSI;
    config.rcc.mux.rngsel = mux::Rngsel::HSI48;

    let p = embassy_stm32::init(config);

    // -----------------------------------
    // On board LEDs
    // -----------------------------------

    let led0 = Output::new(p.PB0, Level::High, Speed::Low);
    // // let mut led1 = Output::new(p.PF4, Level::High, Speed::Low);
    // // let mut led2 = Output::new(p.PG4, Level::High, Speed::Low);
    spawner.spawn(led_task(led0)).unwrap();

    // -----------------------------------
    // Configure I2C for module led board devices
    // -----------------------------------
    // LED (output) I2C / Smbus - I2C4
    // SCL: PF5
    // SDA: PF15
    // Alert#: PF13
    // Reset: PF14
    let mut cfg: I2cConfig = I2cConfig::default();
    cfg.frequency = Hertz(100_000);
    cfg.timeout = Duration::from_millis(25); // need to keep the timeout low to ensure there is not a conflict between different uses on the shared bus.

    let i2c_led = I2c::new(p.I2C4, p.PF5, p.PF15, Irqs, p.GPDMA1_CH0, p.GPDMA1_CH1, cfg);

    let i2c_led_bus = NoopMutex::new(RefCell::new(i2c_led));

    let i2c_led_bus_manager = I2C_BUS_LED.init(i2c_led_bus);
    let i2c_led_dev_1 = I2cDevice::new(i2c_led_bus_manager);
    spawner.spawn(eeprom_i2c_task(i2c_led_dev_1, EEPROM_ADDRESS, CHANNEL_EEPROM.receiver(), CHANNEL.sender())).unwrap();

    // info!("Try store module type");
    // let a = CHANNEL_EEPROM.try_send(EepromEvent::WriteModuleType(ModuleType::SmartLed));

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
    spawner
        .spawn(dmx_task(i2c_dmx_bus_manager, DMX_ADDRESS, CHANNEL_DMX.sender(), CHANNEL_DMX_FEEDBACK.receiver().unwrap()))
        .unwrap();

    // -----------------------------------
    // Button (header)
    // -----------------------------------
    let button = Input::new(p.PC13, Pull::Up);
    Timer::after_millis(10).await;
    let factory_reset = button.is_low();
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

    // Composite USB device: Enttec DMX USB Pro widget emulation (USB>DMX mode
    // + DMX-to-PC forwarding), plus a CDC logger interface with the `usb` feature.
    let driver = Driver::new(p.USB, Irqs, p.PA12, p.PA11);
    spawner
        .spawn(enttec_usb_task(driver, CHANNEL_DMX.sender(), CHANNEL_DMX_FEEDBACK.receiver().unwrap()))
        .unwrap();

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
        .spawn(ui_task_spi(
            spi_display,
            display_cs,
            display_dc,
            display_reset,
            display_backlight,
            CHANNEL_UI.receiver(),
            CHANNEL.sender(),
        ))
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
    // Initialize event router
    // -----------------------------------

    info!("Initializing event router.");

    let router = Router::new(
        CHANNEL.receiver(),
        CHANNEL_DMX.receiver(),
        CHANNEL_DMX_FEEDBACK.sender(),
        // CHANNEL_PWM.sender(),
        CHANNEL_PWM_I2C.sender(),
        CHANNEL_SMART_LED.sender(),
        CHANNEL_UI.sender(),
        CHANNEL_EEPROM.sender(),
        CHANNEL_MAIN.sender(),
    );

    spawner.spawn(event_router(router)).unwrap();

    if factory_reset {
        info!("");
        info!("");
        info!("Resetting to factory defaults");
        info!("");
        info!("");
        let _ = CHANNEL_EEPROM.send(EepromEvent::WriteModuleType(ModuleType::SmartLed)).await;
        Timer::after_millis(500).await;
        let _ = CHANNEL_EEPROM.send(EepromEvent::WriteMacAddress([0, 0, 0, 0, 0, 0])).await;
        Timer::after_millis(500).await;
        let _ = CHANNEL_EEPROM.send(EepromEvent::WriteSettings(MenuData::default())).await;
        Timer::after_millis(500).await;
    }

    info!("Try to read previous boot status");
    let _ = CHANNEL_EEPROM.send(EepromEvent::ReadBootStatus).await;
    Timer::after_millis(50).await;

    info!("Try to read module type settings");
    let _ = CHANNEL_EEPROM.send(EepromEvent::ReadModuleType).await;
    Timer::after_millis(50).await;

    info!("Try to read mac address");
    let _ = CHANNEL_EEPROM.send(EepromEvent::ReadMacAddress).await;
    Timer::after_millis(50).await;

    info!("Try to read settings");
    let _ = CHANNEL_EEPROM.send(EepromEvent::ReadSettings).await;
    Timer::after_millis(50).await;

    info!("Try to get boot status");
    let _ = CHANNEL.try_send(RouterEvent::GetBootStatus(ReturnChannel::Main));
    let boot_status = if let Ok(new_message) = with_timeout(Duration::from_millis(250), CHANNEL_MAIN.receiver().receive()).await {
        match new_message {
            MainEvent::ReturnBootStatus(x) => {
                if let Some(m) = x {
                    m
                } else {
                    BootStatus::Failed
                }
            }
            _ => BootStatus::Failed,
        }
    } else {
        error!("Unable to get boot status.");
        BootStatus::Failed
    };
    info!("Boot Status {:?}", boot_status);

    info!("Try to reset boot status on eeprom to failed.");
    let _ = CHANNEL_EEPROM.send(EepromEvent::WriteBootStatus(ui::BootStatus::Failed)).await;
    Timer::after_millis(50).await;


    info!("Try to get module type");
    let _ = CHANNEL.try_send(RouterEvent::GetModuleType(ReturnChannel::Main));
    let module_type = if let Ok(new_message) = with_timeout(Duration::from_millis(250), CHANNEL_MAIN.receiver().receive()).await {
        match new_message {
            MainEvent::ReturnModuleType(x) => {
                if let Some(m) = x {
                    m
                } else {
                    ModuleType::Unknown
                }
            }
            _ => ModuleType::Unknown,
        }
    } else {
        error!("Unable to get module type.");
        ModuleType::Unknown
    };
    info!("Module Types {:?}", module_type);

    info!("Try to get settings");
    let _ = CHANNEL.try_send(RouterEvent::GetSettings(ReturnChannel::Main));
    let mut settings_from_eeprom = if let Ok(new_message) = with_timeout(Duration::from_millis(250), CHANNEL_MAIN.receiver().receive()).await {
        match new_message {
            MainEvent::ReturnSettings(x) => x,
            _ => MenuData::default(),
        }
    } else {
        error!("Unable to get settings.");
        MenuData::default()
    };
    info!("Settings from EEPROM {:?}", settings_from_eeprom);


    if boot_status == BootStatus::Failed {
        info!("Previous boot was not successful.  Disable ethernet to prevent potential lockout.");

        // Update the local copy too so ethernet stays disabled for *this* boot,
        // not just the next one.
        settings_from_eeprom.ethernet_enabled = false;
        let _ = CHANNEL_EEPROM.send(EepromEvent::WriteSettings(settings_from_eeprom)).await;
        Timer::after_millis(50).await;
    }

    
    cfg_if! {
        if #[cfg(feature = "ethernet")] {
            if settings_from_eeprom.ethernet_enabled {
                info!("Try to get mac address");
                let _ = CHANNEL.try_send(RouterEvent::GetMacAddress(ReturnChannel::Main));
                let (mac_address, read_mac_success) = if let Ok(new_message) = with_timeout(Duration::from_millis(250), CHANNEL_MAIN.receiver().receive()).await {
                    match new_message {
                        MainEvent::ReturnMacAddress(x) => {
                            if let Some(m) = x {
                                (m, true)
                            } else {
                                ([0, 0, 0, 0, 0, 0], false)
                            }
                        },
                        _ => ([0, 0, 0, 0, 0, 0], false),
                    }
                } else {
                    error!("Unable to get mac address.");
                    ([0, 0, 0, 0, 0, 0], false)
                };

                info!("MAC Address currently: {:?}", mac_address);

                let mut mac_addr = mac_address;
                let mut rng = Rng::new(p.RNG, Irqs);

                if mac_address == [0, 0, 0, 0, 0, 0] {
                    // generate random mac address
                    rng.fill_bytes(&mut mac_addr);

                    // force the least significant bit of addr0 to be 0 so the mac is unicast.
                    mac_addr[0] = (mac_addr[0] >> 1) << 1;
                    info!("New calculated MAC Address: {:?}", mac_addr);

                    // store new mac address but only if we successfully read all zeros.  otherwise we assume i2c error and don't overwrite the mac
                    if read_mac_success {
                        let _ = CHANNEL_EEPROM.send(EepromEvent::WriteMacAddress(mac_addr)).await;
                        Timer::after_millis(100).await;
                    }

                }

                // Art-Net primary IP rule: 2.(MAC[3]+OemHi+OemLo mod 256).MAC[4].MAC[5]
                // wrapping_add gives the mod-256 sum without overflow panics
                let oem: [u8; 2] = ARTNET_OEM.to_be_bytes();
                let static_ip = [2, mac_addr[3].wrapping_add(oem[0]).wrapping_add(oem[1]), mac_addr[4], mac_addr[5]];

                info!("Calcuated IP: {:?}", static_ip);

                // Generate random seed.
                // let mut rng = Rng::new(p.RNG, Irqs);
                let mut seed = [0; 8];

                rng.fill_bytes(&mut seed);

                let seed = u64::from_le_bytes(seed);

                static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();
                let l = PACKETS.init(PacketQueue::<4, 4>::new());

                let ethernet_device = Ethernet::new(
                    l,
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
                    GenericPhy::new_auto(),
                    mac_addr,
                );

                // Choose between dhcp or static ip
                let config = match settings_from_eeprom.ethernet_ip_mode {
                    EthernetIPMode::Dhcp => embassy_net::Config::dhcpv4(Default::default()),
                    EthernetIPMode::Static => embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
                        address: Ipv4Cidr::new(Ipv4Address::new(static_ip[0],static_ip[1],static_ip[2],static_ip[3]), 24),
                        dns_servers: Default::default(),
                        gateway: Some(Ipv4Address::new(192,168,86,1)),
                    })
                };

                // Init network stack
                static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
                let (stack, runner) = embassy_net::new(ethernet_device, config, RESOURCES.init(StackResources::new()), seed);

                // Launch network task
                spawner.spawn(net_task(runner)).unwrap_or_else(|_| error!("Unable to spawn net task."));

                spawner
                    .spawn(artnet_task(stack, CHANNEL_DMX.sender(), CHANNEL.sender(), CHANNEL_DMX_FEEDBACK.receiver().unwrap()))
                    .unwrap_or_else(|_| error!("Unable to spawn artnet task."));
            }
        }
    }

    let _ = CHANNEL_EEPROM.send(EepromEvent::WriteBootStatus(ui::BootStatus::Success)).await;
    Timer::after_millis(50).await;
    // let _ = CHANNEL.try_send(RouterEvent::StoreBootComplete(true));
}
