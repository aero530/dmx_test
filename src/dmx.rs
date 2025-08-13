//! DMX interaction
use defmt::{error, info};
use embassy_stm32::exti::ExtiInput;
use embassy_time::Instant;

use embassy_stm32::usart::Uart;

use static_cell::StaticCell;

use crate::channels::DmxChannelTx;
use crate::event_router::DmxEvent;


/// Monitor dmx_break_pin interrupt
#[embassy_executor::task]
pub async fn dmx_task(mut usart: Uart<'static, embassy_stm32::mode::Async>, mut dmx_break_pin: ExtiInput<'static>, tx: DmxChannelTx) {

    const MAB_DELAY: u64 = 8;
    const BREAK_DELAY: u64 = 88;
    const BREAK_TIMEOUT: u64 = 1000000;

    static DMX_BUFFER: StaticCell<[u8; 513]> = StaticCell::new();
    let dmx_buffer = DMX_BUFFER.init([0_u8; 513]);

    loop {
        dmx_break_pin.wait_for_falling_edge().await;
        let break_fall = Instant::now();
        dmx_break_pin.wait_for_rising_edge().await;
        let rise = Instant::now();

        let break_time = (rise - break_fall).as_micros();
        if (break_time >= BREAK_DELAY) & (break_time < BREAK_TIMEOUT) {
            // info!("DMX BREAK detected");
        } else {
            // info!("DMX break timeout {}", break_time);
            continue
        }

        dmx_break_pin.wait_for_falling_edge().await;
        let mab_fall = Instant::now();


        let mab_time = (mab_fall - rise).as_micros();
        if (mab_time >= MAB_DELAY) & (mab_time < BREAK_TIMEOUT) {
            // info!("DMX MAB detected");
        } else {
            info!("DMX MAB timeout {}", mab_time);
            continue
        }

        if usart.read(dmx_buffer).await.is_ok() {
            if dmx_buffer[0] == 0x00 {
                // info!("DMX sending packet to router");
                if tx.is_full() {
                    tx.clear(); // clear any existing message on the channel
                }
                match tx.try_send(DmxEvent::DmxPacket(*dmx_buffer)) { // since we just cleared the buffer this should complete immediately
                        Ok(()) => {},
                        Err(e) => error!("DMX channel error {}", e)
                }
            } else {
                info!("DMX packet start byte was not 0x00");
            }
        } else {
            info!("DMX error reading data break: {}, mab: {}", break_time, mab_time);
        }
    }
}

// #[embassy_executor::task]
// pub async fn led_task(pin: Output<'static>, rx: LedChannelRx) {
//     let mut led = Led::new(pin, rx);
//     loop {
//         led.show().await;
//     }
// }
