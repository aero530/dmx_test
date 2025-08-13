//! Event router to send commands between tasks
use crate::buttons::{KeyPadButton, KeyPadEvent};
use crate::ui::UiEvent;
use crate::{channels::*};
// use crate::led::LedEvent;
use crate::pwm::PwmEvent;
use crate::smart_led::SmartLedEvent;
use defmt::*;
use embassy_time::{with_timeout, Duration};

/// Data stored for global use (primarily for logging / terminal display)
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct GlobalData {
    pub dmx_address: u16,
    // pub dmx: [u8; 513],
}

impl Default for GlobalData {
    fn default() -> Self { 
        Self {
            dmx_address: 0,
            // dmx: [0; 513]
        }
    }
}

/// Events the router watches for.  These trigger the router to pass along an event to another object.
#[derive(Format)]
pub enum RouterEvent {
    UsbCommand(u8),
    Button((KeyPadButton, KeyPadEvent)),
}

#[derive(Format)]
pub enum DmxEvent {
    DmxPacket([u8;513]),
}

pub struct Router {
    /// Listen for event router tasks
    pub channel: RouterChannelRx,
    pub channel_dmx: DmxChannelRx,
    
    // /// Channel to send LED events
    // pub channel_led: LedChannelTx,

    /// Channel to send LED events
    pub channel_pwm: PwmChannelTx,
    pub channel_pwm_i2c: PwmChannelTx,
    
    /// Channel to send Smart Led events
    pub channel_smart_led: SmartLedChannelTx,
    
    /// Channel to send UI events
    pub channel_ui: UiChannelTx,

    /// Channel to send global data events
    // pub channel_log: GlobalDataChannelTx,

    // Global data store
    pub data: GlobalData,
}

impl Router {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel: RouterChannelRx,
        channel_dmx: DmxChannelRx,
        // channel_led: LedChannelTx,
        channel_pwm: PwmChannelTx,
        channel_pwm_i2c: PwmChannelTx,
        channel_smart_led: SmartLedChannelTx,
        channel_ui: UiChannelTx,
        // channel_log: GlobalDataChannelTx,
    ) -> Self {
        Self {
            channel,
            channel_dmx,
            // channel_led,
            channel_pwm,
            channel_pwm_i2c,
            channel_smart_led,
            channel_ui,
            // channel_log,
            data: GlobalData::default(),
        }
    }

    pub async fn process_event(&mut self, event: RouterEvent) {
        match event {
            RouterEvent::UsbCommand(input) => match input {
                // 1 => {
                //     let _ = self.channel_led.try_send(LedEvent::On);
                // }
                // 2 => {
                //     let _ = self.channel_led.try_send(LedEvent::Off);
                // }
                // _ => {
                //     let _ = self.channel_led.try_send(LedEvent::Blink);
                // }
                _ => {
                    info!("USB command {}", input);
                }
            },

            RouterEvent::Button((btn, evt)) => {
                info!("Button event {} {}", btn, evt);
                match btn {
                    KeyPadButton::A => {},
                    KeyPadButton::B => {},
                    KeyPadButton::C => {},
                    KeyPadButton::D => {},
                    KeyPadButton::N0 => {},
                    KeyPadButton::N1 => {},
                    KeyPadButton::N2 => {},
                    KeyPadButton::N3 => {},
                    KeyPadButton::N4 => {},
                    KeyPadButton::N5 => {},
                    KeyPadButton::N6 => {},
                    KeyPadButton::N7 => {},
                    KeyPadButton::N8 => {},
                    KeyPadButton::N9 => {},
                    KeyPadButton::Star => {let _ = self.channel_ui.try_send(UiEvent::PreviousTab);},
                    KeyPadButton::Pound => {let _ = self.channel_ui.try_send(UiEvent::NextTab);},
                    KeyPadButton::None => {},
                }
            }
        }
    }
    pub async fn process_dmx(&mut self, event: DmxEvent) {
        match event {
            DmxEvent::DmxPacket(input) => {
                // info!("Router got DMX data");
                // for i in 0..8 {
                //     // +1 because we skip the address bit
                //     info!("{}",input[(i*64+1)..(i*64-1+1)]);
                // }
                // The first byte should be 0x00 to start the packet transmission
                // info!("{}",input[1..11]);
                
                // self.data.dmx = input;

                let _ = self.channel_pwm.try_send(PwmEvent::Value([input[1], input[2], input[3]]));
                let _ = self.channel_pwm_i2c.try_send(PwmEvent::Value([input[1], input[2], input[3]]));
                let _ = self.channel_smart_led.try_send(
                    SmartLedEvent::Value([input[1], input[2], input[3], input[4]])
                );
                // let _ = self.channel_smart_led.try_send(SmartLedEvent::Value([input[1], input[2], input[3]]));
            },
            
        }
    }
}

#[embassy_executor::task]
pub async fn event_router(mut router: Router) {
    loop {
        if let Ok(new_message) =
            with_timeout(Duration::from_millis(10), router.channel.receive()).await
        {
            router.process_event(new_message).await;
        }

        if let Ok(dmx_message) = router.channel_dmx.try_receive() {
            router.process_dmx(dmx_message).await;
        }
    }
}
