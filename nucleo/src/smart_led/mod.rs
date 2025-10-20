//! Smart LED Interface
use defmt::Format;
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_time::{with_timeout, Duration};
use smart_leds::{SmartLedsWriteAsync, RGB8};

use crate::channels::SmartLedChannelRx;

mod ws2812_async;
use ws2812_async::{Grb, Ws2812};
pub use ws2812_async::NUM_LEDS_MAX;

pub enum SmartLedEvent {
    // On,
    // Off,
    Value(([u16; 4], [u8; 3])),
    Individual(([u16; 4], [[RGB8; NUM_LEDS_MAX]; 4])),
    CombinedByPort(([u16; 4], [RGB8; 4])),
    CombinedByModule(([u16; 4], RGB8)),
}

impl Format for SmartLedEvent {
    fn format(&self, f: defmt::Formatter) {
        match self {
            SmartLedEvent::Value((_leds, colors)) => defmt::write!(f, "Value: ({}, {}, {})", colors[0], colors[1], colors[2]),
            SmartLedEvent::Individual((_leds, colors)) => defmt::write!(f, "Individual: ({}, {}, {})...", colors[0][0].r, colors[0][0].g, colors[0][0].b),
            SmartLedEvent::CombinedByPort((_leds, colors)) => defmt::write!(f, "CombinedByPort: ({}, {}, {}), ({}, {}, {}), ({}, {}, {}), ({}, {}, {})", colors[0].r, colors[0].g, colors[0].b, colors[1].r, colors[1].g, colors[1].b, colors[2].r, colors[2].g, colors[2].b, colors[3].r, colors[3].g, colors[3].b),
            SmartLedEvent::CombinedByModule((_leds, colors)) => defmt::write!(f, "CombinedByModule: ({}, {}, {})", colors.r, colors.g, colors.b),
        }
        
    }
}


pub struct SmartLed<'a> {
    ws_1: Ws2812<Spi<'a, Async>, Grb>,
    ws_2: Ws2812<Spi<'a, Async>, Grb>,
    ws_3: Ws2812<Spi<'a, Async>, Grb>,
    ws_4: Ws2812<Spi<'a, Async>, Grb>,
    num_leds: [u16; 4],
    data: [[RGB8; NUM_LEDS_MAX]; 4],
    rx: SmartLedChannelRx,
}

impl<'a> SmartLed<'a> {
    pub fn new(spi_1: Spi<'a, Async>, spi_2: Spi<'a, Async>, spi_3: Spi<'a, Async>, spi_4: Spi<'a, Async>, num_leds: [u16; 4], rx: SmartLedChannelRx) -> Self {
        let ws_1: Ws2812<_, Grb> = Ws2812::new(spi_1);
        let ws_2: Ws2812<_, Grb> = Ws2812::new(spi_2);
        let ws_3: Ws2812<_, Grb> = Ws2812::new(spi_3);
        let ws_4: Ws2812<_, Grb> = Ws2812::new(spi_4);
        // let ws = [ws_1, ws_2, ws_3, ws_4];
        let data = [[RGB8::default(); NUM_LEDS_MAX], [RGB8::default(); NUM_LEDS_MAX], [RGB8::default(); NUM_LEDS_MAX], [RGB8::default(); NUM_LEDS_MAX]];

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
        for (port, num) in self.num_leds.iter().enumerate() {   
            for i in 0..*num {
                self.data[port][i as usize] = RGB8::default();
            }
        }
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
        let prev_lengths = self.num_leds;


        match event {
            SmartLedEvent::Value((num_leds, _)) => self.num_leds = num_leds,
            SmartLedEvent::Individual((num_leds, _)) => self.num_leds = num_leds,
            SmartLedEvent::CombinedByPort((num_leds, _)) => self.num_leds = num_leds,
            SmartLedEvent::CombinedByModule((num_leds, _)) => self.num_leds = num_leds,
        }

        for (port, num) in self.num_leds.iter().enumerate() {
            for i in 0..*num {
                match event {
                    SmartLedEvent::Value((_, colors)) => self.data[port][i as usize] = RGB8::new(colors[0], colors[1], colors[2]),
                    SmartLedEvent::Individual((_, colors)) => self.data[port][i as usize] = colors[port][i as usize],
                    SmartLedEvent::CombinedByPort((_, colors)) => self.data[port][i as usize] = colors[port],
                    SmartLedEvent::CombinedByModule((_, colors)) => self.data[port][i as usize] = colors,
                }
            }
            for i in *num..(prev_lengths[port] + 1) {
                self.data[port][i as usize] = RGB8::default();
            }
        }

        self.send_to_leds().await.ok();
    }

    async fn send_to_leds(&mut self) -> Result<(), &'static str> {
        let results = embassy_futures::join::join_array([self.ws_1.write(self.data[0]), self.ws_2.write(self.data[1]), self.ws_3.write(self.data[2]), self.ws_4.write(self.data[3])]).await;

        let a = results.iter().filter(|&&r| r.is_err()).count();

        if a > 0 {
            Err("SPI write error to LEDs.")
        } else {
            Ok(())
        }
    }

}

#[embassy_executor::task]
pub async fn smart_led_task(spi_1: Spi<'static, Async>, spi_2: Spi<'static, Async>, spi_3: Spi<'static, Async>, spi_4: Spi<'static, Async>, rx: SmartLedChannelRx) {

    let mut smart_led = SmartLed::new(spi_1, spi_2, spi_3, spi_4, [0,0,0,0], rx);
    smart_led.enable().await;
    loop {
        smart_led.show().await;
    }
}
