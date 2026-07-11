//! DMX bridge (RP2040) interface
//!
//! Master side of the I2C protocol with the RP2040 DMX bridge
//! (see `rp2040_dmx/src/main.rs` for the slave side):
//!
//! * In DMX input mode the received universe is polled with the block-read
//!   commands 0x01/0x02/0x03 and copied into `DMX_BUFFER`.
//! * In ArtNet>DMX and USB>DMX modes the bridge's port direction is switched
//!   to output (command 0x20) and the outgoing frame is pushed with the
//!   block-write commands 0x11/0x12/0x13 every 30ms. The bridge itself
//!   retransmits the latest frame continuously, so this only needs to keep
//!   it fresh.
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{error, info};
    } else {
        use defmt::{error, info};
    }
}

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

use crate::artnet::PortAddress;
use crate::channels::{DmxChannelTx, DmxFeedbackChannelRx};
use crate::event_router::{DmxEvent, DmxFeedbackEvent, PacketAddress};
use crate::ui::InputMode;
use crate::I2c1Bus;
use crate::{DMX_BUFFER, DMX_BUFF_SIZE, DMX_UNIVERSE_SIZE};

// Block-read commands: the 513-byte received frame in three chunks.
const CMD_READ_BLOCK_1: u8 = 0x01; // slots [0, 200)
const CMD_READ_BLOCK_2: u8 = 0x02; // slots [200, 400)
const CMD_READ_BLOCK_3: u8 = 0x03; // slots [400, 513)

// Block-write commands: the 513-byte output frame in the same chunk layout.
const CMD_WRITE_BLOCK_1: u8 = 0x11;
const CMD_WRITE_BLOCK_2: u8 = 0x12;
const CMD_WRITE_BLOCK_3: u8 = 0x13;

/// Port direction command: 0x20 + mode byte.
const CMD_SET_DIRECTION: u8 = 0x20;
const DIRECTION_INPUT: u8 = 0x00;
const DIRECTION_OUTPUT: u8 = 0x01;

/// Re-send the direction every N loop iterations (~1s at 30ms) while
/// transmitting, so a bridge that rebooted (and defaulted back to input)
/// recovers without user action.
const DIRECTION_REFRESH_LOOPS: u32 = 32;

/// Exchange DMX data with the RP2040 bridge via I2C.
///
/// Direction depends on the operating mode selected in the main menu:
/// polls received DMX in DMX mode, pushes outgoing DMX in the `*>DMX` modes,
/// and idles in plain ArtNet mode.
#[embassy_executor::task]
pub async fn dmx_task(i2c_bus_manager: &'static I2c1Bus, address: u8, tx: DmxChannelTx, mut rx: DmxFeedbackChannelRx) {
    let mut i2c_bus_dev = I2cDevice::new(i2c_bus_manager);

    let mut data_buffer = [0_u8; DMX_BUFF_SIZE];
    let mut input_mode = InputMode::default();
    let mut artnet_universe: u8 = 0;
    // Direction last acknowledged by the bridge; None forces a (re)send
    let mut bridge_direction: Option<u8> = None;
    let mut refresh_countdown: u32 = 0;

    loop {
        // Try to update current mode
        if let Some(input_data) = rx.try_changed() {
            match input_data {
                DmxFeedbackEvent::Mode(new_mode, universe) => {
                    if new_mode != input_mode {
                        info!("DMX bridge mode change {:?} -> {:?}", input_mode, new_mode);
                    }
                    input_mode = new_mode;
                    artnet_universe = universe;
                }
            }
        }

        // Keep the bridge's port direction in line with the operating mode
        let desired_direction = if input_mode.is_dmx_output() { DIRECTION_OUTPUT } else { DIRECTION_INPUT };
        refresh_countdown = refresh_countdown.saturating_sub(1);
        if bridge_direction != Some(desired_direction) || (desired_direction == DIRECTION_OUTPUT && refresh_countdown == 0) {
            match i2c_bus_dev.write(address, &[CMD_SET_DIRECTION, desired_direction]).await {
                Ok(()) => {
                    bridge_direction = Some(desired_direction);
                    refresh_countdown = DIRECTION_REFRESH_LOOPS;
                }
                Err(e) => {
                    bridge_direction = None;
                    error!("DMX direction error {:?}", e);
                    Timer::after_millis(250).await;
                }
            }
        }

        match input_mode {
            // Poll the received universe from the bridge.
            // DMX data is split between multiple ranges due to limited hardware buffer sizes.
            InputMode::Dmx => {
                match i2c_bus_dev.write_read(address, &[CMD_READ_BLOCK_1], &mut data_buffer[0..200]).await {
                    Ok(()) => {
                        match i2c_bus_dev.write_read(address, &[CMD_READ_BLOCK_2], &mut data_buffer[200..400]).await {
                            Ok(()) => {
                                match i2c_bus_dev.write_read(address, &[CMD_READ_BLOCK_3], &mut data_buffer[400..513]).await {
                                    Ok(()) => {
                                        let mut dmx_buffer = DMX_BUFFER.lock().await;
                                        dmx_buffer[0..513].copy_from_slice(&data_buffer);
                                        let _ = tx.try_send(DmxEvent::DmxPacket(PacketAddress::new(PortAddress::new(0, 0, 0), 0)));
                                    }
                                    Err(e) => {
                                        error!("DMX Error {:?}", e);
                                        Timer::after_millis(250).await;
                                    }
                                };
                            }
                            Err(e) => {
                                error!("DMX Error {:?}", e);
                                Timer::after_millis(250).await;
                            }
                        };
                    }
                    Err(e) => {
                        error!("DMX Error {:?}", e);
                        Timer::after_millis(250).await;
                    }
                };
            }

            // Push the outgoing frame to the bridge.
            InputMode::ArtNetToDmx | InputMode::UsbToDmx => {
                let mut frame = [0_u8; DMX_BUFF_SIZE];
                {
                    let dmx_buffer = DMX_BUFFER.lock().await;
                    match input_mode {
                        // USB data is stored DMX-style: start code + channels at offset 0
                        InputMode::UsbToDmx => frame.copy_from_slice(&dmx_buffer[0..DMX_BUFF_SIZE]),
                        // Art-Net data has no start code and lives at its universe offset
                        _ => {
                            let start = (artnet_universe as usize) * DMX_UNIVERSE_SIZE;
                            frame[0] = 0x00; // standard dimmer-data start code
                            frame[1..].copy_from_slice(&dmx_buffer[start..start + DMX_UNIVERSE_SIZE]);
                        }
                    }
                }

                let mut block = [0_u8; 201];
                for (cmd, range) in [
                    (CMD_WRITE_BLOCK_1, 0..200),
                    (CMD_WRITE_BLOCK_2, 200..400),
                    (CMD_WRITE_BLOCK_3, 400..513),
                ] {
                    let len = range.len();
                    block[0] = cmd;
                    block[1..1 + len].copy_from_slice(&frame[range]);
                    if let Err(e) = i2c_bus_dev.write(address, &block[..1 + len]).await {
                        error!("DMX write error {:?}", e);
                        bridge_direction = None; // re-sync direction on the next loop
                        Timer::after_millis(250).await;
                        break;
                    }
                }
            }

            // The network task feeds the LEDs directly; nothing to do here.
            InputMode::ArtNet => {}
        }

        Timer::after_millis(30).await;
    }
}
