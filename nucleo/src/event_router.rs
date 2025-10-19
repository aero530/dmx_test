//! Event router to send commands between tasks
use crate::button::ButtonEvent;
use crate::button_array::{KeyPadButton, KeyPadEvent};
use crate::channels::*;
use crate::eeprom::EepromEvent;
use crate::ui::{MenuData, ModuleType, SmartLedColorMode, UiEvent};
// use crate::led::LedEvent;
use crate::pwm_i2c::PwmEvent;
use crate::smart_led::SmartLedEvent;
use crate::DMX_BUFF_SIZE;
use defmt::*;
use embassy_time::{with_timeout, Duration};

/// Data stored for global use (primarily for logging / terminal display)
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct GlobalData {
    // pub dmx_address: u16,
    pub menu_settings: MenuData,
    // pub dmx: [u8; DMX_BUFF_SIZE],
    pub module_type: ModuleType,
}

/// Events the router watches for.  These trigger the router to pass along an event to another object.
#[derive(Format)]
pub enum RouterEvent {
    UsbCommand(u8),
    ButtonArray((KeyPadButton, KeyPadEvent)),
    Button(ButtonEvent),
    MenuDataUpdate(MenuData),
    UpdateSettings(MenuData),
    UpdateModuleType(ModuleType),
    SendMeSettings,
}

#[derive(Format)]
pub enum DmxEvent {
    DmxPacket([u8; DMX_BUFF_SIZE]),
}

pub struct Router {
    /// Listen for event router tasks
    pub channel: RouterChannelRx,
    pub channel_dmx: DmxChannelRx,

    // /// Channel to send LED events
    // pub channel_led: LedChannelTx,
    /// Channel to send LED events
    // pub channel_pwm: PwmChannelTx,
    pub channel_pwm_i2c: PwmChannelTx,

    /// Channel to send Smart Led events
    pub channel_smart_led: SmartLedChannelTx,

    /// Channel to send UI events
    pub channel_ui: UiChannelTx,

    /// Channel to send data to the eeprom (write to the eeprom)
    pub channel_eeprom: EepromChannelTx,

    /// Channel to send global data events
    pub channel_log: GlobalDataChannelTx,

    // Global data store
    pub data: GlobalData,
}

