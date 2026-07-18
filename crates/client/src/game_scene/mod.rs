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

use avian3d::prelude::{PhysicsDebugPlugin, PhysicsPlugins};
use avian3d::schedule::PhysicsSchedule;
use bevy::prelude::*;
use bevy::render::{
    RenderPlugin,
    settings::{Backends, WgpuSettings},
};
use bevy::window::{PresentMode, WindowResolution};
use bevy_hanabi::HanabiPlugin;
use bevy_tnua::prelude::{TnuaControllerPlugin, TnuaUserControlsSystems};
use bevy_tnua_avian3d::prelude::TnuaAvian3dPlugin;
use lk2_core::content::game_content_registry;

use crate::game_scene::animation::{
    animate_boss, animate_clouds_and_rain, animate_creature_hit_reactions,
    animate_exported_content_visuals, animate_grass_wind, animate_rabbits, animate_sun,
    animate_tree_sway, animate_water_fish, animate_water_seaweed, animate_wildlife, animate_wolves,
    bind_quadruped_visual_parts, grow_berries, grow_grass, grow_nature_plants,
    update_ground_after_rain,
};
use crate::game_scene::building::{
    explore_buildings, setup_explorable_buildings, update_building_prompt,
};
use crate::game_scene::capture::{exit_preview, maybe_take_screenshot};
use crate::game_scene::cave::{
    mine_cave_ore, setup_cave_realm, setup_mine_preview, update_cave_presentation,
};
use crate::game_scene::codex::{
    CodexSceneState, consume_action_pulses, observe_decide_act, stop_after_requested_runtime,
};
use crate::game_scene::content_visuals::living_content_layout_from_args;
use crate::game_scene::dimension::{
    animate_starfall_guardian, animate_starfall_shards, claim_starfall_relic,
    collect_starfall_shards, setup_dimension, toggle_dimension_travel,
    starfall_guardian_attack, update_dimension_presentation, update_starfall_guardian,
    update_starfall_relic,
};
use crate::game_scene::farm_ui::{
    FarmingUiState, handle_farming_actions, handle_farming_buttons, reconcile_farm_visuals,
    setup_farming_ui, toggle_farming_ui, update_farming_ui,
};
use crate::game_scene::hud::{
    setup_gameplay_hud, update_gameplay_hud, update_gameplay_hud_combat,
    update_gameplay_hud_target,
};
use crate::game_scene::inventory::{
    InventoryUiState, handle_inventory_buttons, setup_inventory_ui, toggle_inventory_ui,
    update_inventory_ui,
};
use crate::game_scene::keybindings::{
    KeyBindings, SettingsUiState, UiSettings, capture_keybinding_input, handle_keybinding_buttons,
    setup_keybinding_ui, toggle_keybinding_ui, update_keybinding_ui,
};
use crate::game_scene::offline::{
    OfflineNature, OfflineQuestPhase, advance_offline_nature, update_offline_quest,
};
use crate::game_scene::player::{
    PlayerControlScheme, advance_scene, camera_look_input, mine_surface_input,
    pickup_legendary_weapon, player_combat_controls, player_controls, recover_players_from_void,
    swim_player, sync_player_physics_state, toggle_cave_teleport, toggle_collision_debug,
    update_camera, update_collision_debug, update_cursor_capture, update_player_crouch,
    update_player_ik,
};
use crate::game_scene::procedural_motion::ProceduralAnimationConfig;
pub(crate) use crate::game_scene::procedural_motion::{GrassWind, WindField, grass_wind_rotation};
use crate::game_scene::reconcile::reconcile_nature_entities;
use crate::game_scene::settlement::{build_camp_input, restart_settlement_input};
pub(crate) use crate::game_scene::setup::build_grass_tuft_mesh;
use crate::game_scene::setup::{
    cleanup_retired_terrain_meshes, rebuild_edited_terrain, setup_camera, setup_collision_debug,
    setup_farm_plots, setup_lighting, setup_living_scene, setup_rendering, setup_terrain,
    setup_water_fish, update_underwater_presentation,
};
use crate::game_scene::state::{
    BuildingExplorationState, CaveTravelState, CodexSceneInput, CollisionDebugState,
    CreatureHitSettings, DimensionTravelState, LivingSceneState, ProceduralTerrainSurface,
    StarfallProgress, TerrainRebuildState, TerrainUndergroundState,
};
use crate::game_scene::targeting::{TargetingState, update_targeting};
pub(crate) use crate::game_scene::stylized_material::{
    GrassWindExtension, GrassWindMaterial, WaterSurfaceMaterial, animate_grass_gpu_wind,
    animate_water_surface,
};
use crate::game_scene::stylized_material::{StylizedTerrainMaterial, stylize_asset_materials};
use crate::game_scene::ui_drag::{UiDragState, drag_ui_panels};
use crate::game_scene::vehicle::{
    CartRideState, setup_cart, sync_cart_to_rider, sync_cart_visibility, toggle_cart_ride,
};
use crate::game_scene::world_debug::{
    WorldDebugUiState, setup_world_debug_ui, toggle_world_debug_ui, update_world_debug_ui,
};
use lk2_core::legendary::LegendaryLoadout;

