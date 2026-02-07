//! Event router to send commands between tasks

use crate::artnet::PortAddress;
use crate::button::ButtonEvent;
use crate::button_array::{KeyPadButton, KeyPadEvent};
use crate::channels::*;
use crate::eeprom::EepromEvent;
use crate::pwm_i2c::PwmEvent;
use crate::smart_led::SmartLedEvent;
use crate::ui::{InputMode, IpAddrMenu, MenuData, ModuleSettings, ModuleType, SmartLedPortMode, UiEvent};
use crate::{DMX_BUFFER, DMX_UNIVERSE_SIZE, LED_COLORS};
use core::net::Ipv4Addr;
use defmt::Format;

use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{error, info, warn};
    } else {
        use defmt::{error, info, warn};
    }
}



use embassy_time::{with_timeout, Duration};
use micromath::F32Ext;

/// Data stored for global use (primarily for logging / terminal display)
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct GlobalData {
    /// Current menu settings
    pub menu_settings: MenuData,
    /// Detected module type
    pub module_type: Option<ModuleType>,
    /// Board MAC address - only defined if ETH is enabled
    pub mac: Option<[u8; 6]>,
    /// Monitor boot process
    pub boot_complete: bool,
}

/// Indicate which channel should be used to return data
#[derive(Format, Debug)]
pub enum ReturnChannel {
    Main,
    #[allow(unused)]
    Ui,
}

/// Events the router watches for.  These trigger the router to pass along an event to another object.
#[derive(Format, Debug)]
pub enum RouterEvent {
    #[allow(unused)]
    UsbCommand(u8),

    ButtonArray((KeyPadButton, KeyPadEvent)),
    Button(ButtonEvent),
    WriteSettingsToEeprom(MenuData),
    /// Store settings in global data
    StoreSettings(Option<MenuData>),
    /// Store module type in global data
    StoreModuleType(Option<ModuleType>),
    /// Store MAC address in global data
    StoreMacAddress(Option<[u8; 6]>),
    /// Store boot complete in global data
    StoreBootComplete(bool),
    /// Store IP address in global data
    #[allow(unused)]
    StoreIpAddr(Option<Ipv4Addr>),
    /// Get module type from global data
    GetModuleType(ReturnChannel),
    /// Get MAC address from global data
    #[allow(unused)]
    GetMacAddress(ReturnChannel),
    /// Get settings from global data
    GetSettings(ReturnChannel),
}

/// Data packet address and sequence info either passed through from ArtNet or set to default for DMX packets
#[derive(Format)]
pub struct PacketAddress {
    /// packet address info either passed through from ArtNet or set to default for DMX packets
    port: PortAddress,
    /// ArtNet packet sequence address - can be used to ensure packet order
    sequence: u8,
}

impl PacketAddress {
    pub fn new(port: PortAddress, sequence: u8) -> Self {
        Self { port, sequence }
    }
}

/// Send data from DMX or ArtNet task to event router
#[derive(Format)]
pub enum DmxEvent {
    /// New data came from wired DMX
    #[allow(unused)]
    DmxPacket(PacketAddress),
    /// New data came from ethernet / ArtNet
    #[allow(unused)]
    ArtNetPacket(PacketAddress),
}

/// Send data back to DMX task or ArtNet task
#[derive(Copy, Clone, Debug, Format)]
pub enum DmxFeedbackEvent {
    Mode(InputMode),
}

/// Send data back to the main task
#[derive(Format)]
#[allow(clippy::enum_variant_names)]
pub enum MainEvent {
    ReturnModuleType(Option<ModuleType>),
    ReturnSettings(MenuData),
    ReturnMacAddress(Option<[u8; 6]>),
}

/// Main communication interface between application tasks
pub struct Router {
    /// Listen for event router tasks
    pub channel: RouterChannelRx,
    pub channel_dmx: DmxChannelRx,
    pub channel_dmx_feedback: DmxFeedbackChannelTx,

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

