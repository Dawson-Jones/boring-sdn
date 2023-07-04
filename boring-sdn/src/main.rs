use std::time::Duration;
use std::{net::SocketAddr};
use std::sync::Arc;

use futures::future;
use socket2::{Socket, Domain, Type, SockAddr};
use tokio::{net::UdpSocket};
use tokio_tun::{TunBuilder, Tun};
use lru_time_cache::LruCache;

// use boring_sdn::client;
// use boring_sdn::server;
use boring_sdn::parse;

const MAX_PACKET_LEN: usize = 1500;

// TODO: buf
// struct Buff {
//     data_meta: *const u8,
//     data: *const u8,
//     data_end: *const u8,
//     data_meta_end: *const u8,
// }

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let command = parse::parse_command();

    let num_cpus = num_cpus::get();
    log::info!("cpus: {}", num_cpus);
    let mut tun_builder = TunBuilder::new()
        .name(&command.tun_name)
        .mtu(command.mtu)
        .tap(if command.tun_mode == "tap" {true} else {false})
        .packet_info(false)
        .up();
    if let Some((ip, mask)) = command.tun_ip_cidr {
        tun_builder = tun_builder.address(ip).netmask(mask);
    }
    let tuns = tun_builder.try_build_mq(num_cpus).unwrap();
    log::info!("tun interface built");

    let tuns: Vec<Arc<Tun>> = tuns.into_iter().map(Arc::new).collect();
    let mut handles = Vec::new();
    for i in 0..num_cpus {
        let tun = tuns[i].clone();

        let h = tokio::spawn(async move {
            let mut buf_from_local = [0u8; MAX_PACKET_LEN];
            let mut buf_from_remote = [0u8; MAX_PACKET_LEN];
            let udp_srv = new_udp_reuseport(SocketAddr::V4(command.listen));
            log::info!("cpu {}, udp server: {} created", i, command.listen.to_string());
            // let connections = Arc::new(RwLock::new(HashMap::<SocketAddr, Arc<UdpSocket>>::new()));
            let udp_ttl = Duration::from_secs(180);
            let max_count = 180;
            let mut connections = LruCache::<SocketAddr, UdpSocket>
                ::with_expiry_duration_and_capacity(udp_ttl, max_count);

            loop {
                tokio::select! {
                    // if let Ok(size) = tun.recv(&mut buf_from_local).await {
                    Ok(size) = tun.recv(&mut buf_from_local) => {
                        log::info!("tun recv from queue: {}", i);
                        let (buf, remote_addr) = simple_route::route_from_local(&mut buf_from_local[..size]);

                        // TODO: restore the UDP socket in somewhere 
                        //  to avoid calling syscall when creating udp_cli
                        if !connections.contains_key(&remote_addr) {
                            log::info!("queue {} not found udp_socket in LRU", i);
                            let udp_cli = new_udp_cli(remote_addr);
                            connections.insert(remote_addr, udp_cli);
                        } else {
                            log::info!("queue {} found udp_socket in LRU", i);
                        }
                        let udp_cli = connections.get(&remote_addr).unwrap();

                        match udp_cli.send(buf).await {
                            Ok(size) => {
                                log::info!("send packet {} to {}", size, remote_addr);
                            },
                            Err(e) => {
                                log::error!("send packet error: {}", e);
                            }
                        }
                    }

                    // if let Ok((size, remote_addr)) = udp_srv.recv_from(&mut buf_from_remote).await {
                    Ok((size, _remote_addr)) = udp_srv.recv_from(&mut buf_from_remote) => {
                        let (buf, forward) = simple_route::route_from_remote(&mut buf_from_remote[..size]);
                        if let Some(addr) = forward {
                            // TODO: see log
                            log::warn!("relay node, forward to {}", addr)
                        } else {
                            match tun.send(buf).await {
                                Ok(size) => {
                                    log::info!("packet {} up to application", size);
                                },
                                Err(e) => {
                                    log::error!("packet up to error: {}", e);
                                },
                            }
                        }
                    }
                }
            }

        });

        handles.push(h);
    }

    let futures = future::join_all(handles).await;
    println!("{:?}", futures);
}


fn new_udp_reuseport(local_addr: SocketAddr) -> UdpSocket {
    let udp_sock = Socket::new(
        if local_addr.is_ipv4() {
            Domain::IPV4
        } else {
            Domain::IPV6
        },
        Type::DGRAM, 
        None)
        .unwrap();
    
    udp_sock.set_reuse_port(true).unwrap();
    udp_sock.set_cloexec(true).unwrap();
    udp_sock.set_nonblocking(true).unwrap();
    udp_sock.bind(&SockAddr::from(local_addr)).unwrap();

    let udp_sock: std::net::UdpSocket = udp_sock.into();
    udp_sock.try_into().unwrap()
    // udp_sock.into()
}

fn new_udp_cli(remote_addr: SocketAddr) -> UdpSocket {
    let udp_sock = Socket::new(
        if remote_addr.is_ipv4() {
            Domain::IPV4
        } else {
            Domain::IPV6
        },
        Type::DGRAM, 
        None)
        .unwrap();

    udp_sock.set_nonblocking(true).unwrap();
    udp_sock.connect(&SockAddr::from(remote_addr)).unwrap();

    let udp_sock: std::net::UdpSocket = udp_sock.into();
    udp_sock.try_into().unwrap()
}