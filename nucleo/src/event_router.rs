//! Event router to send commands between tasks
use crate::button::ButtonEvent;
use crate::button_array::{KeyPadButton, KeyPadEvent};
use crate::channels::*;
use crate::eeprom::EepromEvent;
use crate::ui::{EthernetIPMode, InputMode, MenuData, ModuleType, UiEvent};
// use crate::led::LedEvent;
use crate::pwm_i2c::PwmEvent;
use crate::smart_led::{SmartLedEvent, NUM_LEDS_MAX};
use crate::DMX_BUFF_SIZE;
use defmt::*;
use embassy_time::{with_timeout, Duration};
use smart_leds::{RGB, RGB8};

/// Data stored for global use (primarily for logging / terminal display)
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct GlobalData {
    // pub dmx_address: u16,
    pub menu_settings: MenuData,
    // pub dmx: [u8; DMX_BUFF_SIZE],
    pub module_type: ModuleType,
    pub mac: [u8; 6],
    pub boot_complete: bool,
}

#[derive(Format)]
pub enum ReturnChannel {
    Main,
    Ui,
}

/// Events the router watches for.  These trigger the router to pass along an event to another object.
#[derive(Format)]
pub enum RouterEvent {
    UsbCommand(u8),
    ButtonArray((KeyPadButton, KeyPadEvent)),
    Button(ButtonEvent),
    WriteSettingsToEeprom(MenuData),
    
    StoreSettings(MenuData),
    StoreModuleType(ModuleType),
    StoreMacAddress([u8; 6]),
    StoreBootComplete(bool),
    
    GetModuleType(ReturnChannel),
    GetMacAddress(ReturnChannel),
    GetSettings(ReturnChannel),
}

#[derive(Format)]
pub enum DmxEvent {
    DmxPacket([u8; DMX_BUFF_SIZE]),
    ArtNetPacket([u8; DMX_BUFF_SIZE]),
}

#[derive(Format)]
pub enum MainEvent {
    // ReturnIpMode(EthernetIPMode),
    ReturnModuleType(ModuleType),
    ReturnSettings(MenuData),
    ReturnMacAddress([u8; 6]),
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

    /// Channel to send data back to the main function (used during boot process)
    pub channel_main: MainChannelTx,

