mod game_scene;
mod model_preview;
mod nature;
mod online;
mod ray_aabb;
mod terrain_preview;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClientMode {
    GameScene,
    ModelPreview,
    TerrainPreview,
    Online,
}

fn select_mode(args: &[String]) -> Result<ClientMode, &'static str> {
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
    fn auto_demo_selects_the_game_scene() {
        assert_eq!(
            select_mode(&["--auto-demo".into()]),
            Ok(ClientMode::GameScene)
        );
    }
}
