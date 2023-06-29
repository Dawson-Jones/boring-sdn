pub fn parse_command() -> clap::ArgMatches {
    let matches = clap::Command::new("Boring-SDN")
        .version(clap::crate_version!())
        .author("Dawson Jones (jiadongqing@bytedance.com)")
        // TODO:
        // .arg(
        //     clap::Arg::new("config")
        //         .long("config")
        //         .short('c')
        //         .required(false)
        //         .value_name("PATH")
        //         .help("read configure from File")
        // )
        .arg(
            clap::Arg::new("tun_name")
                .long("tun")
                .required(false)
                .value_name("tunX")
                .help("Sets the Tun interface name, if absent, pick the next available name")
                .default_value("")
        )
        .arg(
            clap::Arg::new("tun_ip")
                .long("tun-ip")
                .required(false)
                .value_name("IP")
                .help("Sets the Tun interface address")
        )
        .arg(
            clap::Arg::new("type")
                .long("type")
                .required(false)
                .value_name("tun/tap")
                .help("Set type of interface. default: tun")
                .default_value("tun")
        )
        .arg(
            clap::Arg::new("listen")
                .long("listen")
                .short('l')
                .required(true)
                .value_name("PORT")
                .help("which port be listened, default: 55001")
                .default_value("55001")
        )
        .get_matches();

    matches
}
