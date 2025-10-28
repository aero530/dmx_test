//! Event router to send commands between tasks
use core::net::Ipv4Addr;

use crate::button::ButtonEvent;
use crate::button_array::{KeyPadButton, KeyPadEvent};
use crate::channels::*;
use crate::eeprom::EepromEvent;
use crate::ui::{EthernetIPMode, InputMode, MenuData, ModuleType, UiEvent, IpAddrMenu, ModuleSettings, SmartLedPortMode, SmartLedColorMode};
// use crate::led::LedEvent;
use crate::pwm_i2c::PwmEvent;
use crate::smart_led::{SmartLedEvent, NUM_LEDS_MAX};
use crate::{DMX_BUFF_SIZE, SMARTLED_PORT_COUNT};
use defmt::*;
use embassy_time::{with_timeout, Duration};
use smart_leds::{RGB, RGB8};

use heapless::Vec;

/// Data stored for global use (primarily for logging / terminal display)
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct GlobalData {
    // pub dmx_address: u16,
    pub menu_settings: MenuData,
    // pub dmx: [u8; DMX_BUFF_SIZE],
    pub module_type: Option<ModuleType>,
    pub mac: Option<[u8; 6]>,
    pub boot_complete: bool,
    // pub ip_addr: Option<Ipv4Addr>
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
    
    StoreSettings(Option<MenuData>),
    StoreModuleType(Option<ModuleType>),
    StoreMacAddress(Option<[u8; 6]>),
    StoreBootComplete(bool),
    StoreIpAddr(Option<Ipv4Addr>),
    
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
    ReturnModuleType(Option<ModuleType>),
    ReturnSettings(MenuData),
    ReturnMacAddress(Option<[u8; 6]>),
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
                        if let Some(x) = menu_data {
                            self.data.menu_settings = x;
                            info!("Update settings on display {}", x);
                            let _ = self.channel_ui.try_send(UiEvent::Load(x));
                        }
                    },
            RouterEvent::StoreModuleType(module_type) => {
                        self.data.module_type = module_type;
                    },
            RouterEvent::StoreMacAddress(mac) => {
                    self.data.mac = mac;
                    },
            RouterEvent::StoreBootComplete(complete) => {
                self.data.boot_complete = complete;
            },
            RouterEvent::StoreIpAddr(data) => {
                if let Some(addr) = data {
                    self.data.menu_settings.ip_addr = addr.into();
                } else {
                    self.data.menu_settings.ip_addr = IpAddrMenu::default();
                }
                let _ = self.channel_ui.try_send(UiEvent::Load(self.data.menu_settings));
                
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

        }
    }
    pub async fn process_dmx(&mut self, event: DmxEvent) {

        if self.data.boot_complete {

        
        let event_data = match (event, self.data.menu_settings.input_mode) {
            (DmxEvent::DmxPacket(data), InputMode::DMX) => {
                // info!("DMX Packet - DMX Mode");
                Some(data)
            },
            (DmxEvent::DmxPacket(_data), InputMode::ArtNet) => {
                // info!("DMX Packet - ArtNet Mode");
                None
            },
            (DmxEvent::ArtNetPacket(_data), InputMode::DMX) => {
                // info!("ArtNet Packet - DMX Mode");
                None
            },
            (DmxEvent::ArtNetPacket(data), InputMode::ArtNet) => {
                // info!("ArtNet Packet - ArtNet Mode");
                Some(data)
            },
        };


        if let Some(dmx_data) = event_data {
                let dmx_start = self.data.menu_settings.dmx_address as usize;

                match self.data.menu_settings.module {
                    ModuleSettings::Pwm(_pwm_settings) => {
                        error!("Module LED settings not programmed");
                        let _ = self.channel_pwm_i2c.try_send(PwmEvent::Value([dmx_data[1], dmx_data[2], dmx_data[3]]));
                    },
                    ModuleSettings::SmartLed(smart_led_settings) => {
                        let color_size = smart_led_settings.color_mode.addr_size();
                        let led_per_port = smart_led_settings.leds_per_port;
                        let dmx_group_size = smart_led_settings.dmx_group_size.0;

                        let virtual_leds_per_port = led_per_port.iter()
                            .zip(dmx_group_size.iter())
                            .map(|(led_count, grouping)| {
                                let number = *led_count as i16 / *grouping as i16;
                                let whole = (number as u16) as i16;
                                let round = if (number - whole) > 0 {1} else {0};
                                (whole + round) as usize
                            })
                            .collect::<Vec<usize, SMARTLED_PORT_COUNT>>();

                        // info!("color size {}", color_size);
                        // info!("leds {}", led_per_port);
                        // info!("color size {} ledperport {} dmx {} virt [{}, {}, {}, {}]", color_size, led_per_port, dmx_group_size, virtual_leds_per_port[0], virtual_leds_per_port[1], virtual_leds_per_port[2], virtual_leds_per_port[3]);
                       
                        let num_dmx_dimmers = match smart_led_settings.port_mode {
                            SmartLedPortMode::Individual => {
                                // LEDs are all individually addressed
                                virtual_leds_per_port.iter().map(|n| n*color_size).collect::<Vec<usize, SMARTLED_PORT_COUNT>>()
                            },
                            SmartLedPortMode::Mirror => {
                                // all LEDs on a port are addressed as one so we get the max number of leds on any of the ports
                                // virtual_leds_per_port.iter().max().map(|x| *x).unwrap_or(0) as usize * color_size
                                let m = virtual_leds_per_port.iter().max().map(|x| *x).unwrap_or(0) as usize * color_size;
                                let v: Vec<usize, SMARTLED_PORT_COUNT> = Vec::from_array([m, 0, 0, 0]);
                                v
                            },
                        };

                        let range_1_inc = (dmx_start, upper_limit(dmx_start + num_dmx_dimmers[0].saturating_sub(1), 512));
                        let range_2_inc = (range_1_inc.1+1, upper_limit(range_1_inc.1+1 + num_dmx_dimmers[1].saturating_sub(1), 512));
                        let range_3_inc = (range_2_inc.1+1, upper_limit(range_2_inc.1+1 + num_dmx_dimmers[2].saturating_sub(1), 512));
                        let range_4_inc = (range_3_inc.1+1, upper_limit(range_3_inc.1+1 + num_dmx_dimmers[3].saturating_sub(1), 512));

                        let ranges = match smart_led_settings.port_mode {
                            SmartLedPortMode::Individual => [range_1_inc, range_2_inc, range_3_inc, range_4_inc],
                            SmartLedPortMode::Mirror => [range_1_inc, range_1_inc, range_1_inc, range_1_inc], // reuse the same range for each port
                        };

                        let mut colors : [[RGB8; NUM_LEDS_MAX]; SMARTLED_PORT_COUNT] = [[RGB8::default(); NUM_LEDS_MAX], [RGB8::default(); NUM_LEDS_MAX], [RGB8::default(); NUM_LEDS_MAX], [RGB8::default(); NUM_LEDS_MAX]];

                        virtual_leds_per_port.iter().enumerate().for_each(|(port_index, _ledperport)| { // for each port
                            &dmx_data[ranges[port_index].0 ..= ranges[port_index].1] // get the dmx data for the ports range based on number of dmx dimmers calculated that the port uses
                                .chunks(smart_led_settings.color_mode.addr_size())
                                .enumerate()
                                .for_each(|(vled_index, vled_dmx_data)| { // iterate through each chunk of dmx data (which is split by virtual led)
                                    let c = smart_led_settings.color_mode.rgb(vled_dmx_data); // calculate a color from the dmx data
                                    for i in 0..dmx_group_size[port_index] { // iterate through each led in the led group
                                        colors[port_index][i as usize + dmx_group_size[port_index] as usize *vled_index] = c; // apply color to the physical led
                                    }
                                });
                        });

                        let _ = self.channel_smart_led.try_send(SmartLedEvent::Individual((led_per_port, colors)));

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