use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Runner, Stack};
use embassy_stm32::eth::generic_smi::GenericSMI;
use embassy_stm32::eth::Ethernet;
use embassy_stm32::peripherals::ETH;

use crate::channels::DmxChannelTx;
use crate::event_router::DmxEvent;

mod tiny_artnet;
use tiny_artnet::Art;

#[embassy_executor::task]
pub async fn artnet_task(
    stack: Stack<'static>,
    runner: Runner<'static, Ethernet<'static, ETH, GenericSMI>>,
    spawner: Spawner,
    tx: DmxChannelTx,
) {
    // let mut art_net = ArtNet::new(eth, 8, rx);

    // art_net.enable().await;
    // loop {
    //     art_net.show().await;
    // }

    // UDP example https://github.com/embassy-rs/embassy/blob/embassy-stm32-v0.2.0/examples/rp/src/bin/ethernet_w5500_udp.rs

    // Launch network task
    unwrap!(spawner.spawn(net_task(runner)));

    // Ensure DHCP configuration is up before trying connect
    info!("Waiting for DHCP...");
    let cfg = wait_for_config(stack).await;
    let local_addr = cfg.address.address();
    info!("IP address: {:?}", local_addr);

    // Lookup the mac address -- assumed to be the first 6 bytes of the micro UID
    let uid = embassy_stm32::uid::uid();
    let mac_address_bytes = [uid[0], uid[1], uid[2], uid[3], uid[4], uid[5]];

    info!("Network task initialized");

    // Then we can use it!
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    // let mut buf = [0; 4096];

    info!("Network buffers initialized");

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    let port = tiny_artnet::PORT;
    // let port = 6;
    socket.bind(port).unwrap();

    info!("Socked bound");

    let mut buf = [0; 65_507];

    info!("Buffer created");

    loop {
        let (len, from_addr) = socket.recv_from(&mut buf).await.unwrap();

        // info!("{:?}", buf);

        match tiny_artnet::from_slice(&buf[..len]) {
            Ok(Art::Dmx(dmx)) => {
                info!(
                    "RX: ArtDMX - These packets contain data for one DMX512 universe Seq: {:?} physical: {:?} port_address: {:?} Data: {:?}...",
                    dmx.sequence,
                    dmx.physical,
                    dmx.port_address,
                    &dmx.data[0..10],
                );

                // package the dmx data into 513 bytes
                let mut buf = [0_u8; DMX_BUFF_SIZE];
                // dmx.data does not include the DMX start byte. The packet exepcted by
                dmx.data
                    .iter()
                    .enumerate()
                    .for_each(|(i, v)| buf[i + 1] = *v);

                if !tx.is_empty() {
                    info!("Clearing DMX channel");
                    tx.clear(); // clear any existing message on the channel
                }
                match tx.try_send(DmxEvent::DmxPacket(buf)) {
                    Ok(_) => {}
                    Err(_) => error!("Unable to send DMX Event packet"),
                }
            }
            Ok(Art::Sync) => {
                info!("RX: ArtSync - Use these to buffer DMX packets and then synchronize the rendering of multiple DMX universes.");
            }
            Ok(Art::Poll(poll)) => {
                // info!("RX: ArtPoll - Someone is looking for ArtNet nodes. Let's respond to them to make this node discoverable! {:?}", poll);
                info!("RX: ArtPoll - Someone is looking for ArtNet nodes. Let's respond to them to make this node discoverable!");

                // info!("poll: {:?} {:?} {:?}", poll.flags, poll.min_diagnostic_priority, poll.target_port_addresses);

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
                // let msg_len = poll_reply.serialize(&mut buf);

                socket
                    .send_to(
                        //&buf[..msg_len],
                        &reply_message,
                        from_addr,
                    )
                    .await
                    .unwrap();
                // let broadcast: UdpSocket = UdpSocket::bind("0.0.0.0:0").unwrap();
                // broadcast
                //     .set_read_timeout(Some(Duration::new(5, 0)))
                //     .unwrap();
                // broadcast.set_broadcast(true).unwrap();
                // broadcast
                //     .send_to(&buf[..msg_len], "255.255.255.255")
                //     .unwrap();
                info!(
                    "Sent ArtPollReply to {:?}:{:?} {:?}",
                    from_addr.endpoint.addr, from_addr.endpoint.port, poll_reply
                );
                // info!("TX: Sent ArtPollReply");
            }
            Ok(Art::Command(command)) => {
                info!(
                    "command {:?} - {:?}",
                    command.esta_manufacturer_code, command.data
                );
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

type Device = Ethernet<'static, ETH, GenericSMI>;

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, Device>) -> ! {
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
