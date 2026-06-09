//! `lk2-core::transport` -- cross server / client shared transport + CLI parsing
//!
//! This module has **zero bevy dependency** (only `std::net` + `std::env`),
//! so `lk2-core` compiles without any bevy runtime resource, and both
//! `lk2-server` (MinimalPlugins headless) and `lk2-client` (DefaultPlugins)
//! can `use` it without side effects.
//!
//! Contents:
//!
//! - Port constants [`DEFAULT_PORT`] + lightyear protocol id [`PROTOCOL_ID`]
//! (both ends MUST agree, otherwise server rejects client).
//! - [`server_listen_addr`] -- parses `LK2_PORT` env, falls back to
//! [`DEFAULT_PORT`].
//! - [`parse_connect_arg`] -- parses `--connect=<ip:port>` (client side).
//! - [`CliArgs`] + [`CliArgs::parse`] -- centralises the scattered
//! `--offline` / `--auto-demo` / `--preset=` / `--walk=` parsing currently
//! inlined in client main.rs, so future client / server mains can just
//! call `CliArgs::parse()` to get all flags in one place.
//!
//! **Not in this module**: lightyear `ClientConfig` / `ServerConfig`
//! assembly -- that's per-binary concern (server uses MinimalPlugins +
//! ServerPlugins; client uses DefaultPlugins + ClientPlugins; config
//! surfaces differ wildly). This module only shares the **transport
//! consensus** (port / protocol id / startup args).

use std::net::SocketAddr;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Server default listen port. Client also falls back to this port when
/// the user did not pass `--connect=`.
pub const DEFAULT_PORT: u16 = 5000;

/// lightyear protocol id -- both ends MUST agree, otherwise the server
/// rejects the client.
///
///64-bit arbitrary constant (recommended: avoid common port hashes). We
/// picked a UUID-looking value (`1cbe_4f9e_d4a0_4c2b`) as a placeholder;
/// swap it for something less likely to collide if you ship publicly.
pub const PROTOCOL_ID: u64 = 0x1cbe_4f9e_d4a0_4c2b;

// ---------------------------------------------------------------------------
// server_listen_addr -- parse LK2_PORT env, fall back to DEFAULT_PORT
// ---------------------------------------------------------------------------

/// Parse `LK2_PORT` env, fall back to [`DEFAULT_PORT`], assemble as
/// `0.0.0.0:<port>`.
///
/// This is the server-side listen address; client side uses
/// [`parse_connect_arg`] to read the remote address to connect to.
///
/// # Examples
///
/// ```no_run
/// let addr = lk2_core::transport::server_listen_addr();
/// assert_eq!(addr.port(),5000);
/// ```
pub fn server_listen_addr() -> SocketAddr {
    let port: u16 =
        std::env::var("LK2_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_PORT);
    SocketAddr::from(([0, 0, 0, 0], port))
}

// ---------------------------------------------------------------------------
// parse_connect_arg -- parse --connect=<ip:port>
// ---------------------------------------------------------------------------

/// Pick `--connect=<ip:port>` out of argv and parse it as a `SocketAddr`.
///
/// No `--connect=` flag => returns `None` (client should run in
/// `--offline` mode).
/// Flag present but parse fails (port not a number / IP malformed) =>
/// also returns `None`; caller should `warn!` so the user knows their IP
/// was silently rejected.
///
/// # Examples
///
/// ```ignore
/// let args = vec![
/// "lk2-client".to_string(),
/// "--connect=127.0.0.1:5000".to_string(),
/// ];
/// let addr = lk2_core::transport::parse_connect_arg(&args);
/// assert_eq!(addr.unwrap().port(),5000);
/// ```
pub fn parse_connect_arg(args: &[String]) -> Option<SocketAddr> {
    args.iter()
        .find(|a| a.starts_with("--connect="))
        .and_then(|a| a.trim_start_matches("--connect=").parse().ok())
}

// ---------------------------------------------------------------------------
// CliArgs -- centralise the scattered client CLI flags
// ---------------------------------------------------------------------------

/// Centralised client-side CLI arguments.
///
/// Today client main.rs inlines things like
/// `args.iter().any(|a| a == "--offline")`. This struct's goal is to
/// collect them once; server can also reuse it later (e.g. if server
/// wants `--auto-demo` to disable monsters / disable screenshots).
///
/// Field semantics:
/// - `offline` -- start an **in-process** sim, do not connect to server
/// (`loop.ps1` default mode).
/// - `connect` -- server address to connect to (online client mode only).
/// - `auto_demo` -- auto-demo mode (used by `loop.ps1` / AI iteration).
/// - `preset` -- terrain preset name (defaults to `"default"`).
/// - `walk` -- forced spawn position `(x, z)`, skipping normal spawn
/// logic.
#[derive(Debug, Clone)]
pub struct CliArgs {
    /// `--offline` -- in-process sim, do not connect to server.
    pub offline: bool,
    /// `--connect=<ip:port>` -- server address to connect to (online mode).
    pub connect: Option<SocketAddr>,
    /// `--auto-demo` -- auto-demo mode (used by `loop.ps1` / AI iteration).
    pub auto_demo: bool,
    /// `--preset=<name>` -- terrain preset (defaults to `"default"`).
    pub preset: String,
    /// `--walk=<x>,<z>` -- forced spawn position (skips normal spawn logic).
    pub walk: Option<(i32, i32)>,
}

impl CliArgs {
    /// Collect all flags from `std::env::args()` in one call.
    ///
    /// Behaviour details:
    /// - `--offline` / `--auto-demo` -- present => `true`.
    /// - `--connect=<ip:port>` -- see [`parse_connect_arg`]; a parse
    /// failure is silently ignored (caller should `warn!` to surface
    /// the typo rather than the user silently getting `--offline`).
    /// - `--preset=<name>` -- defaults to `"default"`.
    /// - `--walk=<x>,<z>` -- bad format (missing comma / non-numeric) is
    /// silently ignored.
    pub fn parse() -> Self {
        Self::parse_from(&std::env::args().collect::<Vec<_>>())
    }

    /// Parse from a given argv slice -- exposed for tests and for mains
    /// that already collected their own argv.
    pub fn parse_from(args: &[String]) -> Self {
        let offline = args.iter().any(|a| a == "--offline");
        let auto_demo = args.iter().any(|a| a == "--auto-demo");
        let connect = parse_connect_arg(args);

        let preset = args
            .iter()
            .find(|a| a.starts_with("--preset="))
            .map(|a| a.trim_start_matches("--preset=").to_string())
            .unwrap_or_else(|| "default".to_string());

        // --walk=x,z -> Option<(i32, i32)>
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

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

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
        //0 is lightyear's "unset" placeholder; a real id must be nonzero.
        assert_ne!(PROTOCOL_ID, 0);
    }

    #[test]
    fn server_listen_addr_default() {
        // Without LK2_PORT set ->0.0.0.0:5000.
        // SAFETY: cargo test runs unit tests in parallel by default, but
        // env mutations on a const-named var are best-effort (this test
        // only asserts the default port when the env var is unset).
        unsafe {
            std::env::remove_var("LK2_PORT");
        }
        let addr = server_listen_addr();
        assert_eq!(addr.port(), DEFAULT_PORT);
        assert!(addr.ip().is_unspecified());
    }

    #[test]
    fn server_listen_addr_env_override() {
        unsafe {
            std::env::set_var("LK2_PORT", "7777");
        }
        let addr = server_listen_addr();
        assert_eq!(addr.port(), 7777);
        unsafe {
            std::env::remove_var("LK2_PORT");
        }
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
        // Missing comma / non-numeric -> silently None, do not panic.
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
