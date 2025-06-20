//! USB <-> Serial / CDC Interface
use defmt::{panic, *};
use embassy_futures::join::join;
use embassy_stm32::usb::{Driver, Instance};
use embassy_stm32::peripherals;
use embassy_time::{with_timeout, Duration};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;


use static_cell::StaticCell;

use crate::channels::{RouterChannelTx, UsbChannelRx};
use crate::event_router::RouterEvent;

pub struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

pub async fn process_data<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
    rx: UsbChannelRx,
    router_tx: RouterChannelTx,
) -> Result<(), Disconnected> {
    let mut buf = [0; 64];
    loop {
        // Try to read input
        if let Ok(n) = with_timeout(Duration::from_millis(2), class.read_packet(&mut buf)).await {
            match n {
                Ok(byte_count) => {
                    let data = &buf[..byte_count];
                    let x = data[0];
                    #[allow(unused_assignments)]
                    match x {
                        0x31 => {
                            // 0x31 = ascii "1"
                            if !router_tx.is_full() {
                                router_tx
                                    .try_send(RouterEvent::UsbCommand(1))
                                    .map_err(|_| Disconnected {})?;
                            }
                        }
                        0x32 => {
                            // 0x32 = ascii "2"
                            if !router_tx.is_full() {
                                router_tx
                                    .try_send(RouterEvent::UsbCommand(2))
                                    .map_err(|_| Disconnected {})?;
                            }
                        }
                        0x33 => {
                            // 0x33 = ascii "3"
                            if !router_tx.is_full() {
                                router_tx
                                    .try_send(RouterEvent::UsbCommand(3))
                                    .map_err(|_| Disconnected {})?;
                            }
                        }
                        _ => {
                            // found nothing
                        }
                    }
                }
                Err(e) => error!("Endpoint error: {:?}", e),
            }
        }
        if let Ok(new_message) = with_timeout(Duration::from_millis(2), rx.receive()).await {
            let len = new_message
                .iter()
                .enumerate()
                .find(|&n| n.1 == &0)
                .map(|x| x.0)
                .unwrap_or(63);

            class.write_packet(&new_message[0..len]).await?;
        }
    }
}

#[embassy_executor::task(pool_size = 1)]
pub async fn usb_task(driver: Driver<'static, peripherals::USB_OTG_FS>, rx: UsbChannelRx, router_tx: RouterChannelTx) {

    // Create embassy-usb Config
    let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("us");
    config.product = Some("DMX POC");
    config.serial_number = Some("12345678");

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        let builder = embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        );
        builder
    };


    // Create classes on the builder.
    let mut class = {
        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init(State::new());
        CdcAcmClass::new(&mut builder, state, 64)
    };

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    // Do stuff with the class
    let process_fut = async move {
        loop {
            class.wait_connection().await;
            info!("USB Connected");
            let _ = process_data(&mut class, rx, router_tx).await;
            info!("USB Disconnected");
        }
    };

    join(usb_fut, process_fut).await;
}
