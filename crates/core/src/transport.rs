use std::net::SocketAddr;

pub const DEFAULT_PORT: u16 = 5000;

pub const PROTOCOL_ID: u64 = 0x1cbe_4f9e_d4a0_4c2b;

pub fn server_listen_addr() -> SocketAddr {
    let port = server_listen_port_from_env(std::env::var("LK2_PORT").ok().as_deref());
    SocketAddr::from(([0, 0, 0, 0], port))
}

fn server_listen_port_from_env(raw: Option<&str>) -> u16 {
    raw.and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_PORT)
}

pub fn parse_connect_arg(args: &[String]) -> Option<SocketAddr> {
    args.iter()
        .find(|a| a.starts_with("--connect="))
        .and_then(|a| a.trim_start_matches("--connect=").parse().ok())
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

        let walk = args.iter().find(|a| a.starts_with("--walk=")).and_then(|a| {
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

        Self { offline, connect, auto_demo, preset, walk }
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
    fn protocol_id_is_nonzero() {
        assert_ne!(PROTOCOL_ID, 0);
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
