//! Event router to send commands between tasks
use crate::channels::*;
use crate::led::LedEvent;
use defmt::*;
use embassy_time::{with_timeout, Duration};

/// Data stored for global use (primarily for logging / terminal display)
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct GlobalData {
    pub button: u8,
    pub dmx: [u8; 513],
}

impl Default for GlobalData {
    fn default() -> Self { 
        Self {
            button: 0,
            dmx: [0; 513]
        }
    }
}

/// Events the router watches for.  These trigger the router to pass along an event to another object.
// #[derive(Copy, Clone)]
pub enum RouterEvent {
    UsbCommand(u8),

    ButtonHold,
    ButtonPressed,
    ButtonDouble,

    DmxPacket([u8;513]),
}

pub struct Router {
    /// Listen for event router tasks
    pub channel: RouterChannelRx,
    
    /// Channel to send LED events
    pub channel_led: LedChannelTx,
    
    /// Channel to send global data events
    pub channel_log: GlobalDataChannelTx,
    // Global data store
    pub data: GlobalData,
}

impl Router {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel: RouterChannelRx,
        channel_led: LedChannelTx,
        channel_log: GlobalDataChannelTx,
    ) -> Self {
        Self {
            channel,
            channel_led,
            channel_log,
            data: GlobalData::default(),
        }
    }

    pub async fn process_event(&mut self, event: RouterEvent) {
        match event {
            RouterEvent::ButtonHold => {
                let _ = self.channel_led.try_send(LedEvent::Blink);
                self.data.button = 1;
                self.channel_log.send(self.data);
            }
            RouterEvent::ButtonPressed => {
                info!("Event router button pressed");
                let _ = self.channel_led.try_send(LedEvent::On);
                self.data.button = 2;
                self.channel_log.send(self.data);
            }
            RouterEvent::ButtonDouble => {
                let _ = self.channel_led.try_send(LedEvent::Off);
                self.data.button = 3;
                self.channel_log.send(self.data);
            }

            RouterEvent::UsbCommand(input) => match input {
                1 => {
                    let _ = self.channel_led.try_send(LedEvent::On);
                }
                2 => {
                    let _ = self.channel_led.try_send(LedEvent::Off);
                }
                _ => {
                    let _ = self.channel_led.try_send(LedEvent::Blink);
                }
            },
            RouterEvent::DmxPacket(input) => {
                info!("Router got DMX data");
                // for i in 0..8 {
                //     // +1 because we skip the address bit
                //     info!("{}",input[(i*64+1)..(i*64-1+1)]);
                // }
                // The first byte should be 0x00 to start the packet transmission
                info!("{}",input[1..11]);
                self.data.dmx = input;
            },
        }
    }
}

#[embassy_executor::task]
pub async fn event_router(mut router: Router) {
    loop {
        if let Ok(new_message) =
            with_timeout(Duration::from_millis(2), router.channel.receive()).await
        {
            router.process_event(new_message).await;
        }
    }
}
