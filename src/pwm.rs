//! LED & Button interaction
use defmt::{info, Format};
use embassy_stm32::peripherals::{TIM12, TIM3, TIM4};
use embassy_stm32::timer::simple_pwm::SimplePwmChannels;
use embassy_time::{with_timeout, Duration};

use crate::channels::{PwmChannelRx, RouterChannelTx};
use crate::event_router::RouterEvent;

#[derive(Format)]
pub enum PwmEvent {
    On,
    Off,
    Value([u8;3]),
}

pub struct Pwm<'a> {
    channels_a: SimplePwmChannels<'a, TIM12>,
    channels_b: SimplePwmChannels<'a, TIM3>,
    channels_c: SimplePwmChannels<'a, TIM4>,
    rx: PwmChannelRx,
}

impl<'a> Pwm<'a> {
    pub fn new(channels_a: SimplePwmChannels<'a, TIM12>, channels_b: SimplePwmChannels<'a, TIM3>, channels_c: SimplePwmChannels<'a, TIM4>, rx: PwmChannelRx) -> Self {
        Self { channels_a, channels_b, channels_c, rx }
    }

    pub fn enable(&mut self) {
        self.channels_a.ch1.enable();
        self.channels_a.ch2.disable();
        self.channels_a.ch3.disable();
        self.channels_a.ch4.disable();
        self.channels_b.ch1.disable();
        self.channels_b.ch2.disable();
        self.channels_b.ch3.enable();
        self.channels_b.ch4.disable();
        self.channels_c.ch1.disable();
        self.channels_c.ch2.enable();
        self.channels_c.ch3.disable();
        self.channels_c.ch4.disable();
    }

    pub fn disable(&mut self) {
        self.channels_a.ch1.disable();
        self.channels_a.ch2.disable();
        self.channels_a.ch3.disable();
        self.channels_a.ch4.disable();
        self.channels_b.ch1.disable();
        self.channels_b.ch2.disable();
        self.channels_b.ch3.disable();
        self.channels_b.ch4.disable();
        self.channels_c.ch1.disable();
        self.channels_c.ch2.disable();
        self.channels_c.ch3.disable();
        self.channels_c.ch4.disable();
        
        self.channels_a.ch1.set_duty_cycle_fully_off();
        self.channels_a.ch2.set_duty_cycle_fully_off();
        self.channels_a.ch3.set_duty_cycle_fully_off();
        self.channels_a.ch4.set_duty_cycle_fully_off();
        self.channels_b.ch1.set_duty_cycle_fully_off();
        self.channels_b.ch2.set_duty_cycle_fully_off();
        self.channels_b.ch3.set_duty_cycle_fully_off();
        self.channels_b.ch4.set_duty_cycle_fully_off();
        self.channels_c.ch1.set_duty_cycle_fully_off();
        self.channels_c.ch2.set_duty_cycle_fully_off();
        self.channels_c.ch3.set_duty_cycle_fully_off();
        self.channels_c.ch4.set_duty_cycle_fully_off();
    }

    pub async fn show(&mut self) {
        if let Ok(new_message) = with_timeout(Duration::from_millis(100), self.rx.receive()).await {
            info!("led message {:?}", new_message);
            self.process_event(new_message).await;
        }
    }

    async fn process_event(&mut self, event: PwmEvent) {
        match event {
            PwmEvent::On => {
                self.enable();
            }
            PwmEvent::Off => {
                self.disable();
                
            }
            PwmEvent::Value(values) => {
                self.channels_a.ch1.set_duty_cycle_fraction(values[0] as u16, 255);
                self.channels_b.ch3.set_duty_cycle_fraction(values[1] as u16, 255);
                self.channels_c.ch2.set_duty_cycle_fraction(values[2] as u16, 255);
                // set pwm to value
            }
        }
    }
}


#[embassy_executor::task]
pub async fn pwm_task(cs1: SimplePwmChannels<'static, TIM12>, cs2: SimplePwmChannels<'static, TIM3>, cs3: SimplePwmChannels<'static, TIM4>, rx: PwmChannelRx) {
    let mut pwm = Pwm::new(cs1, cs2, cs3, rx);
    pwm.enable();
    loop {
        pwm.show().await;
    }
}
