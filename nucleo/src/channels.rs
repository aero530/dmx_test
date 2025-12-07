//! Communication channels between tasks
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex};
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::watch::{Receiver as WatchReceiver, Sender as WatchSender, Watch};

use crate::event_router::{DmxEvent, DmxFeedbackEvent, GlobalData, MainEvent, RouterEvent};
use crate::pwm_i2c::PwmEvent;
use crate::smart_led::SmartLedEvent;
// use crate::artnet::ArtNetEvent;
use crate::eeprom::EepromEvent;
use crate::ui::UiEvent;

pub type RouterChannel = Channel<ThreadModeRawMutex, RouterEvent, 10>;
pub type RouterChannelRx = Receiver<'static, ThreadModeRawMutex, RouterEvent, 10>;
pub type RouterChannelTx = Sender<'static, ThreadModeRawMutex, RouterEvent, 10>;
pub static CHANNEL: RouterChannel = Channel::new();

pub type DmxChannel = Channel<ThreadModeRawMutex, DmxEvent, 1>;
pub type DmxChannelRx = Receiver<'static, ThreadModeRawMutex, DmxEvent, 1>;
pub type DmxChannelTx = Sender<'static, ThreadModeRawMutex, DmxEvent, 1>;
pub static CHANNEL_DMX: DmxChannel = Channel::new();

pub type DmxFeedbackChannel = Watch<ThreadModeRawMutex, DmxFeedbackEvent, 2>;
pub type DmxFeedbackChannelRx = WatchReceiver<'static, ThreadModeRawMutex, DmxFeedbackEvent, 2>;
pub type DmxFeedbackChannelTx = WatchSender<'static, ThreadModeRawMutex, DmxFeedbackEvent, 2>;
pub static CHANNEL_DMX_FEEDBACK: DmxFeedbackChannel = Watch::new();

pub type PwmChannel = Channel<ThreadModeRawMutex, PwmEvent, 1>;
#[allow(unused)]
pub type PwmChannelRx = Receiver<'static, ThreadModeRawMutex, PwmEvent, 1>;
pub type PwmChannelTx = Sender<'static, ThreadModeRawMutex, PwmEvent, 1>;
#[allow(unused)]
pub static CHANNEL_PWM: PwmChannel = Channel::new();
#[allow(unused)]
pub static CHANNEL_PWM_I2C: PwmChannel = Channel::new();

pub type SmartLedChannel = Channel<ThreadModeRawMutex, SmartLedEvent, 1>;
pub type SmartLedChannelRx = Receiver<'static, ThreadModeRawMutex, SmartLedEvent, 1>;
pub type SmartLedChannelTx = Sender<'static, ThreadModeRawMutex, SmartLedEvent, 1>;
pub static CHANNEL_SMART_LED: SmartLedChannel = Channel::new();

// pub type ArtNetChannel = Channel<ThreadModeRawMutex, ArtNetEvent, 1>;
// pub type ArtNetChannelRx = Receiver<'static, ThreadModeRawMutex, ArtNetEvent, 1>;
// pub type ArtNetChannelTx = Sender<'static, ThreadModeRawMutex, ArtNetEvent, 1>;
// pub static CHANNEL_ARTNET: ArtNetChannel = Channel::new();

pub type UiChannel = Channel<ThreadModeRawMutex, UiEvent, 1>;
pub type UiChannelRx = Receiver<'static, ThreadModeRawMutex, UiEvent, 1>;
pub type UiChannelTx = Sender<'static, ThreadModeRawMutex, UiEvent, 1>;
pub static CHANNEL_UI: UiChannel = Channel::new();

#[allow(unused)]
pub type UsbChannel = Channel<ThreadModeRawMutex, [u8; 64], 1>;
#[allow(unused)]
pub type UsbChannelRx = Receiver<'static, ThreadModeRawMutex, [u8; 64], 1>;
#[allow(unused)]
pub type UsbChannelTx = Sender<'static, ThreadModeRawMutex, [u8; 64], 1>;
#[allow(unused)]
pub static CHANNEL_USB: UsbChannel = Channel::new();

pub type GlobalDataChannel = Watch<CriticalSectionRawMutex, GlobalData, 2>;
// pub type GlobalDataChannelRx = WatchReceiver<'static, CriticalSectionRawMutex, GlobalData, 2>;
pub type GlobalDataChannelTx = WatchSender<'static, CriticalSectionRawMutex, GlobalData, 2>;
pub static CHANNEL_LOG: GlobalDataChannel = Watch::new();

pub type EepromChannel = Channel<ThreadModeRawMutex, EepromEvent, 1>;
pub type EepromChannelRx = Receiver<'static, ThreadModeRawMutex, EepromEvent, 1>;
pub type EepromChannelTx = Sender<'static, ThreadModeRawMutex, EepromEvent, 1>;
pub static CHANNEL_EEPROM: EepromChannel = Channel::new();

pub type MainChannel = Channel<ThreadModeRawMutex, MainEvent, 2>;
// pub type MainChannelRx = Receiver<'static, ThreadModeRawMutex, MainEvent, 2>;
pub type MainChannelTx = Sender<'static, ThreadModeRawMutex, MainEvent, 2>;
pub static CHANNEL_MAIN: MainChannel = Channel::new();
