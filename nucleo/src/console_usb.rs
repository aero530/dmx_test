//! USB console: a small line protocol for host tooling (see `dmx_console/`).
//!
//! Runs on a dedicated CDC-ACM interface of the composite USB device. Every
//! command is one line, every response ends with `ok` or `err <reason>`:
//!
//! ```text
//! get                  -> key=value per settings field, then ok
//! set <key> <value>    -> apply one setting (persisted to EEPROM), ok/err
//! dmx <start> <count>  -> "dmx <ch> v v v ..." lines (16 per line), then ok
//! info                 -> mode/module/ip/boot summary, then ok
//! help                 -> command list
//! ```
//!
//! Settings keys and value parsing come from the same [`FieldId`] metadata
//! table that drives the on-device TFT menu, so the console automatically
//! stays in sync with the UI.
use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::info;
    } else {
        use defmt::info;
    }
}

use alloc::format;

use embassy_stm32::peripherals;
use embassy_stm32::usb::Driver;
use embassy_usb::class::cdc_acm::CdcAcmClass;
use embassy_usb::driver::EndpointError;

use crate::channels::{GlobalDataChannelRx, RouterChannelTx};
use crate::event_router::{GlobalData, RouterEvent};
use crate::ui::all_fields;
use crate::{DMX_BUFFER, DMX_UNIVERSE_SIZE};

type UsbClass<'d> = CdcAcmClass<'d, Driver<'d, peripherals::USB>>;

/// Serve the console protocol forever (reconnecting across USB sessions).
pub async fn run(class: &mut UsbClass<'_>, router_tx: RouterChannelTx, mut global_rx: GlobalDataChannelRx) -> ! {
    let mut line = [0_u8; 96];
    let mut packet = [0_u8; 64];

    loop {
        class.wait_connection().await;
        info!("Console: USB host connected");
        let mut len = 0_usize;

        'connected: loop {
            match class.read_packet(&mut packet).await {
                Err(_) => break 'connected,
                Ok(n) => {
                    for &byte in &packet[..n] {
                        if byte == b'\n' || byte == b'\r' {
                            if len > 0 {
                                let done = {
                                    let text = core::str::from_utf8(&line[..len]).unwrap_or("");
                                    handle_line(class, text, &router_tx, &mut global_rx).await
                                };
                                len = 0;
                                if done.is_err() {
                                    break 'connected;
                                }
                            }
                        } else if len < line.len() {
                            line[len] = byte;
                            len += 1;
                        } else {
                            // Overlong line: discard it entirely
                            len = 0;
                        }
                    }
                }
            }
        }
        info!("Console: USB host disconnected");
    }
}

/// Send a string, chunked into full-speed USB packets.
async fn respond(class: &mut UsbClass<'_>, text: &str) -> Result<(), EndpointError> {
    for chunk in text.as_bytes().chunks(64) {
        class.write_packet(chunk).await?;
    }
    if text.len().is_multiple_of(64) {
        class.write_packet(&[]).await?; // flush a packet-aligned transfer
    }
    Ok(())
}

fn current(global_rx: &mut GlobalDataChannelRx) -> GlobalData {
    global_rx.try_get().unwrap_or_default()
}

async fn handle_line(
    class: &mut UsbClass<'_>,
    line: &str,
    router_tx: &RouterChannelTx,
    global_rx: &mut GlobalDataChannelRx,
) -> Result<(), EndpointError> {
    let mut parts = line.split_whitespace();
    let command = parts.next().unwrap_or("");

    match command {
        "get" => {
            let data = current(global_rx);
            for field in all_fields() {
                respond(class, &format!("{}={}\n", field.key(), field.display(&data.menu_settings))).await?;
            }
            respond(class, "ok\n").await?;
        }

        "set" => {
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");
            let result: Result<(), &str> = if key.is_empty() || value.is_empty() {
                Err("usage: set <key> <value>")
            } else {
                match all_fields().find(|f| f.key() == key) {
                    None => Err("unknown key"),
                    Some(field) => {
                        // Validate/parse onto a scratch copy, then have the
                        // router merge only this field into its authoritative
                        // settings (same semantics as a TFT commit), so a
                        // console `set` can't clobber a concurrent edit.
                        let mut settings = current(global_rx).menu_settings;
                        field.set_from_str(&mut settings, value).map(|()| {
                            let _ = router_tx.try_send(RouterEvent::WriteFieldToEeprom(field, settings));
                        })
                    }
                }
            };
            match result {
                Ok(()) => respond(class, "ok\n").await?,
                Err(e) => respond(class, &format!("err {}\n", e)).await?,
            }
        }

        "dmx" => {
            // Channels are read DMX-style: buffer index == channel number
            // (slot 0 is the start code).
            let start: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
            let count: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(32);
            let start = start.clamp(1, DMX_UNIVERSE_SIZE);
            let count = count.min(DMX_UNIVERSE_SIZE + 1 - start);

            let mut values = [0_u8; DMX_UNIVERSE_SIZE];
            {
                let buffer = DMX_BUFFER.lock().await;
                values[..count].copy_from_slice(&buffer[start..start + count]);
            }

            for (i, chunk) in values[..count].chunks(16).enumerate() {
                let mut out = format!("dmx {}", start + i * 16);
                for v in chunk {
                    out.push_str(&format!(" {}", v));
                }
                out.push('\n');
                respond(class, &out).await?;
            }
            respond(class, "ok\n").await?;
        }

        "info" => {
            let data = current(global_rx);
            let ip = data.menu_settings.ip_addr.octets();
            respond(class, &format!("mode={}\n", data.menu_settings.input_mode)).await?;
            respond(class, &format!("module={:?}\n", data.module_type)).await?;
            respond(class, &format!("ip={}.{}.{}.{}\n", ip[0], ip[1], ip[2], ip[3])).await?;
            respond(class, &format!("boot={:?}\n", data.boot_status)).await?;
            respond(class, "ok\n").await?;
        }

        "help" => {
            respond(class, "get | set <key> <value> | dmx <start> <count> | info\n").await?;
            respond(class, "ok\n").await?;
        }

        "" => {}

        _ => respond(class, "err unknown command (try help)\n").await?,
    }

    Ok(())
}
