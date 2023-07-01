use std::{net::{Ipv4Addr, SocketAddrV4}, fs};

use serde::{Serialize, Deserialize};


fn tun_mode_default() -> String {
    String::from("tun")
}

fn tun_name_default() -> String {
    String::from("")
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "tun_name_default")]
    tun_name: String,
    tun_ip_cidr: Option<String>,
    #[serde(default = "tun_mode_default")]
    tun_mode: String,
    listen: String,
    mtu: Option<i32>,
}


pub struct Command {
    pub tun_name: String,
    pub tun_ip_cidr: Option<(Ipv4Addr, Ipv4Addr)>,
    pub tun_mode: String,
    pub listen: SocketAddrV4,
    pub mtu: i32,
}

fn cidr_to_netmask(cidr: u8) -> Ipv4Addr {
    let mask: u32 = 0xFFFFFFFF << (32 - cidr);
    Ipv4Addr::from(mask)
}


fn split_ip_cidr(s: &str) -> (Ipv4Addr, Ipv4Addr) {
    let ip_cidr: Vec<&str> = s.split("/").collect();
    
    let ip = ip_cidr[0].parse::<Ipv4Addr>().unwrap();
    if ip_cidr.len() > 1 {
        let cidr = ip_cidr[1].parse::<u8>().unwrap();
        (ip, cidr_to_netmask(cidr))
    } else {
        (ip, cidr_to_netmask(32))
    }
}

fn split_ip_port(s: &str) -> SocketAddrV4 {
    let ip_cidr: Vec<&str> = s.split(":").collect();
    let ip;
    let port;
    
    if ip_cidr.len() > 1 {
        ip = ip_cidr[0].parse::<Ipv4Addr>().unwrap();
        port = ip_cidr[1].parse::<u16>().unwrap();
    } else {
        ip = "0.0.0.0".parse::<Ipv4Addr>().unwrap();
        port = ip_cidr[0].parse::<u16>().unwrap();
    }

    SocketAddrV4::new(ip, port)
}

fn usage() -> clap::ArgMatches {
    let matches = clap::Command::new("Boring-SDN")
        .version(clap::crate_version!())
        .author("Dawson Jones (jiadongqing@bytedance.com)")
        .arg(
            clap::Arg::new("config")
                .long("config")
                .short('c')
                .required(false)
                .value_name("PATH")
                .help("read configure from File")
        )
        .arg(
            clap::Arg::new("tun_name")
                .long("tun")
                .required(false)
                .value_name("tunX")
                .help("Sets the Tun interface name, if absent, pick the next available name")
        )
        .arg(
            clap::Arg::new("tun_ip")
                .long("tun-ip")
                .required(false)
                .value_name("IP/CIDR")
                .help("Sets the Tun interface address in CIDR format, if CIDR is absent, it defaults to use 32.")
        )
        .arg(
            clap::Arg::new("mode")
                .long("type")
                .required(false)
                .value_name("tun/tap")
                .help("Set type of interface. default: tun")
        )
        .arg(
            clap::Arg::new("listen")
                .long("listen")
                .short('l')
                .required(false)
                .value_name("PORT")
                .help("which port be listened, default: 55001")
                // .default_value("55001")
        )
        .arg(
            clap::Arg::new("mtu")
                .long("mtu")
                .value_name("mtu")
                .required(false)
                .help("default: 1400")
        )
        .get_matches();

    matches
}

pub fn parse_command() -> Command {
    let matches = usage();

    let mut command = Command {
        tun_name: "tun0".to_string(),
        tun_ip_cidr: None,
        tun_mode: "tun".to_string(),
        listen: SocketAddrV4::new("0.0.0.0".parse::<Ipv4Addr>().unwrap(), 55001),
        mtu: 1400,
    };

    let config_str= matches.get_one::<String>("config").map(fs::read).transpose().unwrap();
    if let Some(p) = config_str {
        log::info!("--config specified");

        let config: Config = toml::from_slice(p.as_slice()).unwrap();
        log::info!("config: {:#?}", config);

        command.tun_name = config.tun_name;
        if let Some(ip_cidr) = config.tun_ip_cidr {
            command.tun_ip_cidr = Some(split_ip_cidr(&ip_cidr));
        }
        if let Some(mtu) = config.mtu{
            command.mtu = mtu;
        }

        command.tun_mode = config.tun_mode;

        command.listen = split_ip_port(&config.listen);
    }

    if let Some(tun_name) = matches.get_one::<String>("tun_name") {
        command.tun_name = tun_name.to_string();
        log::info!("--command-tun_name specified: {}", tun_name);
    }
    if let Some(ip_cidr) = matches.get_one::<String>("tun_ip") {
        command.tun_ip_cidr = Some(split_ip_cidr(&ip_cidr));
        log::info!("--command-tun_ip specified: {}", ip_cidr);
    }
    if let Some(mode) = matches.get_one::<String>("mode") {
        log::info!("--command-mode specified: {}", mode);
        if mode == "tap" {
            command.tun_mode = mode.to_string();
        }
    }
    if let Some(listen) = matches.get_one::<String>("listen") {
        command.listen = split_ip_port(listen);
        log::info!("--command-listen specified: {}", command.listen.to_string());
    }
    if let Some(mtu) = matches.get_one::<String>("mtu") {
        command.mtu = mtu.parse::<i32>().unwrap();
        log::info!("--command-mtu specified: {}", mtu);
    }

    command
}


mod tests {
    use super::*;

    #[test]
    fn parse_config() {
        // read current package
        let content = fs::read("../config.toml").unwrap();
        let config: Config = toml::from_slice(content.as_slice()).unwrap();
        println!("{:#?}", config);
    }

    #[test]
    fn test_cidr() {
        let cidr = 12;
        let ip = cidr_to_netmask(cidr);
        println!("cidr: {cidr}, mask: {ip}");
    }
}