use crate::crisp_image_plugin;
use crate::milestone_capture::MilestoneCapturePlugin;

mod animation;
mod building;
mod capture;
mod cave;
mod codex;
mod combat_fx;
mod content_visuals;
mod creature_ai;
mod dimension;
mod farm_ui;
mod hud;
mod inventory;
mod keybindings;
mod offline;
mod player;
mod procedural_motion;
mod procedural_rig;
mod reconcile;
mod settlement;
mod setup;
mod state;
mod stylized_material;
mod targeting;
mod ui_drag;
mod util;
pub(crate) use util::CART_FRONT_YAW_OFFSET;
pub(crate) use util::MAIN_APP_MODEL_PATHS;
mod vehicle;
mod world_debug;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LivingGameplaySet {
    Core,
    Tail,
}

#[cfg(test)]
mod tests;

pub fn run_game_scene() {
    let args = std::env::args().collect::<Vec<_>>();
    let codex_mode = args.iter().any(|arg| arg == "--codex-client");
    let auto_demo = args.iter().any(|arg| arg == "--auto-demo");
    let boss_scene_shot = args.iter().any(|arg| arg == "--boss-scene-shot");
    let auto_shot = auto_demo
        || args.iter().any(|arg| arg == "--game-scene-shot")
        || boss_scene_shot;
    let iter_dir = auto_demo
        .then(|| std::env::var_os("LK2_ITER_DIR").map(PathBuf::from))
        .flatten();
    let output_dir = iter_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("screenshots/game_scene"));
    let _ = std::fs::create_dir_all(&output_dir);
    let png_path = if boss_scene_shot && iter_dir.is_none() {
        output_dir.join("boss_scene.png")
    } else {
        screenshot_path(&output_dir, iter_dir.as_deref())
    };
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
            .set(crisp_image_plugin())
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: Some(Backends::VULKAN),
                    ..default()
                }
                .into(),
                ..default()
            })
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
    app.add_plugins((
        PhysicsPlugins::default(),
        PhysicsDebugPlugin,
        TnuaControllerPlugin::<PlayerControlScheme>::new(PhysicsSchedule),
        TnuaAvian3dPlugin::new(PhysicsSchedule),
        HanabiPlugin,
        MilestoneCapturePlugin,
    ));
    app.add_plugins(MaterialPlugin::<StylizedTerrainMaterial>::default());
    app.add_plugins(MaterialPlugin::<GrassWindMaterial>::default());
    app.add_plugins(MaterialPlugin::<WaterSurfaceMaterial>::default());
    app.insert_resource(LivingSceneState {
        elapsed: 0.0,
        frame: 0,
        auto_shot,
        shot_requested: false,
        exit_deadline: None,
        png_path,
        attack_flash: 0.0,
        camera_shake: 0.0,
        auto_demo,
        iter_dir,
        frame_dt_over_50ms: 0,
        frame_dt_max_ms: 0.0,
    })
    .insert_resource(ProceduralTerrainSurface::default_world())
    .insert_resource(CollisionDebugState {
        enabled: args.iter().any(|arg| arg == "--collision-debug"),
    })
    .insert_resource(game_content_registry())
    .insert_resource(living_content_layout_from_args(&args))
    .insert_resource(OfflineNature::playable())
    .init_resource::<OfflineQuestPhase>()
    .insert_resource(WindField::default())
    .init_resource::<ProceduralAnimationConfig>()
    .init_resource::<CreatureHitSettings>()
    .insert_resource(LegendaryLoadout::default())
    .init_resource::<BuildingExplorationState>()
    .init_resource::<KeyBindings>()
    .init_resource::<UiSettings>()
    .init_resource::<SettingsUiState>()
    .init_resource::<InventoryUiState>()
    .init_resource::<FarmingUiState>()
    .init_resource::<UiDragState>()
    .init_resource::<CartRideState>()
    .init_resource::<TerrainRebuildState>()
    .init_resource::<TerrainUndergroundState>()
    .init_resource::<CaveTravelState>()
    .init_resource::<DimensionTravelState>()
    .init_resource::<StarfallProgress>()
    .init_resource::<TargetingState>()
    .insert_resource(WorldDebugUiState {
        open: args.iter().any(|arg| arg == "--content-debug-open"),
    });
    app.configure_sets(
        Update,
        (
            LivingGameplaySet::Core,
            LivingGameplaySet::Tail.after(LivingGameplaySet::Core),
        ),
    );
    app.add_systems(
        Startup,
        (
            setup_rendering,
            setup_lighting,
            setup_farm_plots,
            setup_terrain,
            setup_collision_debug,
            setup_living_scene,
            setup_water_fish,
            setup_explorable_buildings,
            setup_cave_realm,
            setup_camera,
            setup_mine_preview,
            setup_keybinding_ui,
            setup_inventory_ui,
            setup_farming_ui,
            setup_gameplay_hud,
            setup_world_debug_ui,
            setup_cart,
            setup_dimension,
        )
            .chain(),
    );
    app.add_systems(
        PhysicsSchedule,
        player_controls.in_set(TnuaUserControlsSystems),
    );
    app.add_systems(
        Update,
        animate_sun
            .after(advance_scene)
            .before(maybe_take_screenshot),
    );
    app.add_systems(
        Update,
        (
            observe_decide_act.run_if(resource_exists::<CodexSceneState>),
            advance_scene,
            advance_offline_nature,
            stylize_asset_materials,
            reconcile_nature_entities,
            camera_look_input,
            swim_player,
            update_underwater_presentation,
            sync_player_physics_state,
            recover_players_from_void,
            pickup_legendary_weapon,
            explore_buildings,
            toggle_cave_teleport,
            toggle_dimension_travel,
            collect_starfall_shards,
            claim_starfall_relic,
            starfall_guardian_attack,
            player_combat_controls,
            mine_cave_ore,
            mine_surface_input,
        )
            .chain()
            .in_set(LivingGameplaySet::Core),
    );
    app.add_systems(
        Update,
        (
            build_camp_input,
            restart_settlement_input,
            update_offline_quest,
        )
            .chain()
            .in_set(LivingGameplaySet::Tail),
    );
    app.add_systems(
        Update,
        (
            consume_action_pulses.run_if(resource_exists::<CodexSceneInput>),
            rebuild_edited_terrain,
            cleanup_retired_terrain_meshes,
            toggle_cart_ride,
            sync_cart_to_rider,
            sync_cart_visibility,
        )
            .chain()
            .after(LivingGameplaySet::Tail),
    );
    app.add_systems(
        Update,
        (
            toggle_collision_debug,
            toggle_keybinding_ui,
            handle_keybinding_buttons,
            capture_keybinding_input,
            toggle_inventory_ui,
            handle_inventory_buttons,
            toggle_farming_ui,
            handle_farming_buttons,
            handle_farming_actions,
            toggle_world_debug_ui,
            update_player_crouch,
            update_cursor_capture,
            drag_ui_panels,
        )
            .chain()
            .after(LivingGameplaySet::Tail),
    );
    app.add_systems(
        Update,
        (
            bind_quadruped_visual_parts,
            update_player_ik,
            update_collision_debug,
            animate_exported_content_visuals,
            animate_clouds_and_rain,
            grow_grass,
            animate_grass_wind,
            animate_grass_gpu_wind,
            animate_water_surface,
            grow_berries,
            grow_nature_plants,
            animate_rabbits,
            animate_wolves,
            animate_wildlife,
            animate_creature_hit_reactions,
            reconcile_farm_visuals,
        )
            .chain()
            .after(update_cursor_capture)
            .after(reconcile_nature_entities),
    );
    app.add_systems(
        Update,
        (
            animate_boss,
            animate_tree_sway,
            animate_water_fish,
            animate_water_seaweed,
            update_camera,
            update_ground_after_rain,
            update_targeting,
            maybe_take_screenshot,
        )
            .chain()
            .after(bind_quadruped_visual_parts)
            .after(reconcile_farm_visuals),
    );
    app.add_systems(
        Update,
        (
            update_keybinding_ui,
            update_inventory_ui,
            update_farming_ui,
            update_building_prompt,
            update_gameplay_hud,
            update_gameplay_hud_combat,
            update_gameplay_hud_target,
            update_world_debug_ui,
            update_dimension_presentation,
            update_starfall_guardian,
            update_cave_presentation,
            update_starfall_relic,
            animate_starfall_guardian,
            animate_starfall_shards,
            exit_preview,
            stop_after_requested_runtime.run_if(resource_exists::<CodexSceneState>),
        )
            .chain()
            .after(maybe_take_screenshot),
    );
    if codex_mode {
        app.insert_resource(CodexSceneInput::default())
            .insert_resource(CodexSceneState::from_args(&args));
        crate::codex_client::install_codex_worker(&mut app, &args);
    }
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
