#![no_std]
#![no_main]

#[allow(unused_imports)]
use defmt::{panic, *};
// use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, OutputType, Pull, Speed};
use embassy_stm32::time::{hz, Hertz};
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::i2c::I2c;
use embassy_stm32::usb::Driver;
use embassy_stm32::usart::{Config as UsartConfig, DataBits, StopBits, Uart};
use embassy_stm32::spi::{Config as SpiConfig, Mode as SpiMode, Spi, Phase, Polarity};
use embassy_stm32::{bind_interrupts, i2c, peripherals, usb, usart, Config};
use embassy_time::Timer;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;


// https://github.com/cschuhen/oled_drivers/blob/master/examples/i2c.rs


use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};

use embedded_hal_async::i2c::AddressMode;
use oled_async::{prelude::*, Builder};

use static_cell::StaticCell;
use crate::artnet::artnet_task;

use {defmt_rtt as _, panic_probe as _};

// Eth
use embassy_net::StackResources;
use embassy_stm32::eth::generic_smi::GenericSMI;
use embassy_stm32::eth::{Ethernet, PacketQueue};
use embassy_stm32::rng::Rng;
use embassy_stm32::{eth, rng};


// -----

// PWM Chip
// PCA9685

// https://www.st.com/en/microcontrollers-microprocessors/stm32f746zg.html
// https://www.st.com/en/evaluation-tools/nucleo-f746zg.html

// https://www.st.com/en/evaluation-tools/nucleo-h563zi.html
// https://www.st.com/en/microcontrollers-microprocessors/stm32h563zi.html

// https://www.st.com/en/evaluation-tools/nucleo-h533re.html
// https://www.st.com/en/microcontrollers-microprocessors/stm32h533re.html


// -----------------------
// Global allocator needed for nom which is used by tiny_artnet
// -----------------------
use alloc_cortex_m::CortexMHeap;
#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();
// -----------------------

mod usb_io;
use usb_io::usb_task;

mod event_router;
use event_router::{event_router, Router};

mod led;
use led::{button_task, led_task};

mod pwm;
use pwm::pwm_task;

mod logger;
use logger::log_task;

mod channels;
use channels::*;

mod ansi;

mod dmx;
use dmx::dmx_task;

mod smart_led;
use smart_led::smart_led_task;

mod artnet;

type I2c1Bus = Mutex<NoopRawMutex, I2c<'static, embassy_stm32::mode::Async>>;

/// Shared I2C / Smbus
static I2C_BUS: StaticCell<I2c1Bus> = StaticCell::new();

bind_interrupts!(struct Irqs {
    OTG_FS => usb::InterruptHandler<peripherals::USB_OTG_FS>;
    USART2 => usart::InterruptHandler<peripherals::USART2>;
    I2C2_EV => i2c::EventInterruptHandler<peripherals::I2C2>;
    I2C2_ER => i2c::ErrorInterruptHandler<peripherals::I2C2>;
    I2C4_EV => i2c::EventInterruptHandler<peripherals::I2C4>;
    I2C4_ER => i2c::ErrorInterruptHandler<peripherals::I2C4>;
    ETH => eth::InterruptHandler;
    RNG => rng::InterruptHandler<peripherals::RNG>;
});







