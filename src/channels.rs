//! Communication channels between tasks
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex};
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::watch::{Receiver as WatchReceiver, Sender as WatchSender, Watch};

use crate::event_router::{GlobalData, RouterEvent};
use crate::led::LedEvent;


pub type RouterChannel = Channel<ThreadModeRawMutex, RouterEvent, 10>;
pub type RouterChannelRx = Receiver<'static, ThreadModeRawMutex, RouterEvent, 10>;
pub type RouterChannelTx = Sender<'static, ThreadModeRawMutex, RouterEvent, 10>;
pub static CHANNEL: RouterChannel = Channel::new();

pub type LedChannel = Channel<ThreadModeRawMutex, LedEvent, 1>;
pub type LedChannelRx = Receiver<'static, ThreadModeRawMutex, LedEvent, 1>;
pub type LedChannelTx = Sender<'static, ThreadModeRawMutex, LedEvent, 1>;
pub static CHANNEL_LED: LedChannel = Channel::new();

pub type UsbChannel = Channel<ThreadModeRawMutex, [u8; 64], 1>;
pub type UsbChannelRx = Receiver<'static, ThreadModeRawMutex, [u8; 64], 1>;
pub type UsbChannelTx = Sender<'static, ThreadModeRawMutex, [u8; 64], 1>;
pub static CHANNEL_USB: UsbChannel = Channel::new();

pub type GlobalDataChannel = Watch<CriticalSectionRawMutex, GlobalData, 2>;
pub type GlobalDataChannelRx = WatchReceiver<'static, CriticalSectionRawMutex, GlobalData, 2>;
pub type GlobalDataChannelTx = WatchSender<'static, CriticalSectionRawMutex, GlobalData, 2>;
pub static CHANNEL_LOG: GlobalDataChannel = Watch::new();
