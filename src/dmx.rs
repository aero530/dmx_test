//! DMX interaction
use defmt::info;
use embassy_stm32::exti::ExtiInput;
use embassy_time::Instant;

use embassy_stm32::usart::Uart;

use static_cell::StaticCell;

use crate::channels::RouterChannelTx;
use crate::event_router::RouterEvent;



/// Monitor dmx_break_pin interrupt
#[embassy_executor::task]
pub async fn dmx_task(mut usart: Uart<'static, embassy_stm32::mode::Async>, mut dmx_break_pin: ExtiInput<'static>, tx: RouterChannelTx) {
    
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
        if (break_time > BREAK_DELAY) & (break_time < BREAK_TIMEOUT) {
            info!("DMX BREAK detected");
        } else {
            break
        }

        dmx_break_pin.wait_for_falling_edge().await;
        let mab_fall = Instant::now();

        let mab_time = (mab_fall - rise).as_micros();
        if (mab_time > MAB_DELAY) & (mab_time < BREAK_TIMEOUT) {
            info!("DMX MAB detected");
        } else {
            break
        }

        if usart.read(dmx_buffer).await.is_ok() {
            if dmx_buffer[0] == 0x00 {
                info!("DMX sending packet to router");
                let _ = tx.try_send(RouterEvent::DmxPacket(
                    *dmx_buffer
                ));
            } else {
                info!("DMX packet start byte was not 0x00");
            }
        } else {
            info!("DMX error reading data");
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