#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: Hertz(8_000_000),
            mode: HseMode::Bypass,
        });
        config.rcc.pll_src = PllSource::HSE;
        config.rcc.pll = Some(Pll {
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL216,
            divp: Some(PllPDiv::DIV2), // 8mhz / 4 * 216 / 2 = 216Mhz
            divq: Some(PllQDiv::DIV9), // 8mhz / 4 * 216 / 9 = 48Mhz
            divr: None,
        });
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV4;
        config.rcc.apb2_pre = APBPrescaler::DIV2;
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.mux.clk48sel = mux::Clk48sel::PLL1_Q;
    }
    let p = embassy_stm32::init(config);

    // -----------------------------------
    // Configure I2C for misc devices
    // -----------------------------------
    // CN7 Pin 2 / D15 - PB8 - I2C_A_SCL (I2C1)
    // CN7 Pin 4 / D14 - PB9 - I2C_A_SDA (I2C1)
    // let i2c = I2c::new(
    //     p.I2C4,
    //     p.PF14,
    //     p.PF15,
    //     Irqs,
    //     p.DMA1_CH5,
    //     p.DMA1_CH2,
    //     Hertz(100_000),
    //     Default::default(),
    // );
    // // share i2c bus
    // let i2c_bus = Mutex::new(i2c);
    // let i2c_bus_manager = I2C_BUS.init(i2c_bus);
    // let i2c_bus_dev = I2cDevice::new(i2c_bus_manager);


    // -----------------------------------
    // Configure I2C for display
    // -----------------------------------
    // CN7 Pin 2 / D15 - PB8 - I2C_A_SCL (I2C1)
    // CN7 Pin 4 / D14 - PB9 - I2C_A_SDA (I2C1)
    let i2c = I2c::new(
        p.I2C2,
        p.PF1,
        p.PF0,
        Irqs,
        p.DMA1_CH4,
        p.DMA1_CH3,
        Hertz(100_000),
        Default::default(),
    );

    type I2cDisplay = embassy_stm32::i2c::I2c<
        'static,
        embassy_stm32::mode::Async,
    >;

    type I2cInterface = display_interface_i2c::I2CInterface<I2cDisplay>;
    let di: I2cInterface = display_interface_i2c::I2CInterface::new(
        i2c,  // I2C
        0x3C, // I2C Address
        0x40, // Data byte
    );

    let raw_disp = Builder::new(oled_async::displays::sh1107::Sh1107_64_128 {})
        .with_rotation(crate::DisplayRotation::Rotate180)
        .connect(di);

    let mut disp: GraphicsMode<_, _> = raw_disp.into();

    disp.init().await.unwrap();
    disp.clear();
    disp.flush().await.unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
        .draw(&mut disp)
        .unwrap();
    
    // -----------------------------------
    // On board LEDs
    // -----------------------------------

    // let mut led_red = Output::new(p.PB14, Level::Low, Speed::Low);
    // let mut led_green = Output::new(p.PB0, Level::Low, Speed::Low);
    // let led_blue = Output::new(p.PB7, Level::Low, Speed::Low);
    // spawner
    //     .spawn(led_task(led_blue, CHANNEL_LED.receiver()))
    //     .unwrap();
    
    // -----------------------------------
    // PWMs
    // -----------------------------------

    // PB14 is on TIM12 CH1 (red)
    let pwm_pin1 = PwmPin::new_ch1(p.PB14, OutputType::PushPull);
    let pwm1 = SimplePwm::new(p.TIM12, Some(pwm_pin1), None, None, None, hz(200), CountingMode::EdgeAlignedUp );
    let cs1 = pwm1.split();

    // PB0 is on TIM3 CH3 (green)
    let pwm_pin2 = PwmPin::new_ch3(p.PB0, OutputType::PushPull);
    let pwm2 = SimplePwm::new(p.TIM3, None, None, Some(pwm_pin2), None, hz(200), CountingMode::EdgeAlignedUp  );
    let cs2 = pwm2.split();

    // PB7 is on TIM3 CH3 (green)
    let pwm_pin3 = PwmPin::new_ch2(p.PB7, OutputType::PushPull);
    let pwm3 = SimplePwm::new(p.TIM4, None, Some(pwm_pin3), None, None, hz(200), CountingMode::EdgeAlignedUp  );
    let cs3 = pwm3.split();

    spawner
        .spawn(pwm_task(cs1, cs2, cs3, CHANNEL_PWM.receiver()))
        .unwrap();

    // -----------------------------------
    // On board button
    // -----------------------------------
    // button is used to trigger NIC on / off
    let button = ExtiInput::new(p.PC13, p.EXTI13, Pull::None);
    spawner
        .spawn(button_task(button, CHANNEL.sender()))
        .unwrap();

    // -----------------------------------
    // USB
    // -----------------------------------

    // Setup needed for nucleo-stm32f303ze
    let mut dp_pullup = Output::new(p.PG6, Level::Low, Speed::Medium);
    Timer::after_millis(10).await;
    dp_pullup.set_high();


    // Create the driver, from the HAL.
    // let mut ep_out_buffer = [0u8; 256];
    let mut config = embassy_stm32::usb::Config::default();

    // Do not enable vbus_detection. This is a safe default that works in all boards.
    // However, if your USB device is self-powered (can stay powered on if USB is unplugged), you need
    // to enable vbus_detection to comply with the USB spec. If you enable it, the board
    // has to support it or USB won't work at all. See docs on `vbus_detection` for details.
    config.vbus_detection = false;

    
    // Create the driver, from the HAL.
    let driver = {
        static EP_OUT: StaticCell<[u8; 256]> = StaticCell::new();
        let d = Driver::new_fs(p.USB_OTG_FS, Irqs, p.PA12, p.PA11, EP_OUT.init([0; 256]), config);
        d
    };


    spawner
        .spawn(usb_task(driver, CHANNEL_USB.receiver(), CHANNEL.sender()))
        .unwrap();


    spawner
        .spawn(log_task(
            CHANNEL.sender(),
            CHANNEL_LOG.receiver().unwrap(),
            CHANNEL_USB.sender(),
        ))
        .unwrap();


    // -----------------------------------
    // Setup USART for RS485 / DMX
    // https://ww1.microchip.com/downloads/aemDocuments/documents/OTH/ApplicationNotes/ApplicationNotes/00001659A.pdf
    // -----------------------------------
    // USART_2
    // ConnectorPin    PinName SignalName      STM32Pin
    // 2               D51     USART_B_SCLK    PD7
    // 4               D52     USART_B_RX      PD6
    // 6               D53     USART_B_TX      PD5
    // 8               D54     USART_B_RTS     PD4
    // 10              D55     USART_B_CTS     PD3

    //A data byte is a Start bit, eight data bits and two Stop bits with LSB sent first
    let mut usart_config = UsartConfig::default();
    
    usart_config.baudrate = 250000;
    usart_config.data_bits = DataBits::DataBits9; // set to 9 data bits but we will ignore the start bit
    usart_config.stop_bits = StopBits::STOP2;

    let usart = Uart::new(p.USART2, p.PD6, p.PD5, Irqs, p.DMA1_CH6, p.DMA1_CH5, usart_config).unwrap();
    
    // Connect this pin to RX pin so we can detect DMX BREAK and MAB independent of the USART peripheral
    let dmx_break_pin = ExtiInput::new(p.PD7, p.EXTI7, Pull::None);
    spawner
        .spawn(dmx_task(usart, dmx_break_pin, CHANNEL.sender()))
        .unwrap();

    // let write_buf : [u8; 512] = [0xAA; 512];
    // unwrap!(usart.write(&write_buf).await);

    // -----------------------------------
    // Config SPI for WS2812B
    // -----------------------------------

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(3_000_000);
    spi_config.mode = SpiMode { polarity: Polarity::IdleLow, phase: Phase::CaptureOnFirstTransition };

    let spi = Spi::new_txonly(p.SPI3, p.PB3, p.PB5, p.DMA1_CH7, spi_config);
    spawner
        .spawn(smart_led_task(spi, CHANNEL_SMART_LED.receiver()))
        .unwrap();

    
    // -----------------------------------
    // Config ethernet for ArtNet
    // -----------------------------------

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
        p.PB13,
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
        .spawn(artnet_task(stack, runner, spawner.clone(), CHANNEL_ARTNET.receiver()))
        .unwrap();


    // -----------------------------------
    // Initialize event router
    // -----------------------------------

    info!("Initializing event router.");

    let router = Router::new(
        CHANNEL.receiver(),
        CHANNEL_LED.sender(),
        CHANNEL_PWM.sender(),
        CHANNEL_SMART_LED.sender(),
        CHANNEL_LOG.sender(),
    );

    spawner.spawn(event_router(router)).unwrap();

}