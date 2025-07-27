//! Smart LED Interface
use defmt::Format;
use embassy_time::{with_timeout, Duration};
use embassy_stm32::spi::Spi;
use embassy_stm32::mode::Async;
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
    ws: Ws2812<Spi<'a, Async>, Grb>,
    num_leds: usize,
    data: [RGB8; NUM_LEDS_MAX],
    rx: SmartLedChannelRx,
}

impl<'a> SmartLed<'a> {
    pub fn new(spi: Spi<'a, Async>, num_leds: usize, rx: SmartLedChannelRx) -> Self {
        let ws: Ws2812<_, Grb> = Ws2812::new(spi);
        let data = [RGB8::default(); NUM_LEDS_MAX];
        Self { ws, num_leds, data, rx }
    }

    pub async fn enable(&mut self) {
        for i in 0..self.num_leds {
            self.data[i] = RGB8::default();
        }
        self.ws.write(self.data).await.ok();
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
                for i in self.num_leds..prev_length+1 {
                    self.data[i] = RGB8::default();
                }
                self.ws.write(self.data).await.ok();
            }
        }
    }
}


#[embassy_executor::task]
pub async fn smart_led_task(spi: Spi<'static, Async>, rx: SmartLedChannelRx) {
    let mut smart_led = SmartLed::new(spi, 8, rx);
    smart_led.enable().await;
    loop {
        smart_led.show().await;
    } 
}