use defmt::*;
use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpAddress, Runner, Stack};
use embassy_stm32::eth::generic_smi::GenericSMI;
use embassy_stm32::eth::Ethernet;
use embassy_stm32::peripherals::ETH;
use embassy_futures::yield_now;

use tiny_artnet::Art;

use crate::channels::ArtNetChannelRx;


#[derive(Format)]
pub enum ArtNetEvent {
    Value([u8; 4]),
}


pub struct ArtNet<'a> {
    stack: Stack<'a>,
    rx_buffer: [u8; 4096],
    tx_buffer: [u8; 4096],
    rx: ArtNetChannelRx,
}

impl<'a> ArtNet<'a> {
    pub fn new(stack: Stack<'a>, rx_buffer: [u8; 4096], tx_buffer: [u8; 4096], rx: ArtNetChannelRx,) -> Self {
        
        Self { stack, rx_buffer, tx_buffer, rx }
    }

    pub async fn show(&mut self) {
        // if let Ok(new_message) = with_timeout(Duration::from_millis(100), self.rx.receive()).await {
        //     // info!("led message {:?}", new_message);
        //     self.process_event(new_message).await;
        // }
    }

    async fn process_event(&mut self, event: ArtNetEvent) {
        match event {
            
            ArtNetEvent::Value(values) => {
                // let prev_length = self.num_leds;
                // self.num_leds = ((values[3] as usize) * 100) / 255 as usize;
                
                // for i in 0..self.num_leds {
                //     self.data[i] = RGB8::new(values[0], values[1], values[2]);
                // }
                // for i in self.num_leds..prev_length+1 {
                //     self.data[i] = RGB8::default();
                // }
                // self.ws.write(self.data).await.ok();
            }
        }
    }
}



#[embassy_executor::task]
pub async fn artnet_task(stack: Stack<'static>, runner: Runner<'static, Ethernet<'static, ETH, GenericSMI>>, spawner: Spawner, rx: ArtNetChannelRx) {
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

    let mut socket = UdpSocket::new(stack,  &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);
    let port = tiny_artnet::PORT;
    // let port = 6;
    socket.bind(port).unwrap();


    let mut buf = [0; 65_507];

    loop {
        let (len, from_addr) = socket.recv_from(&mut buf).await.unwrap();

        println!("{:?}", buf);
        match tiny_artnet::from_slice(&buf[..len]) {
            Ok(Art::Dmx(dmx)) => {
                info!(
                    "RX: ArtDMX - These packets contain data for one DMX512 universe - use them to control your node's lighting, etc. Seq: {:?} Data: {:?}...",
                    dmx.sequence,
                    &dmx.data[0..10],
                );
            }
            Ok(Art::Sync) => {
                info!("RX: ArtSync - Use these to buffer DMX packets and then synchronize the rendering of multiple DMX universes.");
            }
            Ok(Art::Poll(poll)) => {
                // info!("RX: ArtPoll - Someone is looking for ArtNet nodes. Let's respond to them to make this node discoverable! {:?}", poll);
                info!("RX: ArtPoll - Someone is looking for ArtNet nodes. Let's respond to them to make this node discoverable!");
                
                info!("poll: {:?} {:?} {:?}", poll.flags, poll.min_diagnostic_priority, poll.target_port_addresses);

                let poll_reply = tiny_artnet::PollReply {
                    ip_address: &local_addr.octets(),
                    port,
                    firmware_version: 0x0001,
                    short_name: "Example Node",
                    long_name: "Tiny Artnet Example Node",
                    mac_address: &mac_address_bytes,
                    // This Node has one port
                    num_ports: 1,
                    // This node has one output channel
                    port_types: &[0b10000000, 0, 0, 0],
                    // Report that data is being output correctly
                    good_output_a: &[0b10000000, 0, 0, 0],
                    ..Default::default()
                };

                let msg_len = poll_reply.serialize(&mut buf);
                socket.send_to(
                    &buf[..msg_len],
                    from_addr
                ).await.unwrap();
                // let broadcast: UdpSocket = UdpSocket::bind("0.0.0.0:0").unwrap();
                // broadcast
                //     .set_read_timeout(Some(Duration::new(5, 0)))
                //     .unwrap();
                // broadcast.set_broadcast(true).unwrap();
                // broadcast
                //     .send_to(&buf[..msg_len], "255.255.255.255")
                //     .unwrap();

                // info!("TX: Sent ArtPollReply to {:?}: {:?}", from_addr, poll_reply);
                info!("TX: Sent ArtPollReply");
            }
            Ok(Art::Command(command)) => {
                info!("command {:?} - {:?}", command.esta_manufacturer_code, command.data);
            }
            Err(err) => {
                // info!("Error: {:?}", err);
                
                match err {
                    tiny_artnet::Error::UnsupportedProtocolVersion(_) => error!("ArtNet Unsupported protocol version"),
                    tiny_artnet::Error::UnsupportedOpCode(_) => error!("ArtNet Unsupported op code"),
                    tiny_artnet::Error::ParserError(_) => error!("ArtNet parse error"),
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
