//! USB <-> Serial Logger
#![allow(dead_code)]
use crate::ansi::{cyan, magenta, yellow, Color, Colorable, WithForeground};
use crate::channels::{GlobalDataChannelRx, RouterChannelTx, UsbChannelTx};

use embassy_time::{Instant, Timer};


#[embassy_executor::task(pool_size = 1)]
pub async fn log_task(
    router_tx: RouterChannelTx,
    mut rx_data: GlobalDataChannelRx,
    usb_tx: UsbChannelTx,
) {
    let mut buf = [0u8; 64];

    loop {
        let now = Instant::now().as_millis();
        match rx_data.try_changed() {
            Some(data) => {
                hide_cursor(usb_tx, &mut buf).await;

                home(usb_tx, &mut buf).await;

                buf.fill(0_u8);
                let _ =
                    format_no_std::show(&mut buf, format_args!("{}sec ", now.fg(cyan()))).unwrap();
                usb_tx.send(buf).await;

                buf.fill(0_u8);
                let _ = format_no_std::show(&mut buf, format_args!("Button{}", data.button)).unwrap();
                usb_tx.send(buf).await;
                new_line(usb_tx, &mut buf).await;

                show_cursor(usb_tx, &mut buf).await;
            }
            None => {
                // do nothing if there is no data
                // let _ = format_no_std::show(
                //     &mut buf,
                //     format_args!("{}: Unable to get the data\n", now),
                // )
                // .unwrap();
                // let _ = usb_tx.try_send(buf);
            }
        }

        if !router_tx.is_full() {
            // let _ = router_tx.try_send(RouterEvent::UpdateData);
        }

        Timer::after_millis(100).await;
    }
}


/// Formatted display for a bool as colored "high" and "low"
fn high_low_color(input: bool) -> WithForeground<&'static str, Color> {
    if input {
        high_low(input).fg(magenta())
    } else {
        high_low(input).fg(yellow())
    }
}

/// Formatted display for a bool as "high" and "low"
pub fn high_low(input: bool) -> &'static str {
    if input {
        "High"
    } else {
        "Low"
    }
}

/// ANSI character sequence to create new line
async fn new_line(usb_tx: UsbChannelTx, buf: &mut [u8; 64]) {
    buf.fill(0_u8);
    // \u{1b}[0K clears remaining line
    // \r\n does a new line carriage return
    let _ = format_no_std::show(buf, format_args!("\u{1b}[0K\r\n")).unwrap();
    usb_tx.send(*buf).await;
}

/// ANSI character sequence to return to home position
async fn home(usb_tx: UsbChannelTx, buf: &mut [u8; 64]) {
    buf.fill(0_u8);
    let _ = format_no_std::show(buf, format_args!("\u{1b}[H")).unwrap();
    usb_tx.send(*buf).await;
}

/// ANSI character sequence to hide cursor
async fn hide_cursor(usb_tx: UsbChannelTx, buf: &mut [u8; 64]) {
    buf.fill(0_u8);
    let _ = format_no_std::show(buf, format_args!("\u{1b}[?25l")).unwrap();
    usb_tx.send(*buf).await;
}

/// ANSI character sequence to show cursor
async fn show_cursor(usb_tx: UsbChannelTx, buf: &mut [u8; 64]) {
    buf.fill(0_u8);
    let _ = format_no_std::show(buf, format_args!("\u{1b}[?25h")).unwrap();
    usb_tx.send(*buf).await;
}
