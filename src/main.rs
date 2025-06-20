#![no_std]
#![no_main]

#[allow(unused_imports)]
use defmt::{panic, *};
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::time::Hertz;
use embassy_stm32::usb::Driver;
use embassy_stm32::{bind_interrupts, peripherals, usb, Config};
use embassy_time::Timer;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};


mod usb_io;
use usb_io::usb_task;

mod event_router;
use event_router::{event_router, Router};

mod led;
use led::{button_task, led_task};

mod logger;
use logger::log_task;

mod channels;
use channels::*;

mod ansi;

bind_interrupts!(struct Irqs {
    OTG_FS => usb::InterruptHandler<peripherals::USB_OTG_FS>;
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
    // On board LEDs
    // -----------------------------------

    let mut led_red = Output::new(p.PB14, Level::Low, Speed::Low);
    let led_blue = Output::new(p.PB7, Level::Low, Speed::Low);
    // let mut led_green = Output::new(p.PB0, Level::Low, Speed::Low);

    // blink red led forever
    let blink_fut = async {
        loop {
            led_red.toggle();
            Timer::after_millis(500).await;
        }
    };
    spawner
        .spawn(led_task(led_blue, CHANNEL_LED.receiver()))
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
    // Initialize event router
    // -----------------------------------

    info!("Initializing event router.");

    let router = Router::new(
        CHANNEL.receiver(),
        CHANNEL_LED.sender(),
        CHANNEL_LOG.sender(),
    );

    spawner.spawn(event_router(router)).unwrap();

    // -----------------------------------
    // Start blinking red LED
    // -----------------------------------
    blink_fut.await;

}
