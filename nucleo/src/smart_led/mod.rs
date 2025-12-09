//! SmartLED (WS2812) interface
use defmt::Format;
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_time::{with_timeout, Duration};
use smart_leds::SmartLedsWriteAsync;

use crate::channels::SmartLedChannelRx;

mod ws2812_async;
use crate::LED_COLORS;

use ws2812_async::{Grb, Ws2812};

#[allow(unused)]
pub enum SmartLedEvent {
    UpdateLEDs,
}

impl Format for SmartLedEvent {
    fn format(&self, f: defmt::Formatter) {
        match self {
            SmartLedEvent::UpdateLEDs => defmt::write!(f, "Update LEDs: ()..."),
        }
    }
}

/// SmartLED module
///
/// Each module has 4 ports and a return communication channel
pub struct SmartLed<'a> {
    ws_1: Ws2812<Spi<'a, Async>, Grb>,
    ws_2: Ws2812<Spi<'a, Async>, Grb>,
    ws_3: Ws2812<Spi<'a, Async>, Grb>,
    ws_4: Ws2812<Spi<'a, Async>, Grb>,

    rx: SmartLedChannelRx,
}

impl<'a> SmartLed<'a> {
    pub fn new(spi_1: Spi<'a, Async>, spi_2: Spi<'a, Async>, spi_3: Spi<'a, Async>, spi_4: Spi<'a, Async>, rx: SmartLedChannelRx) -> Self {
        let ws_1: Ws2812<_, Grb> = Ws2812::new(spi_1);
        let ws_2: Ws2812<_, Grb> = Ws2812::new(spi_2);
        let ws_3: Ws2812<_, Grb> = Ws2812::new(spi_3);
        let ws_4: Ws2812<_, Grb> = Ws2812::new(spi_4);

        Self { ws_1, ws_2, ws_3, ws_4, rx }
    }

    pub async fn enable(&mut self) {
        self.send_to_leds().await.ok();
    }

    pub async fn show(&mut self) {
        if let Ok(new_message) = with_timeout(Duration::from_millis(100), self.rx.receive()).await {
            // info!("led message {:?}", new_message);
            self.process_event(new_message).await;
        }
    }

    async fn process_event(&mut self, event: SmartLedEvent) {
        match event {
            SmartLedEvent::UpdateLEDs => {
                self.send_to_leds().await.ok();
            }
        }
    }

    async fn send_to_leds(&mut self) -> Result<(), &'static str> {
        // Get lock on LED colors data
        let colors = LED_COLORS.lock().await;

        // Write color data to the four ports
        let results = embassy_futures::join::join_array([self.ws_1.write(colors[0]), self.ws_2.write(colors[1]), self.ws_3.write(colors[2]), self.ws_4.write(colors[3])]).await;

        let a = results.iter().filter(|&&r| r.is_err()).count();

        if a > 0 {
            Err("SPI write error to LEDs.")
        } else {
            Ok(())
        }
    }
}

/// Task to manage SmartLED module (WS2812 and the like)
#[embassy_executor::task]
pub async fn smart_led_task(spi_1: Spi<'static, Async>, spi_2: Spi<'static, Async>, spi_3: Spi<'static, Async>, spi_4: Spi<'static, Async>, rx: SmartLedChannelRx) {
    let mut smart_led = SmartLed::new(spi_1, spi_2, spi_3, spi_4, rx);
    smart_led.enable().await;
    loop {
        smart_led.show().await;
    }
}
