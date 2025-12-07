//! Event router to send commands between tasks
use core::net::Ipv4Addr;

use crate::artnet::PortAddress;
use crate::button::ButtonEvent;
use crate::button_array::{KeyPadButton, KeyPadEvent};
use crate::channels::*;
use crate::eeprom::EepromEvent;
use crate::pwm_i2c::PwmEvent;
use crate::smart_led::{SmartLedEvent, NUM_LEDS_MAX};
use crate::ui::{InputMode, IpAddrMenu, MenuData, ModuleSettings, ModuleType, SmartLedPortMode, UiEvent};
use crate::SMARTLED_PORT_COUNT;
use defmt::*;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{with_timeout, Duration};
use micromath::F32Ext;
use smart_leds::RGB8;

pub type LedBuffer = Mutex<ThreadModeRawMutex, [[RGB8; NUM_LEDS_MAX]; SMARTLED_PORT_COUNT]>;
// static LED_COLORS: LedBuffer = Mutex::new([[]]);
pub static LED_COLORS: LedBuffer = Mutex::new([[RGB8::new(0, 0, 0); NUM_LEDS_MAX]; SMARTLED_PORT_COUNT]);

type DmxBuffer = Mutex<ThreadModeRawMutex, [u8; 256 * 512]>; // 4 is the max number of colors per LED // type DmxBuffer = Mutex<ThreadModeRawMutex, [u8; SMARTLED_PORT_COUNT * NUM_LEDS_MAX * COLORS_PER_LED_MAX]>; // 4 is the max number of colors per LED

pub static DMX_BUFFER: DmxBuffer = Mutex::new([0_u8; 256 * 512]); // pub static DMX_BUFFER: DmxBuffer = Mutex::new([0_u8; SMARTLED_PORT_COUNT * NUM_LEDS_MAX * COLORS_PER_LED_MAX]);

/// Data stored for global use (primarily for logging / terminal display)
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct GlobalData {
    pub menu_settings: MenuData,
    pub module_type: Option<ModuleType>,
    pub mac: Option<[u8; 6]>,
    pub boot_complete: bool,

}

#[derive(Format)]
pub enum ReturnChannel {
    Main,
    #[allow(unused)]
    Ui,
}

/// Events the router watches for.  These trigger the router to pass along an event to another object.
#[derive(Format)]
pub enum RouterEvent {
    #[allow(unused)]
    UsbCommand(u8),
    ButtonArray((KeyPadButton, KeyPadEvent)),
    Button(ButtonEvent),
    WriteSettingsToEeprom(MenuData),

    StoreSettings(Option<MenuData>),
    StoreModuleType(Option<ModuleType>),
    StoreMacAddress(Option<[u8; 6]>),
    StoreBootComplete(bool),
    #[allow(unused)]
    StoreIpAddr(Option<Ipv4Addr>),

    GetModuleType(ReturnChannel),
    #[allow(unused)]
    GetMacAddress(ReturnChannel),
    GetSettings(ReturnChannel),
}

#[derive(Format)]
pub struct PacketAddress {
    port: PortAddress,
    sequence: u8,
}

impl PacketAddress {
    pub fn new(port: PortAddress, sequence: u8) -> Self {
        Self { port, sequence }
    }
}

#[derive(Format)]
pub enum DmxEvent {
    #[allow(unused)]
    DmxPacket(PacketAddress),
    #[allow(unused)]
    ArtNetPacket(PacketAddress),
}

#[derive(Copy, Clone, Debug, Format)]
pub enum DmxFeedbackEvent {
    Mode(InputMode),
}

#[derive(Format)]
#[allow(clippy::enum_variant_names)]
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
        channel_dmx_feedback: DmxFeedbackChannelTx,
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
            channel_dmx_feedback,
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
            RouterEvent::WriteSettingsToEeprom(menu_data) => {
                self.data.menu_settings = menu_data;
                info!("Store settings in eeprom {}", menu_data);
                let _ = self.channel_eeprom.try_send(EepromEvent::WriteSettings(menu_data));
            }
            RouterEvent::StoreSettings(menu_data) => {
                if let Some(x) = menu_data {
                    self.data.menu_settings = x;

                    // update internal data based on menu_data

                    info!("Update settings on display {}", x);
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
                        info!("get module type {}", self.data.module_type);
                        let _ = self.channel_main.try_send(MainEvent::ReturnModuleType(self.data.module_type));
                    }
                    ReturnChannel::Ui => {} //self.channel_ui.try_send(MainEvent::ReturnModuleType(self.data.module_type)),
                }
            }
            RouterEvent::GetMacAddress(ch) => {
                info!("Router event get mac {:#X}", self.data.mac);
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
    pub async fn update_leds(&mut self, event: DmxEvent) {
        if self.data.boot_complete {
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

            let mut colors = LED_COLORS.lock().await;

            match self.data.menu_settings.module {
                ModuleSettings::Pwm(_pwm_settings) => {
                    error!("Module LED settings not programmed");
                    let dmx_buffer = DMX_BUFFER.lock().await;
                    let _ = self.channel_pwm_i2c.try_send(PwmEvent::Value([dmx_buffer[1], dmx_buffer[2], dmx_buffer[3]]));
                }
                ModuleSettings::SmartLed(smart_led_settings) => {
                    let dmx_group_size = smart_led_settings.dmx_group_size.0;

                    let dmx_buffer = DMX_BUFFER.lock().await;

                    match smart_led_settings.port_mode {
                        SmartLedPortMode::Individual => {
                            // Offset index to account for many universes stored flat in dmx_buffer.  This value is always 0 for DMX but can vary for ArtNet data.
                            let mut port_u_offset = match self.data.menu_settings.input_mode {
                                InputMode::Dmx => 0, // DMX can only handle one universe
                                InputMode::ArtNet => self.data.menu_settings.artnet_address.0[2] as usize,
                            };

                            for (port_index, num_virtual_leds) in smart_led_settings.virtual_leds_per_port().iter().enumerate() {
                                // Calculate hoe many universes this port consumes. Each new port will start at a new universe...I think.
                                let port_universe_count = match self.data.menu_settings.input_mode {
                                    InputMode::Dmx => 0, // DMX can only handle one universe
                                    InputMode::ArtNet => (*num_virtual_leds as f32 * smart_led_settings.color_mode.addr_size() as f32 / 512.0).ceil() as usize,
                                };

                                for vled_index in 0..*num_virtual_leds as usize {
                                    let dmx_buffer_start = port_u_offset * 512 + self.data.menu_settings.dmx_address as usize + vled_index * smart_led_settings.color_mode.addr_size();
                                    let dmx_buffer_end = dmx_buffer_start + smart_led_settings.color_mode.addr_size() - 1;

                                    let c = smart_led_settings.color_mode.rgb(&dmx_buffer[dmx_buffer_start..=dmx_buffer_end]); // calculate a color from the dmx data

                                    // Loop through each group and assign the individual
                                    for i in 0..dmx_group_size[port_index] {
                                        let place = i as usize + dmx_group_size[port_index] as usize * vled_index;
                                        colors[port_index][place] = c;
                                        // apply color to the physical led
                                    }
                                }
                                port_u_offset += port_universe_count;
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
            router.process_event(new_message).await;
        }

        if let Ok(new_message) = with_timeout(Duration::from_millis(5), router.channel_dmx.receive()).await {
            router.update_leds(new_message).await;
        }
    }
}
