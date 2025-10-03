//! Smart LED Interface
use defmt::Format;
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_time::{with_timeout, Duration};
use smart_leds::{SmartLedsWriteAsync, RGB8};

use crate::channels::SmartLedChannelRx;

mod ws2812_async;
use ws2812_async::{Grb, Ws2812, NUM_LEDS_MAX};

#[derive(Format)]
pub enum SmartLedEvent {
    // On,
    // Off,
    Value([u8; 4]),
}

pub struct SmartLed<'a> {
    ws_1: Ws2812<Spi<'a, Async>, Grb>,
    ws_2: Ws2812<Spi<'a, Async>, Grb>,
    ws_3: Ws2812<Spi<'a, Async>, Grb>,
    ws_4: Ws2812<Spi<'a, Async>, Grb>,
    num_leds: usize,
    data: [RGB8; NUM_LEDS_MAX],
    rx: SmartLedChannelRx,
}

impl<'a> SmartLed<'a> {
    pub fn new(
        spi_1: Spi<'a, Async>,
        spi_2: Spi<'a, Async>,
        spi_3: Spi<'a, Async>,
        spi_4: Spi<'a, Async>,
        num_leds: usize,
        rx: SmartLedChannelRx,
    ) -> Self {
        let ws_1: Ws2812<_, Grb> = Ws2812::new(spi_1);
        let ws_2: Ws2812<_, Grb> = Ws2812::new(spi_2);
        let ws_3: Ws2812<_, Grb> = Ws2812::new(spi_3);
        let ws_4: Ws2812<_, Grb> = Ws2812::new(spi_4);
        // let ws = [ws_1, ws_2, ws_3, ws_4];
        let data = [RGB8::default(); NUM_LEDS_MAX];
        Self {
            ws_1,
            ws_2,
            ws_3,
            ws_4,
            num_leds,
            data,
            rx,
        }
    }

    pub async fn enable(&mut self) {
        for i in 0..self.num_leds {
            self.data[i] = RGB8::default();
        }
        // for ws in self.ws {
        //     ws.write(self.data).await.ok();
        // }
        self.send_to_leds().await.ok();
    }

    // pub async fn disable(&mut self) {
    //     for i in 0..self.num_leds {
    //         self.data[i] = RGB8::default();
    //     }
    //     self.ws.write(self.data).await.ok();
    // }

    pub async fn show(&mut self) {
        if let Ok(new_message) = with_timeout(Duration::from_millis(100), self.rx.receive()).await {
            // info!("led message {:?}", new_message);
            self.process_event(new_message).await;
        }
    }

    async fn process_event(&mut self, event: SmartLedEvent) {
        match event {
            // SmartLedEvent::On => {
            //     self.enable().await;
            // }
            // SmartLedEvent::Off => {
            //     self.disable().await;

            // }
            SmartLedEvent::Value(values) => {
                let prev_length = self.num_leds;
                self.num_leds = ((values[3] as usize) * 100) / 255 * 3 as usize;

                for i in 0..self.num_leds {
                    self.data[i] = RGB8::new(values[0], values[1], values[2]);
                }
                for i in self.num_leds..prev_length + 1 {
                    self.data[i] = RGB8::default();
                }

                self.send_to_leds().await.ok();
            }
        }
    }

    async fn send_to_leds(&mut self) -> Result<(), &'static str> {
        let results = embassy_futures::join::join_array([
            self.ws_1.write(self.data),
            self.ws_2.write(self.data),
            self.ws_3.write(self.data),
            self.ws_4.write(self.data),
        ])
        .await;

        let a = results.iter().filter(|&&r| r.is_err()).count();

        if a > 0 {
            Err("SPI write error to LEDs.")
        } else {
            Ok(())
        }
    }
}

#[embassy_executor::task]
pub async fn smart_led_task(
    spi_1: Spi<'static, Async>,
    spi_2: Spi<'static, Async>,
    spi_3: Spi<'static, Async>,
    spi_4: Spi<'static, Async>,
    rx: SmartLedChannelRx,
) {
    let mut smart_led = SmartLed::new(spi_1, spi_2, spi_3, spi_4, 5, rx);
    smart_led.enable().await;
    loop {
        smart_led.show().await;
    }
}
