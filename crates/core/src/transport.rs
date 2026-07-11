use std::net::SocketAddr;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 5000;
pub const GAMEPLAY_PORT_OFFSET: u16 = 1;

pub const PROTOCOL_ID: u64 = 0x1cbe_4f9e_d4a0_4c2b;
pub const PRIVATE_KEY: [u8; 32] = [0xAA; 32];
pub const NETCODE_CLIENT_TIMEOUT_SECS: i32 = 60;
pub const NETCODE_TOKEN_EXPIRE_SECS: i32 = -1;
pub const CLIENT_SEND_INTERVAL: Duration = Duration::from_millis(33);
pub const PING_INTERVAL: Duration = Duration::from_secs(1);
pub const SERVER_POS_UPDATE_INTERVAL_TICKS: u32 = 2;

pub fn server_listen_addr() -> SocketAddr {
    let port = server_listen_port_from_env(std::env::var("LK2_PORT").ok().as_deref());
    SocketAddr::from(([0, 0, 0, 0], port))
}

pub fn gameplay_port_for(main_port: u16) -> u16 {
    main_port.saturating_add(GAMEPLAY_PORT_OFFSET)
}

pub fn gameplay_addr_for_server(mut server_addr: SocketAddr) -> SocketAddr {
    server_addr.set_port(gameplay_port_for(server_addr.port()));
    server_addr
}

fn server_listen_port_from_env(raw: Option<&str>) -> u16 {
    raw.and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_PORT)
}

pub fn parse_connect_arg(args: &[String]) -> Option<SocketAddr> {
    args.iter()
        .find(|a| a.starts_with("--connect="))
        .and_then(|a| a.trim_start_matches("--connect=").parse().ok())
}

pub fn generate_client_id() -> u64 {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    client_id_from_time_and_process(duration.as_nanos(), std::process::id())
}

fn client_id_from_time_and_process(nanos_since_epoch: u128, process_id: u32) -> u64 {
    let folded_time = (nanos_since_epoch as u64) ^ ((nanos_since_epoch >> u64::BITS) as u64);
    folded_time ^ u64::from(process_id)
}

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub offline: bool,

    pub connect: Option<SocketAddr>,

    pub auto_demo: bool,

    pub preset: String,

    pub walk: Option<(i32, i32)>,
}

impl CliArgs {
    pub fn parse() -> Self {
        Self::parse_from(&std::env::args().collect::<Vec<_>>())
    }

