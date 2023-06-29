use std::net::SocketAddr;
use std::net::{Ipv4Addr, IpAddr};

use socket2::{Socket, Domain, Type, SockAddr};
use tokio::net::UdpSocket;
use tokio_tun::{TunBuilder, Tun};

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
    let matches = parse::parse_command();

    // TODO: 迁移到 parse 里面, 使用结构体输出结构
    let tun_name = matches.get_one::<String>("tun_name").unwrap();
    let tun_ip = matches.get_one::<String>("tun_ip").unwrap().parse::<Ipv4Addr>().unwrap();
    // let tpe = matches.get_one::<String>("type").unwrap();
    let listen = matches.get_one::<String>("listen").unwrap().parse::<u16>().unwrap();

    let tun: Tun = TunBuilder::new()
        .name(&tun_name)
        // TODO: 可以在内部或设置
        .address(tun_ip)
        // TODO: 命令行 tun_ip 设置 192.168.1.5/24
        .netmask("255.255.255.0".parse::<Ipv4Addr>().unwrap())
        // .mtu(mtu)
        .tap(false)
        .packet_info(false)
        .up()
        .try_build()
        .unwrap();
    log::info!("tun interface built");

    let local_addr = SocketAddr::from(("0.0.0.0".parse::<IpAddr>().unwrap(), listen));
    let udp_srv = new_udp_reuseport(local_addr);
    log::info!("udp server: {} created", local_addr.to_string());

    let mut buf_from_local = [0u8; MAX_PACKET_LEN];
    let mut buf_from_remote = [0u8; MAX_PACKET_LEN];
    // TODO: 
    //  try multiple, 
    loop {
        tokio::select! {
            // if let Ok(size) = tun.recv(&mut buf_from_local).await {
            Ok(size) = tun.recv(&mut buf_from_local) => {
                let (buf, remote_addr) = simple_route::route_from_local(&mut buf_from_local[..size]);

                // TODO: restore the UDP socket in somewhere 
                //  to avoid calling syscall when creating udp_cli
                let udp_cli = new_udp_cli(remote_addr);
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