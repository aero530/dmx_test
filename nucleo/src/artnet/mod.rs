use defmt::*;
// use embassy_executor::Spawner;
use embassy_futures::yield_now;
// use embassy_net::tcp::TcpSocket;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::Stack;
use embassy_stm32::eth::{Ethernet, GenericPhy};
use embassy_stm32::peripherals::ETH;

// use embassy_time::{Duration, Timer};
// use embassy_net::Ipv4Address;
// use embedded_io_async::Write;

use crate::channels::{DmxChannelTx, DmxFeedbackChannelRx, RouterChannelTx};
use crate::event_router::{DmxEvent, DmxFeedbackEvent, RouterEvent, DMX_BUFFER};
use crate::ui::InputMode;
use crate::DMX_ADDR_MAX;

mod tiny_artnet;
pub use tiny_artnet::{Art, PortAddress};

#[embassy_executor::task]
pub async fn artnet_task(
    stack: Stack<'static>,
    // runner: Runner<'static, Ethernet<'static, ETH, GenericPhy>>,
    // spawner: Spawner,
    tx: DmxChannelTx,
    tx_router: RouterChannelTx,
    mut rx: DmxFeedbackChannelRx,
) {
    // Ensure DHCP configuration is up before trying connect
    info!("Waiting for DHCP...");
    let a = stack.wait_config_up().await;

    let cfg = stack.config_v4().unwrap();

    let local_addr = cfg.address.address();
    info!(" ");
    info!("IP address: {:?}", local_addr);
    info!(" ");

    let _ = tx_router.try_send(RouterEvent::StoreIpAddr(Some(local_addr)));

    // Lookup the mac address -- assumed to be the first 6 bytes of the micro UID
    // let uid = embassy_stm32::uid::uid();
    // let mac_address_bytes = [uid[0], uid[1], uid[2], uid[3], uid[4], uid[5]];
    let hw_addr = stack.hardware_address();
    let mut mac_address_bytes = [0; 6];
    match hw_addr {
        embassy_net::HardwareAddress::Ethernet(address) => {
            for (i, b) in address.as_bytes().iter().enumerate() {
                mac_address_bytes[i] = *b;
            }
        }
    }
    // let mac_address_bytes = [mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]];

    // Then we can use it!
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];

    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);

    let port = tiny_artnet::PORT;
    socket.bind(port).unwrap();

    let mut buf = [0; 65_507];
    let mut input_mode = InputMode::default();

    loop {
        if let Some(input_data) = rx.try_changed() {
            // info!("ArtNet - update mode to {}", input_data);
            match input_data {
                DmxFeedbackEvent::Mode(new_mode) => {
                    input_mode = new_mode;
                }
            }
        }

        let (len, from_addr) = socket.recv_from(&mut buf).await.unwrap();
        // trace!("Ethernet {:?}", buf);

        match tiny_artnet::from_slice(&buf[..len]) {
            Ok(Art::Dmx(dmx)) => {
                // info!(
                //     "RX: ArtDMX - These packets contain data for one DMX512 universe Seq: {:?} physical: {:?} port_address: {:?} Data: {:?}...",
                //     dmx.sequence,
                //     dmx.physical,
                //     dmx.port_address,
                //     &dmx.data[0..10],
                // );

                let universe = dmx.port_address.universe as usize;
                let start = 0 + DMX_ADDR_MAX * universe;
                let end = DMX_ADDR_MAX + DMX_ADDR_MAX * universe;
                let mut dmx_buffer = DMX_BUFFER.lock().await;
                dmx_buffer[start..end].copy_from_slice(dmx.data);
                // info!("{}", dmx_buffer[start..end]);

                if input_mode == InputMode::ArtNet {
                    // if dmx.port_address.universe == 2 {
                    let _ = tx.try_send(DmxEvent::ArtNetPacket((dmx.port_address, dmx.sequence)));
                    // }
                }
                // match tx.try_send(DmxEvent::ArtNetPacket((dmx.port_address, dmx.sequence))) {
                //     Ok(_) => {}
                //     Err(_) => error!("Unable to send DMX Event packet"),
                // };

                // // package the dmx data into 513 bytes
                // let mut buf_out = [0_u8; DMX_BUFF_SIZE];
                // // dmx.data does not include the DMX start byte.
                // dmx.data.iter().enumerate().for_each(|(i, v)| buf_out[i + 1] = *v);

                // if !tx.is_empty() {
                //     info!("ArtNet DMX Buffer Full - Clearing DMX channel");
                //     tx.clear(); // clear any existing message on the channel
                // }
                // match tx.try_send(DmxEvent::ArtNetPacket((buf_out, dmx.port_address, dmx.sequence))) {
                //     Ok(_) => {}
                //     Err(_) => error!("Unable to send DMX Event packet"),
                // }
            }
            Ok(Art::Sync) => {
                debug!("RX: ArtSync - Use these to buffer DMX packets and then synchronize the rendering of multiple DMX universes.");
            }
            Ok(Art::Poll(poll)) => {
                // info!("RX: ArtPoll - Someone is looking for ArtNet nodes. Let's respond to them to make this node discoverable! {:?}", poll);
                debug!("RX: ArtPoll - Someone is looking for ArtNet nodes. Let's respond to them to make this node discoverable!");

                let poll_reply = tiny_artnet::PollReply {
                    ip_address: &local_addr.octets(),
                    port,
                    firmware_version: 0x0001,
                    short_name: "Example Node",
                    long_name: "Tiny Artnet Example Node",
                    //  &'static [u8] = b"Art-Net\0";
                    mac_address: &mac_address_bytes,
                    // This Node has one port
                    num_ports: 1,
                    // This node has one output channel
                    port_types: &[0b10000000, 0, 0, 0],
                    // Report that data is being output correctly
                    good_output_a: &[0b10000000, 0, 0, 0],
                    ..Default::default()
                };

                let reply_message = poll_reply.ser();

                let _ = socket
                    .send_to(
                        //&buf[..msg_len],
                        &reply_message,
                        from_addr,
                    )
                    .await
                    .map_err(|e| error!("Artnet Unable to send on socket."));

                debug!("Sent ArtPollReply to {:?}:{:?} {:?}", from_addr.endpoint.addr, from_addr.endpoint.port, poll_reply);
            }
            Ok(Art::Command(command)) => {
                debug!("command {:?} - {:?}", command.esta_manufacturer_code, command.data);
            }
            Err(err) => {
                // info!("Error: {:?}", err);

                match err {
                    tiny_artnet::Error::UnsupportedProtocolVersion(e) => {
                        error!("ArtNet Unsupported protocol version {}", e)
                    }
                    tiny_artnet::Error::UnsupportedOpCode(e) => {
                        error!("ArtNet Unsupported op code {}", e)
                    }
                    tiny_artnet::Error::ParseIncomplete(e) => {
                        error!("ArtNet parse incomplete {}", e)
                    }
                    tiny_artnet::Error::ParseError(error_kind) => {
                        error!("ArtNet parse error {}", error_kind)
                    }
                    tiny_artnet::Error::ParseFailure => error!("ArtNet parse failure"),
                }
            }
        };
    }
}

type Device = Ethernet<'static, ETH, GenericPhy>;

#[embassy_executor::task]
pub async fn net_task(mut runner: embassy_net::Runner<'static, Device>) -> ! {
    runner.run().await
}

async fn wait_for_config(stack: Stack<'static>) -> embassy_net::StaticConfigV4 {
    loop {
        if let Some(config) = stack.config_v4() {
            return config.clone();
        }
        yield_now().await;
    }
}
