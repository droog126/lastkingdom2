use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy::window::WindowResolution;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin { file_path: "../../assets".into(), ..default() })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "audit_player".into(),
                        resolution: WindowResolution::new(800, 600),
                        present_mode: PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin { level: bevy::log::Level::INFO, ..default() }),
        )
        .add_systems(Startup, spawn_player)
        .add_systems(Update, exit_after)
        .run();
}

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("=== audit_player: spawn sokpop_gatherer.glb at (0, 0, 0) ===");
    let scene = asset_server
        .load(GltfAssetLabel::Scene(0).from_asset("procedural/pretty/sokpop_gatherer.glb"));
    commands.spawn((
        WorldAssetRoot(scene),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)).with_scale(Vec3::splat(20.0)),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(3.0, 3.0, 3.0)).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_translation(Vec3::new(1.0, 2.0, 1.0)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    info!("spawned");
}

fn exit_after(time: Res<Time>) {
    if time.elapsed_secs() > 5.0 {
        std::process::exit(0);
    }
}