impl Router {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel: RouterChannelRx,
        channel_dmx: DmxChannelRx,
        // channel_led: LedChannelTx,
        // channel_pwm: PwmChannelTx,
        channel_pwm_i2c: PwmChannelTx,
        channel_smart_led: SmartLedChannelTx,
        channel_ui: UiChannelTx,
        channel_eeprom: EepromChannelTx,
        channel_log: GlobalDataChannelTx,
    ) -> Self {
        Self {
            channel,
            channel_dmx,
            // channel_led,
            // channel_pwm,
            channel_pwm_i2c,
            channel_smart_led,
            channel_ui,
            channel_log,
            channel_eeprom,
            data: GlobalData::default(),
        }
    }

    pub async fn process_event(&mut self, event: RouterEvent) {
        match event {
            RouterEvent::UsbCommand(input) => {
                        info!("USB command {}", input);
                    }
            RouterEvent::Button(evt) => {
                        info!("Button event {}", evt);
                    }
            RouterEvent::ButtonArray((btn, evt)) => {
                        info!("Button array event {} {}", btn, evt);
                        match btn {
                            KeyPadButton::A => {}
                            KeyPadButton::B => {}
                            KeyPadButton::C => {}
                            KeyPadButton::D => {
                                let _ = self.channel_ui.try_send(UiEvent::Down);
                            }
                            KeyPadButton::N0 => {
                                let _ = self.channel_ui.try_send(UiEvent::Select);
                            }
                            KeyPadButton::N1 => {}
                            KeyPadButton::N2 => {}
                            KeyPadButton::N3 => {}
                            KeyPadButton::N4 => {}
                            KeyPadButton::N5 => {}
                            KeyPadButton::N6 => {}
                            KeyPadButton::N7 => {}
                            KeyPadButton::N8 => {}
                            KeyPadButton::N9 => {}
                            KeyPadButton::Star => {
                                let _ = self.channel_ui.try_send(UiEvent::Esc);
                            }
                            KeyPadButton::Pound => {
                                let _ = self.channel_ui.try_send(UiEvent::Up);
                            }
                            KeyPadButton::None => {}
                        }
                    }
            RouterEvent::MenuDataUpdate(new_data) => {
                        self.data.menu_settings = new_data;
                        info!("New data RX {}", new_data);
                        let _ = self.channel_eeprom.try_send(EepromEvent::StoreSettings(new_data));
                    }
            RouterEvent::UpdateSettings(menu_data) => {
                self.data.menu_settings = menu_data;

                        info!("Read eeprom data {}", menu_data);
                        // let _ = self.channel.try_send(RouterEvent::MenuDataUpdate(new_data));
                        let _ = self.channel_ui.try_send(UiEvent::Load(menu_data));
                    }
            RouterEvent::UpdateModuleType(module_type) => {
                info!("Read module type as {}", module_type);
                self.data.module_type = module_type;
            }
            RouterEvent::SendMeSettings => {
                let _ = self.channel_ui.try_send(UiEvent::Load(self.data.menu_settings));
            }
        }
    }
    pub async fn process_dmx(&mut self, event: DmxEvent) {
        match event {
            DmxEvent::DmxPacket(input) => {
                trace!("Router got DMX data {}", input[0..24]);

                let dmx_start = self.data.menu_settings.dmx_address;
                match self.data.menu_settings.module {
                    crate::ui::ModuleSettings::Pwm(pwm_settings) => {
                        error!("Module LED settings not programmed");
                        
                    },
                    crate::ui::ModuleSettings::SmartLed(smart_led_settings) => {
                        let color_size = smart_led_settings.color_mode.addr_size();
                        let led_per_port = smart_led_settings.leds_per_port;
                       
                        info!("color size {}", color_size);
                        info!("leds {}", led_per_port);
                       
                        let dmx_size = match smart_led_settings.grouping {
                            crate::ui::SmartLedGrouping::Individual => {
                                info!("individual");
                                // each LED on each port is addressed on its own
                                // let mut s = 0;
                                // for p in led_per_port {
                                //     s += p;
                                // }
                                // s as usize * color_size
                                led_per_port.iter().sum::<u16>() as usize * color_size

                            },
                            crate::ui::SmartLedGrouping::CombineByPort => {
                                info!("CombineByPort");
                                // all LEDs on a port are addressed as one
                                led_per_port.iter().filter(|x| **x > 0).sum::<u16>() as usize * color_size
                            },
                            crate::ui::SmartLedGrouping::CombineByModule => {
                                info!("CombineByModule");
                                // all leds on the module are addressed as one
                                color_size
                            },
                        };
                        info!("DMX address size {}", dmx_size);
                        
                        
                    },
                }

                // self.data.dmx = input;

                // let _ = self.channel_pwm.try_send(PwmEvent::Value([input[1], input[2], input[3]]));
                let _ = self.channel_pwm_i2c.try_send(PwmEvent::Value([input[1], input[2], input[3]]));
                // let _ = self.channel_smart_led.try_send(SmartLedEvent::Value([
                //     input[1], input[2], input[3], input[4],
                // ]));

                let _ = self.channel_smart_led.try_send(SmartLedEvent::Value([input[1], input[2], input[3], input[4]]));

                // let _ = self.channel_smart_led.try_send(SmartLedEvent::Value([input[1], input[2], input[3]]));
            }
        }
    }

}

#[embassy_executor::task]
pub async fn event_router(mut router: Router) {
    loop {
        if let Ok(new_message) = with_timeout(Duration::from_millis(10), router.channel.receive()).await {
            router.process_event(new_message).await;
        }

        if let Ok(dmx_message) = router.channel_dmx.try_receive() {
            router.process_dmx(dmx_message).await;
        }
    }
}
