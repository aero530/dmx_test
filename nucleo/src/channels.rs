//! Communication channels between tasks.
//!
//! All inter-task messaging goes through the statics defined here. Most are
//! bounded `Channel`s (point-to-point queues); broadcast-style state uses
//! `Watch` (every receiver sees the latest value). The intended senders and
//! receivers are noted on each static.
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex};
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::watch::{Receiver as WatchReceiver, Sender as WatchSender, Watch};

use crate::event_router::{DmxEvent, DmxFeedbackEvent, GlobalData, MainEvent, RouterEvent};
use crate::pwm_i2c::PwmEvent;
use crate::smart_led::SmartLedEvent;
use crate::eeprom::EepromEvent;
use crate::ui::UiEvent;

pub type RouterChannel = Channel<ThreadModeRawMutex, RouterEvent, 10>;
pub type RouterChannelRx = Receiver<'static, ThreadModeRawMutex, RouterEvent, 10>;
pub type RouterChannelTx = Sender<'static, ThreadModeRawMutex, RouterEvent, 10>;
/// Everything -> event router: the main event bus (buttons, settings
/// writes, EEPROM results, console commands).
pub static CHANNEL: RouterChannel = Channel::new();

pub type DmxChannel = Channel<ThreadModeRawMutex, DmxEvent, 1>;
pub type DmxChannelRx = Receiver<'static, ThreadModeRawMutex, DmxEvent, 1>;
pub type DmxChannelTx = Sender<'static, ThreadModeRawMutex, DmxEvent, 1>;
/// Input tasks (dmx_i2c / artnet / usb_device) -> router: "new frame is in
/// `DMX_BUFFER`". Depth 1 on purpose: frames are latest-wins.
pub static CHANNEL_DMX: DmxChannel = Channel::new();

// 3 receivers: dmx_task (I2C bridge), artnet_task, usb_device_task
pub type DmxFeedbackChannel = Watch<ThreadModeRawMutex, DmxFeedbackEvent, 3>;
pub type DmxFeedbackChannelRx = WatchReceiver<'static, ThreadModeRawMutex, DmxFeedbackEvent, 3>;
pub type DmxFeedbackChannelTx = WatchSender<'static, ThreadModeRawMutex, DmxFeedbackEvent, 3>;
/// Router -> input tasks: broadcast of the current operating mode and
/// Art-Net universe whenever settings change.
pub static CHANNEL_DMX_FEEDBACK: DmxFeedbackChannel = Watch::new();

pub type PwmChannel = Channel<ThreadModeRawMutex, PwmEvent, 1>;
#[allow(unused)]
pub type PwmChannelRx = Receiver<'static, ThreadModeRawMutex, PwmEvent, 1>;
pub type PwmChannelTx = Sender<'static, ThreadModeRawMutex, PwmEvent, 1>;
#[allow(unused)]
pub static CHANNEL_PWM: PwmChannel = Channel::new();
/// Router -> PWM output module. NOTE: `pwm_i2c_task` is not currently
/// spawned (the PWM module is unfinished), so this fills after one event
/// (BUGS.md N19).
#[allow(unused)]
pub static CHANNEL_PWM_I2C: PwmChannel = Channel::new();

pub type SmartLedChannel = Channel<ThreadModeRawMutex, SmartLedEvent, 1>;
pub type SmartLedChannelRx = Receiver<'static, ThreadModeRawMutex, SmartLedEvent, 1>;
pub type SmartLedChannelTx = Sender<'static, ThreadModeRawMutex, SmartLedEvent, 1>;
/// Router -> smart_led_task: "`LED_COLORS` changed, push it to the strips".
pub static CHANNEL_SMART_LED: SmartLedChannel = Channel::new();

pub type UiChannel = Channel<ThreadModeRawMutex, UiEvent, 4>;
pub type UiChannelRx = Receiver<'static, ThreadModeRawMutex, UiEvent, 4>;
pub type UiChannelTx = Sender<'static, ThreadModeRawMutex, UiEvent, 4>;
/// Router -> UI task: button presses translated to Up/Down/Select/Esc,
/// plus `Load` events carrying refreshed settings. Depth 4 so a `Load`
/// arriving while the UI task is mid-draw (e.g. a DHCP address update)
/// is not silently dropped.
pub static CHANNEL_UI: UiChannel = Channel::new();

pub type GlobalDataChannel = Watch<CriticalSectionRawMutex, GlobalData, 2>;
pub type GlobalDataChannelRx = WatchReceiver<'static, CriticalSectionRawMutex, GlobalData, 2>;
/// Router -> observers: broadcast of the router's `GlobalData` (settings,
/// module type, MAC, boot status) after every store. Consumed by the USB
/// console task.
pub static CHANNEL_LOG: GlobalDataChannel = Watch::new();

pub type EepromChannel = Channel<ThreadModeRawMutex, EepromEvent, 1>;
pub type EepromChannelRx = Receiver<'static, ThreadModeRawMutex, EepromEvent, 1>;
pub type EepromChannelTx = Sender<'static, ThreadModeRawMutex, EepromEvent, 1>;
/// Router / main -> EEPROM task: read and write requests. Results come back
/// to the router as `RouterEvent::Store*` messages.
pub static CHANNEL_EEPROM: EepromChannel = Channel::new();

pub type MainChannel = Channel<ThreadModeRawMutex, MainEvent, 2>;
pub type MainChannelTx = Sender<'static, ThreadModeRawMutex, MainEvent, 2>;
/// Router -> main: replies to the `Get*` requests issued during the boot
/// sequence.
pub static CHANNEL_MAIN: MainChannel = Channel::new();