    // Global data store
    pub data: GlobalData,
}

impl Router {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel: RouterChannelRx,
        channel_dmx: DmxChannelRx,
        channel_dmx_feedback: DmxFeedbackChannelTx,
        // channel_led: LedChannelTx,
        // channel_pwm: PwmChannelTx,
        channel_pwm_i2c: PwmChannelTx,
        channel_smart_led: SmartLedChannelTx,
        channel_ui: UiChannelTx,
        channel_eeprom: EepromChannelTx,
        channel_main: MainChannelTx,
    ) -> Self {
        Self {
            channel,
            channel_dmx,
            channel_dmx_feedback,
            // channel_led,
            // channel_pwm,
            channel_pwm_i2c,
            channel_smart_led,
            channel_ui,
            channel_eeprom,
            channel_main,
            data: GlobalData::default(),
        }
    }

    /// Main event router
    pub async fn process_router_event(&mut self, event: RouterEvent) {
        match event {
            RouterEvent::UsbCommand(input) => {
                info!("USB command {:#?}", input);
            }
            RouterEvent::Button(evt) => {
                info!("Button event {:#?}", evt);
            }
            RouterEvent::ButtonArray((btn, evt)) => {
                info!("Button array event {:#?} {:#?}", btn, evt);
                match btn {
                    KeyPadButton::D => {
                        let _ = self.channel_ui.try_send(UiEvent::Down);
                    }
                    KeyPadButton::N0 => {
                        let _ = self.channel_ui.try_send(UiEvent::Select);
                    }
                    KeyPadButton::Star => {
                        let _ = self.channel_ui.try_send(UiEvent::Esc);
                    }
                    KeyPadButton::Pound => {
                        let _ = self.channel_ui.try_send(UiEvent::Up);
                    }
                    _ => {}
                }
            }
            RouterEvent::WriteSettingsToEeprom(menu_data) => {
                self.data.menu_settings = menu_data;
                info!("Store settings in eeprom {:#?}", menu_data);
                let _ = self.channel_eeprom.try_send(EepromEvent::WriteSettings(menu_data));
            }
            RouterEvent::StoreSettings(menu_data) => {
                if let Some(x) = menu_data {
                    self.data.menu_settings = x;

                    // update internal data based on menu_data

                    info!("Update settings on display {:#?}", x);
                    let _ = self.channel_ui.try_send(UiEvent::Load(x));
                    self.channel_dmx_feedback.send(DmxFeedbackEvent::Mode(x.input_mode));
                }
            }
            RouterEvent::StoreModuleType(module_type) => {
                self.data.module_type = module_type;
            }
            RouterEvent::StoreMacAddress(mac) => {
                self.data.mac = mac;
            }
            RouterEvent::StoreBootComplete(complete) => {
                self.data.boot_complete = complete;
            }
            RouterEvent::StoreIpAddr(data) => {
                if let Some(addr) = data {
                    self.data.menu_settings.ip_addr = addr.into();
                } else {
                    self.data.menu_settings.ip_addr = IpAddrMenu::default();
                }
                let _ = self.channel_ui.try_send(UiEvent::Load(self.data.menu_settings));
            }
            RouterEvent::GetModuleType(ch) => {
                match ch {
                    ReturnChannel::Main => {
                        info!("get module type {:#?}", self.data.module_type);
                        let _ = self.channel_main.try_send(MainEvent::ReturnModuleType(self.data.module_type));
                    }
                    ReturnChannel::Ui => {} //self.channel_ui.try_send(MainEvent::ReturnModuleType(self.data.module_type)),
                }
            }
            RouterEvent::GetMacAddress(ch) => {
                match self.data.mac {
                    Some(mac) => info!("Router event get MAC {:#?}", mac),
                    None => info!("Router event did not get MAC."),
                }
                // info!("Router event get mac {:#X}", self.data.mac);
                match ch {
                    ReturnChannel::Main => {
                        let _ = self.channel_main.try_send(MainEvent::ReturnMacAddress(self.data.mac));
                    }
                    ReturnChannel::Ui => {} //self.channel_ui.try_send(MainEvent::ReturnMacAddress(self.data.mac)),
                }
            }
            RouterEvent::GetSettings(ch) => match ch {
                ReturnChannel::Main => {
                    let _ = self.channel_main.try_send(MainEvent::ReturnSettings(self.data.menu_settings));
                }
                ReturnChannel::Ui => {
                    let _ = self.channel_ui.try_send(UiEvent::Load(self.data.menu_settings));
                }
            },
        }
    }

    /// Update LED color in memory and apply to physical LEDs
    pub async fn process_dmx_event(&mut self, event: DmxEvent) {
        if self.data.boot_complete {
            // Check to make sure the incoming ArtNet packet address info matches current settings (ie make sure this packet was for us)
            let _packet_addr = match event {
                DmxEvent::DmxPacket(d) => d,
                DmxEvent::ArtNetPacket(packet_addr) => {
                    if packet_addr.port.net != self.data.menu_settings.artnet_address.0[0] {
                        warn!("ArtNet Net does not match {} {}", packet_addr.port.net, self.data.menu_settings.artnet_address.0[0]);
                        return;
                    }
                    if packet_addr.port.sub_net != self.data.menu_settings.artnet_address.0[1] {
                        warn!("ArtNet SubNet does not match {} {}", packet_addr.port.sub_net, self.data.menu_settings.artnet_address.0[1]);
                        return;
                    }
                    packet_addr
                }
            };

            // Lock global led color buffer
            let mut colors = LED_COLORS.lock().await;

            match self.data.menu_settings.module {
                ModuleSettings::Pwm(_pwm_settings) => {
                    error!("Module LED settings not programmed");
                    let dmx_buffer = DMX_BUFFER.lock().await;
                    let _ = self.channel_pwm_i2c.try_send(PwmEvent::Value([dmx_buffer[1], dmx_buffer[2], dmx_buffer[3]]));
                }
                ModuleSettings::SmartLed(smart_led_settings) => {
                    let dmx_group_size = smart_led_settings.dmx_group_size.0;
                    // Lock global dmx data buffer
                    let dmx_buffer = DMX_BUFFER.lock().await;

                    match smart_led_settings.port_mode {
                        SmartLedPortMode::Individual => {
                            // Offset index to account for many universes stored flat in dmx_buffer.  This value is always 0 for DMX but can vary for ArtNet data.
                            // let mut port_u_offset = match self.data.menu_settings.input_mode {
                            //     InputMode::Dmx => 0, // DMX can only handle one universe
                            //     InputMode::ArtNet => self.data.menu_settings.artnet_address.0[2] as usize,
                            // };

                            // Byte offset to place virtual leds at the right buffer location for each port
                            let mut port_virtual_led_offset = match self.data.menu_settings.input_mode {
                                InputMode::Dmx => 0,
                                InputMode::ArtNet => (self.data.menu_settings.artnet_address.0[2] as usize) * DMX_UNIVERSE_SIZE,
                            };

                            for (port_index, num_virtual_leds) in smart_led_settings.virtual_leds_per_port().iter().enumerate() {
                                // Calculate hoe many universes this port consumes. Each new port will start at a new universe...I think.
                                // let port_universe_count = match self.data.menu_settings.input_mode {
                                //     InputMode::Dmx => 0, // DMX can only handle one universe
                                //     InputMode::ArtNet => (*num_virtual_leds as f32 * smart_led_settings.color_mode.addr_size() as f32 / DMX_UNIVERSE_SIZE as f32).ceil() as usize,
                                // };
                                //
                                // let dmx_addr_offset = match self.data.menu_settings.input_mode {
                                //     InputMode::Dmx => 0, // DMX can only handle one universe
                                //     InputMode::ArtNet => port_u_offset * DMX_UNIVERSE_SIZE,
                                // };

                                // Copy data from DMX_BUFFER to LED_COLORS
                                for vled_index in 0..*num_virtual_leds as usize {
                                    let mut dmx_buffer_start = self.data.menu_settings.dmx_address as usize + port_virtual_led_offset + vled_index * smart_led_settings.color_mode.addr_size();
                                    let mut dmx_buffer_end = dmx_buffer_start + smart_led_settings.color_mode.addr_size() - 1;

                                    // Catch out of bounds errors where the calculated start or end are outside the bounds of dmx_buffer
                                    if dmx_buffer_end >= dmx_buffer.len() {
                                        warn!("DMX buffer end {} > buffer length {}.", dmx_buffer_end, dmx_buffer.len());
                                        dmx_buffer_end = dmx_buffer.len() - 1;
                                    }

                                    if dmx_buffer_start >= dmx_buffer.len() {
                                        warn!("DMX buffer start {} > buffer length {}.", dmx_buffer_start, dmx_buffer.len());
                                        dmx_buffer_start = dmx_buffer.len() - 1;
                                    }

                                    if dmx_buffer_start > dmx_buffer_end {
                                        warn!("DMX buffer start {} > buffer end {}.", dmx_buffer_start, dmx_buffer_end);
                                        dmx_buffer_start = dmx_buffer_end;
                                    }

                                    let c = smart_led_settings.color_mode.rgb(&dmx_buffer[dmx_buffer_start..=dmx_buffer_end]); // calculate a color from the dmx data

                                    // Loop through each group and assign to individual LEDs
                                    for i in 0..dmx_group_size[port_index] {
                                        let place = i as usize + dmx_group_size[port_index] as usize * vled_index;
                                        colors[port_index][place] = c;
                                    }
                                }
                                // port_u_offset += port_universe_count;
                                port_virtual_led_offset += match self.data.menu_settings.input_mode {
                                    InputMode::Dmx => *num_virtual_leds as usize * smart_led_settings.color_mode.addr_size(), // offset by the number of virtual LEDs in the current port
                                    InputMode::ArtNet => (*num_virtual_leds as f32 * smart_led_settings.color_mode.addr_size() as f32 / DMX_UNIVERSE_SIZE as f32).ceil() as usize * DMX_UNIVERSE_SIZE // Offset by the number of universes this port uses times the dmx size per universe
                                };
                            }
                        }
                        SmartLedPortMode::Mirror => {
                            for vled_index in 0..smart_led_settings.virtual_leds_per_port()[0] as usize {
                                let dmx_buffer_start = self.data.menu_settings.dmx_address as usize + vled_index * smart_led_settings.color_mode.addr_size();
                                let dmx_buffer_end = dmx_buffer_start + smart_led_settings.color_mode.addr_size() - 1;
                                let c = smart_led_settings.color_mode.rgb(&dmx_buffer[dmx_buffer_start..=dmx_buffer_end]); // calculate a color from the dmx data

                                for i in 0..dmx_group_size[0] {
                                    let place = i as usize + dmx_group_size[0] as usize * vled_index;
                                    for (port_index, _) in smart_led_settings.virtual_leds_per_port().iter().enumerate() {
                                        colors[port_index][place] = c;
                                    }
                                }
                            }
                        }
                    }

                    let _ = self.channel_smart_led.try_send(SmartLedEvent::UpdateLEDs);
                }
            }
        }
    }
}

#[embassy_executor::task]
pub async fn event_router(mut router: Router) {
    loop {
        if let Ok(new_message) = with_timeout(Duration::from_millis(5), router.channel.receive()).await {
            router.process_router_event(new_message).await;
        }

        if let Ok(new_message) = with_timeout(Duration::from_millis(5), router.channel_dmx.receive()).await {
            router.process_dmx_event(new_message).await;
        }
    }
}
