//! Focused playable vertical slice for the living forest scene.
//!
//! Run:
//!   cargo run -p lk2-client
//!   cargo run -p lk2-client -- --game-scene-shot
//!
//! The module is split into focused submodules that each own a single
//! responsibility:
//!
//! - [`state`]      — Bevy resources and marker components.
//! - [`offline`]    — offline authority that owns the live `EcoCycle`.
//! - [`setup`]      — one-shot `Startup` systems (rendering, lighting,
//!   terrain, living scene, camera).
//! - [`reconcile`]  — diff between snapshot and visible entities.
//! - [`player`]     — player input, follow camera, per-frame scene clock.
//! - [`animation`]  — per-frame animation systems.
//! - [`capture`]    — screenshot and shutdown logic.
//! - [`util`]       — asset paths, scene constants, small helpers.
//! - [`tests`]      — unit tests.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};

use crate::game_scene::animation::{
    animate_boss, animate_clouds_and_rain, animate_rabbits, animate_tree_sway, grow_berries,
    grow_grass, update_ground_after_rain,
};
use crate::game_scene::capture::{exit_preview, maybe_take_screenshot};
use crate::game_scene::content_visuals::living_content_layout_from_args;
use crate::game_scene::keybindings::{
    capture_keybinding_input, handle_keybinding_buttons, setup_keybinding_ui, toggle_keybinding_ui,
    update_keybinding_ui, KeyBindings, UiSettings,
};
use crate::game_scene::offline::{advance_offline_nature, OfflineNature};
use crate::game_scene::player::{
    advance_scene, camera_look_input, player_controls, update_camera, update_player_ik,
};
use crate::game_scene::reconcile::reconcile_nature_entities;
use crate::game_scene::setup::{
    setup_camera, setup_lighting, setup_living_scene, setup_rendering, setup_terrain,
};
use crate::game_scene::state::LivingSceneState;

mod animation;
mod capture;
mod content_visuals;
mod creature_ai;
mod keybindings;
mod offline;
mod player;
mod procedural_motion;
mod procedural_rig;
mod reconcile;
mod setup;
mod state;
mod util;

#[cfg(test)]
mod tests;

pub fn run_game_scene() {
    let args = std::env::args().collect::<Vec<_>>();
    let auto_demo = args.iter().any(|arg| arg == "--auto-demo");
    let auto_shot = auto_demo || args.iter().any(|arg| arg == "--game-scene-shot");
    let iter_dir = auto_demo
        .then(|| std::env::var_os("LK2_ITER_DIR").map(PathBuf::from))
        .flatten();
    let output_dir = iter_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("screenshots/game_scene"));
    let _ = std::fs::create_dir_all(&output_dir);
    let png_path = screenshot_path(&output_dir, iter_dir.as_deref());
    if auto_shot {
        let _ = std::fs::remove_file(&png_path);
    }

    let asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets");
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: asset_root.to_string_lossy().into_owned(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Last Kingdom - Living Forest".into(),
                    resolution: WindowResolution::new(1440, 900),
                    present_mode: PresentMode::Immediate,
                    focused: true,
                    visible: true,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::log::LogPlugin {
                level: bevy::log::Level::INFO,
                ..default()
            }),
    );
    app.insert_resource(LivingSceneState {
        elapsed: 0.0,
        frame: 0,
        auto_shot,
        shot_requested: false,
        exit_deadline: None,
        png_path,
        attack_flash: 0.0,
        auto_demo,
        iter_dir,
        frame_dt_over_50ms: 0,
        frame_dt_max_ms: 0.0,
    })
    .insert_resource(living_content_layout_from_args(&args))
    .init_resource::<OfflineNature>()
    .init_resource::<KeyBindings>()
    .init_resource::<UiSettings>();
    app.add_systems(
        Startup,
        (
            setup_rendering,
            setup_lighting,
            setup_terrain,
            setup_living_scene,
            setup_camera,
            setup_keybinding_ui,
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (
            advance_scene,
            advance_offline_nature,
            reconcile_nature_entities,
            camera_look_input,
            player_controls,
            toggle_keybinding_ui,
            handle_keybinding_buttons,
            capture_keybinding_input,
            update_player_ik,
            animate_clouds_and_rain,
            grow_grass,
            grow_berries,
            animate_rabbits,
            animate_boss,
            animate_tree_sway,
            update_camera,
            update_ground_after_rain,
            maybe_take_screenshot,
            update_keybinding_ui,
            exit_preview,
        )
            .chain(),
    );
    app.run();
}

fn screenshot_path(output_dir: &Path, iter_dir: Option<&Path>) -> PathBuf {
    if let Some(iter_dir) = iter_dir {
        let stem = iter_dir
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| name.starts_with("iter_"))
            .unwrap_or("iter_capture");
        return iter_dir.join(format!("{stem}.png"));
    }
    output_dir.join("living_forest.png")
}
