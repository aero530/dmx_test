//! LED & Button interaction
use defmt::{info, Format};
use embassy_stm32::peripherals::{TIM12, TIM3, TIM4};
use embassy_stm32::timer::simple_pwm::SimplePwmChannels;
use embassy_time::{with_timeout, Duration, Timer};
use embassy_stm32::spi::{Config as SpiConfig, Mode as SpiMode, Spi, Phase, Polarity};
use embassy_stm32::mode::Async;

use smart_leds::{brightness, SmartLedsWriteAsync, RGB8};

use crate::channels::{SmartLedChannelRx, RouterChannelTx};
use crate::event_router::RouterEvent;




use crate::ws2812_async::{Grb, Ws2812, NUM_LEDS_MAX, BYTES_PER_LED};




#[derive(Format)]
pub enum SmartLedEvent {
    On,
    Off,
    Value([u8;3]),
}

pub struct SmartLed<'a> {
    // spi: Spi<'a, Async>,
    ws: Ws2812<Spi<'a, Async>, Grb>,
    num_leds: usize,
    data: [RGB8; NUM_LEDS_MAX],
    rx: SmartLedChannelRx,
}

impl<'a> SmartLed<'a> {
    pub fn new(spi: Spi<'a, Async>, num_leds: usize, rx: SmartLedChannelRx) -> Self {
        let ws: Ws2812<_, Grb> = Ws2812::new(spi);
        // let mut data = [RGB8::default(); NUM_LEDS_MAX];
        // for i in 0..NUM_LEDS_MAX {
        //     data[i] = wheel((((i * 256) as u16 / NUM_LEDS_MAX as u16 + 5 as u16) & 255) as u8);
        //     ws.write(brightness(data.iter().cloned(), 32)).await.ok();
        //     Timer::after(Duration::from_millis(5)).await;
        // }
        let data = [RGB8::default(); NUM_LEDS_MAX];
        Self { ws, num_leds, data, rx }
    }

    pub async fn enable(&mut self) {
        for i in 0..self.num_leds {
            self.data[i] = wheel((((i * 256) as u16 / NUM_LEDS_MAX as u16 + 5 as u16) & 255) as u8);
        }
        // self.ws.write(self.data.iter().cloned()).await.ok();
        self.ws.write(self.data).await.ok();
    }

    pub async fn disable(&mut self) {
        for i in 0..self.num_leds {
            self.data[i] = RGB8::default();
        }
        // self.ws.write(brightness(self.data.iter().cloned(), 32)).await.ok();
        self.ws.write(self.data).await.ok();
    }

    pub async fn show(&mut self) {
        if let Ok(new_message) = with_timeout(Duration::from_millis(100), self.rx.receive()).await {
            info!("led message {:?}", new_message);
            self.process_event(new_message).await;
        }
    }

    async fn process_event(&mut self, event: SmartLedEvent) {
        match event {
            SmartLedEvent::On => {
                self.enable().await;
            }
            SmartLedEvent::Off => {
                self.disable().await;
                
            }
            SmartLedEvent::Value(values) => {
                for i in 0..self.num_leds {
                    self.data[i] = RGB8::new(values[0], values[1], values[2]);
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


fn wheel(mut wheel_pos: u8) -> RGB8 {
    wheel_pos = 255 - wheel_pos;
    if wheel_pos < 85 {
        return (255 - wheel_pos * 3, 0, wheel_pos * 3).into();
    }
    if wheel_pos < 170 {
        wheel_pos -= 85;
        return (0, wheel_pos * 3, 255 - wheel_pos * 3).into();
    }
    wheel_pos -= 170;
    (wheel_pos * 3, 255 - wheel_pos * 3, 0).into()
}