    /// Channel to send global data events
    #[allow(unused)]
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
        channel_main: MainChannelTx,
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
            channel_main,
            data: GlobalData::default(),
        }
    }

    pub async fn process_event(&mut self, event: RouterEvent) {
        match event {
            RouterEvent::UsbCommand(input) => {
                        info!("USB command {}", input);
                    },
            RouterEvent::Button(evt) => {
                        info!("Button event {}", evt);
                    },
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
                    },
            RouterEvent::WriteSettingsToEeprom(menu_data) => {
                        self.data.menu_settings = menu_data;
                        info!("Store settings in eeprom {}", menu_data);
                        let _ = self.channel_eeprom.try_send(EepromEvent::WriteSettings(menu_data));
                    },
            RouterEvent::StoreSettings(menu_data) => {
                        self.data.menu_settings = menu_data;
                        info!("Update settings on display {}", menu_data);
                        let _ = self.channel_ui.try_send(UiEvent::Load(menu_data));
                    },
            RouterEvent::StoreModuleType(module_type) => {
                        info!("Data store update - module type {}", module_type);
                        self.data.module_type = module_type;
                    },
            RouterEvent::StoreMacAddress(mac) => {
                        info!("Data store update - mac {:#X}", mac);
                        self.data.mac = mac;
                    },

            RouterEvent::GetModuleType(ch) => {
                match ch {
                    ReturnChannel::Main => {let _ = self.channel_main.try_send(MainEvent::ReturnModuleType(self.data.module_type));},
                    ReturnChannel::Ui => {}//self.channel_ui.try_send(MainEvent::ReturnModuleType(self.data.module_type)),
                }
            },
            RouterEvent::GetMacAddress(ch) => {
                info!("Router event get mac {:#X}", self.data.mac);
                match ch {
                    ReturnChannel::Main => {let _ = self.channel_main.try_send(MainEvent::ReturnMacAddress(self.data.mac));},
                    ReturnChannel::Ui => {}//self.channel_ui.try_send(MainEvent::ReturnMacAddress(self.data.mac)),
                }
            },
            RouterEvent::GetSettings(ch) => {
                match ch {
                    ReturnChannel::Main => {let _ = self.channel_main.try_send(MainEvent::ReturnSettings(self.data.menu_settings));},
                    ReturnChannel::Ui => {let _ = self.channel_ui.try_send(UiEvent::Load(self.data.menu_settings));},
                }
            },
            RouterEvent::StoreBootComplete(complete) => {
                self.data.boot_complete = complete;
            }


        }
    }
    pub async fn process_dmx(&mut self, event: DmxEvent) {

        if self.data.boot_complete {

        
        let event_data = match (event, self.data.menu_settings.input_mode) {
            (DmxEvent::DmxPacket(data), InputMode::DMX) => {
                // info!("DMX Packet - DMX Mode");
                Some(data)
            },
            (DmxEvent::DmxPacket(data), InputMode::ArtNet) => {
                // info!("DMX Packet - ArtNet Mode");
                None
            },
            (DmxEvent::ArtNetPacket(data), InputMode::DMX) => {
                // info!("ArtNet Packet - DMX Mode");
                None
            },
            (DmxEvent::ArtNetPacket(data), InputMode::ArtNet) => {
                info!("ArtNet Packet - ArtNet Mode");
                Some(data)
            },
        };


        if let Some(input) = event_data {

                // info!("Router got DMX data {}", input[0..24]);

                let dmx_start = self.data.menu_settings.dmx_address as usize;

                match self.data.menu_settings.module {
                    crate::ui::ModuleSettings::Pwm(_pwm_settings) => {
                        error!("Module LED settings not programmed");
                        let _ = self.channel_pwm_i2c.try_send(PwmEvent::Value([input[1], input[2], input[3]]));
                    },
                    crate::ui::ModuleSettings::SmartLed(smart_led_settings) => {
                        let color_size = smart_led_settings.color_mode.addr_size();
                        let led_per_port = smart_led_settings.leds_per_port;
                       
                        // info!("color size {}", color_size);
                        // info!("leds {}", led_per_port);
                       
                        let dmx_size = match smart_led_settings.grouping {
                            crate::ui::SmartLedGrouping::Individual => {
                                led_per_port.iter().sum::<u16>() as usize * color_size
                            },
                            crate::ui::SmartLedGrouping::CombineByPort => {
                                // all LEDs on a port are addressed as one
                                led_per_port.iter().filter(|x| **x > 0).map(|_| 1).sum::<u16>() as usize * color_size
                            },
                            crate::ui::SmartLedGrouping::CombineByModule => {
                                // all leds on the module are addressed as one
                                color_size
                            },
                        };
                        
                        match smart_led_settings.grouping {
                            crate::ui::SmartLedGrouping::Individual => {
                                
                                let range_1_inc = (
                                    dmx_start, //1
                                    upper_limit(dmx_start + led_per_port[0] as usize * color_size - 1, 512)
                                );

                                let range_2_inc = (
                                    range_1_inc.1+1,
                                    upper_limit(range_1_inc.1+1 + led_per_port[1] as usize * color_size - 1, 512)
                                );

                                let range_3_inc = (
                                    range_2_inc.1+1,
                                    upper_limit(range_2_inc.1+1 + led_per_port[2] as usize * color_size - 1, 512)
                                );

                                let range_4_inc = (
                                    range_3_inc.1+1,
                                    upper_limit(range_3_inc.1+1 + led_per_port[3] as usize * color_size - 1, 512)
                                );

                                let ranges = [range_1_inc, range_2_inc, range_3_inc, range_4_inc];

                                
                                let mut colors : [[RGB8; NUM_LEDS_MAX]; 4] = [[RGB8::default(); NUM_LEDS_MAX], [RGB8::default(); NUM_LEDS_MAX], [RGB8::default(); NUM_LEDS_MAX], [RGB8::default(); NUM_LEDS_MAX]];

                                led_per_port.iter().enumerate().for_each(|(port, ledperport)| {
                                    &input[ranges[port].0 ..= ranges[port].1]
                                    .chunks(smart_led_settings.color_mode.addr_size())
                                    .enumerate()
                                    .for_each(|(i,a)| {
                                        match smart_led_settings.color_mode {
                                            crate::ui::SmartLedColorMode::RGB => {
                                                colors[port][i] = RGB::new(a[0], a[1], a[2]);
                                            },
                                            crate::ui::SmartLedColorMode::RGBW => {
                                                // White not used at the moment
                                                error!("Using RGBW color space but that it not implimented yet.");
                                                colors[port][i] = RGB::new(a[0], a[1], a[2]);
                                            },
                                        }
                                    });


                                });

                                let _ = self.channel_smart_led.try_send(SmartLedEvent::Individual((led_per_port, colors)));
                            },
                            crate::ui::SmartLedGrouping::CombineByPort => {
      
                                let (color_1, color_2, color_3, color_4) = match smart_led_settings.color_mode {
                                    crate::ui::SmartLedColorMode::RGB => {
                                        (
                                            RGB8::new(input[dmx_start+0], input[dmx_start+1], input[dmx_start+2]),
                                            RGB8::new(input[dmx_start+3], input[dmx_start+4], input[dmx_start+5]),
                                            RGB8::new(input[dmx_start+6], input[dmx_start+7], input[dmx_start+8]),
                                            RGB8::new(input[dmx_start+9], input[dmx_start+10], input[dmx_start+11])
                                        )
                                    },
                                    crate::ui::SmartLedColorMode::RGBW => {
                                        // White not used at the moment
                                        error!("Using RGBW color space but that it not implimented yet.");
                                        (
                                            RGB8::new(input[dmx_start+0], input[dmx_start+1], input[dmx_start+2]),
                                            RGB8::new(input[dmx_start+4], input[dmx_start+5], input[dmx_start+6]),
                                            RGB8::new(input[dmx_start+8], input[dmx_start+9], input[dmx_start+10]),
                                            RGB8::new(input[dmx_start+11], input[dmx_start+12], input[dmx_start+13])
                                        )
                                    },
                                };
                                let _ = self.channel_smart_led.try_send(SmartLedEvent::CombinedByPort((led_per_port, [color_1, color_2, color_3, color_4])));
                            },
                            crate::ui::SmartLedGrouping::CombineByModule => {
                                let color = RGB8::new(input[dmx_start], input[dmx_start+1], input[dmx_start+2]);
                                let _ = self.channel_smart_led.try_send(SmartLedEvent::CombinedByModule((led_per_port, color)));
                            },
                        }
                        
                        
                    },
                }
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

fn upper_limit(input: usize, limit: usize) -> usize {
    if input > limit {
        error!("DMX address range error");
        limit
    } else {
        input
    }
}