    pub fn parse_from(args: &[String]) -> Self {
        let offline = args.iter().any(|a| a == "--offline");
        let auto_demo = args.iter().any(|a| a == "--auto-demo");
        let connect = parse_connect_arg(args);

        let preset = args
            .iter()
            .find(|a| a.starts_with("--preset="))
            .map(|a| a.trim_start_matches("--preset=").to_string())
            .unwrap_or_else(|| "default".to_string());

        let walk = args
            .iter()
            .find(|a| a.starts_with("--walk="))
            .and_then(|a| {
                let s = a.trim_start_matches("--walk=");
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() != 2 {
                    return None;
                }
                match (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                    (Ok(x), Ok(z)) => Some((x, z)),
                    _ => None,
                }
            });

        Self {
            offline,
            connect,
            auto_demo,
            preset,
            walk,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_args(flags: &[&str]) -> Vec<String> {
        flags.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn default_port_is_5000() {
        assert_eq!(DEFAULT_PORT, 5000);
    }

    #[test]
    fn gameplay_port_is_main_port_plus_one_with_saturation() {
        assert_eq!(gameplay_port_for(DEFAULT_PORT), DEFAULT_PORT + 1);
        assert_eq!(gameplay_port_for(u16::MAX), u16::MAX);
    }

    #[test]
    fn protocol_id_is_nonzero() {
        assert_ne!(PROTOCOL_ID, 0);
    }

    #[test]
    fn private_key_has_netcode_length() {
        assert_eq!(PRIVATE_KEY.len(), 32);
    }

    #[test]
    fn network_intervals_are_reasonable() {
        assert!(CLIENT_SEND_INTERVAL.as_millis() <= 50);
        assert_eq!(PING_INTERVAL.as_secs(), 1);
        assert!(SERVER_POS_UPDATE_INTERVAL_TICKS > 0);
        assert!(NETCODE_CLIENT_TIMEOUT_SECS >= 10);
        assert_eq!(NETCODE_TOKEN_EXPIRE_SECS, -1);
    }

    #[test]
    fn server_listen_addr_default() {
        assert_eq!(server_listen_port_from_env(None), DEFAULT_PORT);
        assert_eq!(
            server_listen_port_from_env(Some("not-a-port")),
            DEFAULT_PORT
        );
    }

    #[test]
    fn server_listen_addr_env_override() {
        assert_eq!(server_listen_port_from_env(Some("7777")), 7777);
    }

    #[test]
    fn server_listen_addr_uses_unspecified_ip() {
        let addr = server_listen_addr();
        assert!(addr.ip().is_unspecified());
    }

    #[test]
    fn gameplay_addr_preserves_ip_and_moves_to_gameplay_port() {
        let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let gameplay = gameplay_addr_for_server(addr);
        assert_eq!(gameplay.ip(), addr.ip());
        assert_eq!(gameplay.port(), 5001);
    }

    #[test]
    fn parse_connect_arg_basic() {
        let args = make_args(&["--connect=127.0.0.1:5000"]);
        let addr = parse_connect_arg(&args).unwrap();
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert_eq!(addr.port(), 5000);
    }

    #[test]
    fn parse_connect_arg_missing() {
        let args = make_args(&["--offline"]);
        assert!(parse_connect_arg(&args).is_none());
    }

    #[test]
    fn parse_connect_arg_bad_format() {
        let args = make_args(&["--connect=not_an_address"]);
        assert!(parse_connect_arg(&args).is_none());
    }

    #[test]
    fn client_id_generation_folds_time_and_process_id() {
        let low_only = client_id_from_time_and_process(0xABCD, 0x1234);
        assert_eq!(low_only, 0xB9F9);

        let high_bits = client_id_from_time_and_process(1_u128 << 64, 0);
        assert_eq!(high_bits, 1);

        let mixed = client_id_from_time_and_process((2_u128 << 64) | 5, 7);
        assert_eq!(mixed, 2 ^ 5 ^ 7);
    }

    #[test]
    fn cli_args_default() {
        let args: Vec<String> = vec![];
        let cli = CliArgs::parse_from(&args);
        assert!(!cli.offline);
        assert!(!cli.auto_demo);
        assert!(cli.connect.is_none());
        assert_eq!(cli.preset, "default");
        assert!(cli.walk.is_none());
    }

    #[test]
    fn cli_args_offline_and_auto_demo() {
        let args = make_args(&["--offline", "--auto-demo"]);
        let cli = CliArgs::parse_from(&args);
        assert!(cli.offline);
        assert!(cli.auto_demo);
    }

    #[test]
    fn cli_args_preset() {
        let args = make_args(&["--preset=flat_spawn"]);
        let cli = CliArgs::parse_from(&args);
        assert_eq!(cli.preset, "flat_spawn");
    }

    #[test]
    fn cli_args_walk() {
        let args = make_args(&["--walk=10,20"]);
        let cli = CliArgs::parse_from(&args);
        assert_eq!(cli.walk, Some((10, 20)));
    }

    #[test]
    fn cli_args_walk_bad_format() {
        let args = make_args(&["--walk=10"]);
        let cli = CliArgs::parse_from(&args);
        assert!(cli.walk.is_none());

        let args = make_args(&["--walk=abc,def"]);
        let cli = CliArgs::parse_from(&args);
        assert!(cli.walk.is_none());
    }

    #[test]
    fn cli_args_full() {
        let args = make_args(&["--offline", "--auto-demo", "--preset=hills", "--walk=5,15"]);
        let cli = CliArgs::parse_from(&args);
        assert!(cli.offline);
        assert!(cli.auto_demo);
        assert_eq!(cli.preset, "hills");
        assert_eq!(cli.walk, Some((5, 15)));
    }
}
