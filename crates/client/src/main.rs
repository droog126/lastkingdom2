mod ai_client;
mod codex_client;
mod game_scene;
mod milestone_capture;
mod model_preview;
mod nature;
mod online;
mod ray_aabb;
mod rendering;
mod terrain_preview;

pub(crate) use rendering::crisp_image_plugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClientMode {
    AiClient,
    GameScene,
    ModelPreview,
    TerrainPreview,
    Online,
}

fn select_mode(args: &[String]) -> Result<ClientMode, &'static str> {
    if args.iter().any(|arg| arg == "--codex-client") {
        // Codex is an online player by default, so `--connect` and the loop
        // harness reach the real Lightyear scene. Keep the local prototype
        // available only behind an explicit offline flag.
        return if args.iter().any(|arg| arg == "--offline") {
            Ok(ClientMode::GameScene)
        } else {
            Ok(ClientMode::Online)
        };
    }
    if args.iter().any(|arg| arg == "--ai-client") {
        return Ok(ClientMode::AiClient);
    }
    if args.iter().any(|arg| arg == "--model-preview") {
        return Ok(ClientMode::ModelPreview);
    }
    if args.iter().any(|arg| arg == "--terrain-preview") {
        return Ok(ClientMode::TerrainPreview);
    }
    if args
        .iter()
        .any(|arg| arg == "--online" || arg.starts_with("--connect"))
    {
        return Ok(ClientMode::Online);
    }
    Ok(ClientMode::GameScene)
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    match select_mode(&args) {
        Ok(ClientMode::AiClient) => ai_client::run_ai_client(),
        Ok(ClientMode::ModelPreview) => model_preview::run_model_preview(),
        Ok(ClientMode::TerrainPreview) => terrain_preview::run_terrain_preview(),
        Ok(ClientMode::Online) => online::run_online_scene(),
        Ok(ClientMode::GameScene) => game_scene::run_game_scene(),
        Err(error) => {
            eprintln!("lk2-client: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_offline_flags_select_the_playable_scene() {
        assert_eq!(select_mode(&[]), Ok(ClientMode::GameScene));
        assert_eq!(
            select_mode(&["--offline".into(), "--no-scenario".into()]),
            Ok(ClientMode::GameScene)
        );
    }

    #[test]
    fn connect_flags_select_the_online_scene() {
        assert_eq!(
            select_mode(&["--connect=127.0.0.1:5000".into()]),
            Ok(ClientMode::Online)
        );
    }

    #[test]
    fn ai_client_flag_selects_the_headless_online_client() {
        assert_eq!(
            select_mode(&["--ai-client".into(), "--connect=127.0.0.1:5000".into()]),
            Ok(ClientMode::AiClient)
        );
    }

    #[test]
    fn codex_client_flag_selects_the_online_scene() {
        assert_eq!(
            select_mode(&["--codex-client".into()]),
            Ok(ClientMode::Online)
        );
        assert_eq!(
            select_mode(&["--codex-client".into(), "--connect=127.0.0.1:5000".into()]),
            Ok(ClientMode::Online)
        );
    }

    #[test]
    fn codex_client_can_explicitly_select_the_offline_scene() {
        assert_eq!(
            select_mode(&["--codex-client".into(), "--offline".into()]),
            Ok(ClientMode::GameScene)
        );
    }

    #[test]
    fn auto_demo_selects_the_game_scene() {
        assert_eq!(
            select_mode(&["--auto-demo".into()]),
            Ok(ClientMode::GameScene)
        );
    }
}
