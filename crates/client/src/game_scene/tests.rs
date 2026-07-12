//! Tests for the offline authority and the cloud/rain animation schedule.

use std::path::PathBuf;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::animation::{
    RabbitFoodTarget, animate_boss, animate_clouds_and_rain, animate_rabbits, animate_sun,
    animate_tree_sway, animate_wolves, living_sun_transform, rabbit_ai_decision,
};
use super::content_visuals::{
    DEFAULT_LIVING_CONTENT_SEED, LIVING_CONTENT_DIMENSIONS, MONSTER_ANCHOR_PILLAR_SCALE,
    MONSTER_DECORATIVE_MARKER_SCALE, content_cell_position, grounded_content_position,
    is_monster_anchor, living_content_layout_from_args, parse_content_profile, parse_content_seed,
};
use super::creature_ai::{AiContext, AiMood, AiTarget, CreatureAiProfile, decide_creature_intent};
use super::inventory::InventoryUiState;
use super::keybindings::{Binding, GameAction, KeyBindings};
use super::offline::OfflineNature;
use super::player::{
    PlayerControlScheme, camera_look_input, pickup_dragon_katana, player_combat_controls,
    player_controls, update_cursor_capture,
};
use super::procedural_motion::{ProceduralTreeSway, heading_yaw, hop_height, smooth_follow_alpha};
use super::procedural_rig::{
    DragonRig, DragonTargets, HumanoidRig, HumanoidTargets, QuadrupedRig, QuadrupedTargets,
    TwoBoneLimb, TwoBoneLimbSpec, TwoBoneSolution, apply_segment_between,
};
use super::state::{
    BossActor, Cloud, DragonKatanaPickup, GrassTuft, HitReaction, LivingCameraRig,
    LivingSceneState, LivingSun, PlayerActor, PlayerIkPart, PlayerIkPartKind, PlayerJump,
    PlayerMotion, PlayerSkillState, ProceduralTerrainSurface, Rabbit, RabbitAi, RabbitMood,
    RainDrop, SceneMaterials, SlashFx, Wolf, WolfAi,
};
use super::util::hash01;
use super::world_debug::{WorldDebugUiState, content_debug_text, toggle_world_debug_ui};
use bevy_tnua::prelude::TnuaController;
use lk2_core::constant;
use lk2_core::legendary::{LegendaryLoadout, LegendaryWeapon};
use lk2_core::pvp::{Health, PvpCombatant, SimpleWeapon};
use lk2_core::world::content::{
    ContentSpiceProfile, GAME_CONTENT_MONSTER_TERRITORY, GAME_CONTENT_SETTLEMENT,
};

fn test_scene_state() -> LivingSceneState {
    LivingSceneState {
        elapsed: 0.0,
        frame: 0,
        auto_shot: false,
        shot_requested: false,
        exit_deadline: None,
        png_path: PathBuf::new(),
        attack_flash: 0.0,
        camera_shake: 0.0,
        auto_demo: false,
        iter_dir: None,
        frame_dt_over_50ms: 0,
        frame_dt_max_ms: 0.0,
    }
}

#[test]
fn screenshot_path_matches_loop_iter_contract() {
    let iter_dir = PathBuf::from("screenshots/iter_12");
    assert_eq!(
        super::screenshot_path(&iter_dir, Some(&iter_dir)),
        PathBuf::from("screenshots/iter_12/iter_12.png")
    );
    assert_eq!(
        super::screenshot_path(&PathBuf::from("screenshots/game_scene"), None),
        PathBuf::from("screenshots/game_scene/living_forest.png")
    );
}

#[test]
fn content_seed_and_profile_parse_from_game_scene_args() {
    let args = vec![
        "lk2-client".to_string(),
        "--content-seed=0x2A".to_string(),
        "--content-profile=monster".to_string(),
    ];

    assert_eq!(parse_content_seed(&args), 42);
    assert_eq!(
        parse_content_profile(&args),
        Some(ContentSpiceProfile::MonsterMarch)
    );
    assert_eq!(parse_content_seed(&[]), DEFAULT_LIVING_CONTENT_SEED);
}

#[test]
fn f2_opens_content_debug_by_default() {
    let bindings = KeyBindings::default();

    assert_eq!(
        bindings.binding(GameAction::OpenWorldStatus),
        Binding::Key(KeyCode::F2)
    );
}

#[test]
fn f2_toggles_content_debug_panel_state() {
    let mut app = App::new();
    app.insert_resource(KeyBindings::default())
        .insert_resource(WorldDebugUiState::default())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(ButtonInput::<MouseButton>::default())
        .add_systems(Update, toggle_world_debug_ui);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F2);
    app.update();

    assert!(app.world().resource::<WorldDebugUiState>().open);
}

#[test]
fn content_layout_maps_volume_cells_to_scene_space() {
    let center = content_cell_position(LIVING_CONTENT_DIMENSIONS, [2, 1, 2]);
    let corner = content_cell_position(LIVING_CONTENT_DIMENSIONS, [0, 1, 0]);

    assert_eq!(center, Vec3::new(0.0, 0.04, 0.0));
    assert!(corner.x < center.x);
    assert!(corner.z < center.z);
}

#[test]
fn content_visuals_follow_the_procedural_ground() {
    let terrain = ProceduralTerrainSurface::default_world();
    let raw = content_cell_position(LIVING_CONTENT_DIMENSIONS, [2, 1, 2]);
    let grounded = grounded_content_position(&terrain, raw);

    assert!(grounded.y > raw.y);
    assert_eq!(grounded.x, raw.x);
    assert_eq!(grounded.z, raw.z);
}

#[test]
fn living_content_layout_uses_spice_profiles() {
    let args = vec![
        "lk2-client".to_string(),
        "--content-seed=99".to_string(),
        "--content-profile=monster-march".to_string(),
    ];
    let layout = living_content_layout_from_args(&args);

    assert_eq!(layout.profile, ContentSpiceProfile::MonsterMarch);
    assert_eq!(layout.volume.get([2, 1, 2]), Some(GAME_CONTENT_SETTLEMENT));
    assert!(
        layout.volume.count(GAME_CONTENT_MONSTER_TERRITORY) > 4,
        "monster profile should create a readable monster territory"
    );
}

#[test]
fn content_debug_text_shows_profile_grid_and_pillar_rule() {
    let args = vec![
        "lk2-client".to_string(),
        "--content-seed=99".to_string(),
        "--content-profile=monster".to_string(),
    ];
    let layout = living_content_layout_from_args(&args);
    let text = content_debug_text(&layout);

    assert!(text.contains("配置 MonsterMarch"));
    assert!(text.contains("地表 y=1"));
    assert!(text.contains("地下 y=0"));
    assert!(text.contains("大型锚点 [1,1,1]"));
    assert!(text.contains("A "));
}

#[test]
fn only_required_monster_cell_is_a_large_pillar_anchor() {
    assert!(is_monster_anchor([1, 1, 1]));
    assert!(!is_monster_anchor([0, 1, 1]));
    assert!(!is_monster_anchor([2, 1, 2]));
    assert!(MONSTER_ANCHOR_PILLAR_SCALE > MONSTER_DECORATIVE_MARKER_SCALE * 5.0);
    assert!(MONSTER_DECORATIVE_MARKER_SCALE <= 0.1);
}

#[test]
fn cloud_and_rain_schedule_runs_without_query_conflicts() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .insert_resource(test_scene_state())
        .init_resource::<OfflineNature>()
        .add_systems(Update, animate_clouds_and_rain);
    app.world_mut().spawn((
        Cloud {
            snapshot_index: 0,
            phase: 0.0,
        },
        Transform::default(),
    ));
    app.world_mut().spawn((
        RainDrop {
            cloud_index: 0,
            local: Vec3::ZERO,
            phase: 0.0,
        },
        Transform::default(),
    ));

    app.update();
}

#[test]
fn boss_and_slash_animation_queries_are_disjoint() {
    let mut app = App::new();
    app.insert_resource(test_scene_state())
        .add_systems(Update, animate_boss);
    app.world_mut().spawn((
        BossActor { base: Vec3::ZERO },
        Health::default(),
        Transform::default(),
    ));
    app.world_mut()
        .spawn((SlashFx, Transform::from_scale(Vec3::splat(0.2))));

    app.update();
}

#[test]
fn tree_sway_update_runs_without_query_conflicts() {
    let mut app = App::new();
    app.insert_resource(test_scene_state())
        .add_systems(Update, animate_tree_sway);
    app.world_mut().spawn((
        ProceduralTreeSway {
            base_translation: Vec3::new(1.0, 0.0, 2.0),
            base_yaw: 0.4,
            base_scale: 1.2,
            phase: 0.7,
            strength: 0.02,
        },
        Transform::default(),
    ));

    app.update();
}

#[test]
fn living_sun_transform_rotates_around_scene_center() {
    let initial = living_sun_transform(0.0);
    let later = living_sun_transform(14.0);

    assert!(initial.translation.distance(Vec3::new(-24.0, 38.0, 18.0)) < 0.001);
    assert_eq!(initial.translation.y, 38.0);
    assert_eq!(later.translation.y, 38.0);
    assert!(initial.translation.distance(later.translation) > 20.0);
    assert!(initial.rotation.dot(later.rotation).abs() < 0.98);
}

#[test]
fn sun_animation_update_runs_without_query_conflicts() {
    let mut app = App::new();
    let mut state = test_scene_state();
    state.elapsed = 7.0;
    app.insert_resource(state).add_systems(Update, animate_sun);
    app.world_mut()
        .spawn((LivingSun, Transform::from_xyz(-24.0, 38.0, 18.0)));

    app.update();

    let transform = app
        .world_mut()
        .query::<&Transform>()
        .single(app.world())
        .expect("sun should remain spawned");
    assert!(
        transform
            .translation
            .distance(living_sun_transform(7.0).translation)
            < 0.001
    );
}

#[test]
fn procedural_motion_helpers_keep_expected_bounds() {
    assert_eq!(hop_height(std::f32::consts::PI, 2.0), 0.0);
    assert!((smooth_follow_alpha(0.0, 8.0) - 0.0).abs() < f32::EPSILON);
    assert!(smooth_follow_alpha(0.25, 8.0) > smooth_follow_alpha(0.25, 2.0));
    assert!(
        (heading_yaw(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 0.0) - std::f32::consts::FRAC_PI_2)
            .abs()
            < 0.001
    );
}

#[test]
fn player_ik_update_runs_without_query_conflicts() {
    let mut app = App::new();
    app.insert_resource(test_scene_state())
        .insert_resource(LivingCameraRig::default())
        .insert_resource(ProceduralTerrainSurface::default_world())
        .add_systems(Update, super::player::update_player_ik);
    app.world_mut().spawn((
        PlayerActor,
        PlayerMotion::default(),
        PlayerJump::default(),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    app.world_mut().spawn((
        PlayerIkPart {
            kind: PlayerIkPartKind::ArmUpperR,
        },
        Transform::default(),
        Visibility::Visible,
    ));

    app.update();
}

#[test]
fn rabbit_ai_update_runs_without_query_conflicts() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .insert_resource(test_scene_state())
        .init_resource::<OfflineNature>()
        .add_systems(Update, animate_rabbits);
    app.world_mut()
        .spawn((PlayerActor, Transform::from_xyz(1.0, 0.0, 0.0)));
    app.world_mut().spawn((
        Rabbit { id: 1, phase: 0.0 },
        RabbitAi::default(),
        Transform::default(),
    ));
    app.world_mut().spawn((
        GrassTuft {
            growth_threshold: 0.0,
            mature_scale: Vec3::ONE,
        },
        Transform::from_xyz(2.0, 0.0, 0.0).with_scale(Vec3::splat(1.0)),
    ));

    app.update();
}

#[test]
fn wolf_animation_update_runs_without_query_conflicts() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .insert_resource(test_scene_state())
        .init_resource::<OfflineNature>()
        .add_systems(Update, animate_wolves);
    app.world_mut().spawn((
        Rabbit { id: 1, phase: 0.0 },
        Transform::from_xyz(2.0, 0.0, 0.0),
    ));
    app.world_mut().spawn((
        Wolf { id: 0, phase: 0.0 },
        WolfAi::default(),
        Transform::default(),
    ));

    app.update();
}

#[test]
fn space_feeds_tnua_without_attacking_and_left_click_attacks() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .insert_resource(test_scene_state())
        .insert_resource(super::state::SceneMaterials {
            ground: Handle::default(),
            hit_effect: Handle::default(),
        })
        .insert_resource(ProceduralTerrainSurface::default_world())
        .insert_resource(LivingCameraRig::default())
        .insert_resource(InventoryUiState::default())
        .insert_resource(KeyBindings::default())
        .insert_resource(ButtonInput::<MouseButton>::default())
        .add_systems(Update, (player_controls, player_combat_controls).chain());
    app.world_mut().spawn((
        PlayerActor,
        PlayerJump::default(),
        PlayerMotion::default(),
        PvpCombatant::default(),
        SimpleWeapon::default(),
        TnuaController::<PlayerControlScheme>::default(),
        Transform::default(),
    ));

    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::Space);
    app.insert_resource(keys);
    app.update();

    let (controller, combatant) = app
        .world_mut()
        .query::<(&TnuaController<PlayerControlScheme>, &PvpCombatant)>()
        .single(app.world())
        .expect("player should remain spawned");
    assert_eq!(controller.basis.desired_motion, Vec3::ZERO);
    assert_eq!(combatant.cooldown_remaining, 0.0);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();

    let combatant = app
        .world_mut()
        .query::<&PvpCombatant>()
        .single(app.world())
        .expect("player should remain spawned");
    assert!(combatant.cooldown_remaining > 0.0);
}

#[test]
fn successful_melee_hit_triggers_player_and_target_feedback() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .insert_resource(test_scene_state())
        .insert_resource(SceneMaterials {
            ground: Handle::default(),
            hit_effect: Handle::default(),
        })
        .insert_resource(InventoryUiState::default())
        .insert_resource(KeyBindings::default())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(ButtonInput::<MouseButton>::default())
        .add_systems(Update, player_combat_controls);
    app.world_mut().spawn((
        PlayerActor,
        PvpCombatant::default(),
        SimpleWeapon::default(),
        Transform::default(),
    ));
    app.world_mut().spawn((
        BossActor {
            base: Vec3::new(0.0, 0.0, 2.0),
        },
        HitReaction::default(),
        Health {
            current: 24.0,
            max: 24.0,
            invuln_until_tick: 0,
        },
        Transform::from_xyz(0.0, 0.0, 2.0),
    ));

    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();

    let health = app
        .world_mut()
        .query_filtered::<&Health, With<BossActor>>()
        .single(app.world())
        .expect("boss should remain spawned");
    assert!(health.current < health.max);
    let reaction = app
        .world_mut()
        .query_filtered::<&HitReaction, With<BossActor>>()
        .single(app.world())
        .expect("boss should have a hit reaction");
    assert!(reaction.timer > 0.0);
    assert!(app.world().resource::<LivingSceneState>().camera_shake > 0.0);
    let impact_count = {
        let world = app.world_mut();
        world
            .query::<&super::state::HitImpactFx>()
            .iter(world)
            .count()
    };
    assert_eq!(impact_count, 1);
}

#[test]
fn riverbank_sand_is_walkable_but_subsea_surface_is_water() {
    let terrain = ProceduralTerrainSurface::default_world();
    let riverbank = (0..96)
        .flat_map(|z| (0..96).map(move |x| (x, z)))
        .find_map(|(x, z)| {
            let surface = terrain.pipeline.surface_f32(x, z)?;
            (surface > 12.0 && terrain.landform.river_factor(x, z) > 0.72)
                .then_some((x, z, surface))
        })
        .expect("default terrain should contain an above-sea riverbank sample");
    assert!(!terrain.is_water_at(riverbank.0, riverbank.1, riverbank.2));

    let riverbed = (0..96)
        .flat_map(|z| (0..96).map(move |x| (x, z)))
        .find_map(|(x, z)| {
            let surface = terrain.pipeline.surface_f32(x, z)?;
            (surface < 12.0 && terrain.landform.river_factor(x, z) > 0.72)
                .then_some((x, z, surface))
        })
        .expect("default terrain should contain a below-sea river sample");
    assert!(terrain.is_water_at(riverbed.0, riverbed.1, riverbed.2));
}

#[test]
fn terrain_collision_height_does_not_jump_at_grid_boundaries() {
    let terrain = ProceduralTerrainSurface::default_world();
    let before = terrain.ground_height(Vec3::new(-0.001, 0.0, 0.7));
    let after = terrain.ground_height(Vec3::new(0.001, 0.0, 0.7));

    assert!(
        (before - after).abs() < 0.02,
        "collision height jumped across a grid boundary: before={before}, after={after}"
    );
}

#[test]
fn c_toggles_camera_mode() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .insert_resource(LivingCameraRig::default())
        .insert_resource(InventoryUiState::default())
        .insert_resource(KeyBindings::default())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(ButtonInput::<MouseButton>::default())
        .add_message::<MouseMotion>()
        .add_systems(Update, camera_look_input);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyC);
    app.update();
    assert_eq!(
        app.world().resource::<LivingCameraRig>().mode,
        super::state::CameraMode::ThirdPerson
    );

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyC);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyC);
    app.update();
    assert_eq!(
        app.world().resource::<LivingCameraRig>().mode,
        super::state::CameraMode::FirstPerson
    );
}

#[test]
fn third_person_camera_pitch_changes_orbit_height() {
    let target = Vec3::new(2.0, 1.0, -3.0);
    let looking_down = super::player::third_person_camera_position(target, 0.4, -0.9);
    let looking_up = super::player::third_person_camera_position(target, 0.4, 0.7);

    assert!((looking_down.y - looking_up.y).abs() > 1.0);
    assert!(looking_down.y > looking_up.y);
}

#[test]
fn inventory_open_blocks_camera_input() {
    let mut inventory = InventoryUiState::default();
    inventory.open = true;

    let mut app = App::new();
    app.init_resource::<Time>()
        .insert_resource(LivingCameraRig::default())
        .insert_resource(inventory)
        .insert_resource(KeyBindings::default())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(ButtonInput::<MouseButton>::default())
        .add_message::<MouseMotion>()
        .add_systems(Update, camera_look_input);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyC);
    app.update();

    assert_eq!(
        app.world().resource::<LivingCameraRig>().mode,
        super::state::CameraMode::FirstPerson
    );
}

#[test]
fn cursor_capture_follows_ui_state() {
    let mut app = App::new();
    app.insert_resource(KeyBindings::default())
        .insert_resource(InventoryUiState::default())
        .add_systems(Update, update_cursor_capture);
    app.world_mut()
        .spawn((CursorOptions::default(), PrimaryWindow));

    app.update();
    {
        let cursor_options = app
            .world_mut()
            .query_filtered::<&CursorOptions, With<PrimaryWindow>>()
            .single(app.world())
            .expect("primary cursor options should exist");
        assert!(!cursor_options.visible);
        assert_eq!(cursor_options.grab_mode, CursorGrabMode::Locked);
    }

    app.world_mut().resource_mut::<InventoryUiState>().open = true;
    app.update();

    let cursor_options = app
        .world_mut()
        .query_filtered::<&CursorOptions, With<PrimaryWindow>>()
        .single(app.world())
        .expect("primary cursor options should exist");
    assert!(cursor_options.visible);
    assert_eq!(cursor_options.grab_mode, CursorGrabMode::None);
}

#[test]
fn two_bone_solver_keeps_limb_lengths_reachable() {
    let root = Vec3::new(0.0, 1.0, 0.0);
    let target = Vec3::new(0.45, 0.55, -0.15);
    let solution = TwoBoneLimb {
        root,
        target,
        pole: Vec3::new(0.2, 0.8, -0.5),
        upper_len: 0.38,
        lower_len: 0.42,
    }
    .solve();

    assert!((solution.joint.distance(root) - 0.38).abs() < 0.001);
    assert!((solution.joint.distance(target) - 0.42).abs() < 0.001);
}

#[test]
fn humanoid_rig_solves_all_player_avatar_limbs() {
    let rig = HumanoidRig::player_avatar();
    let solved = rig.solve(HumanoidTargets {
        left_hand: Vec3::new(-0.40, 0.58, 0.08),
        right_hand: Vec3::new(0.40, 0.58, 0.08),
        left_foot: Vec3::new(-0.15, 0.06, 0.03),
        right_foot: Vec3::new(0.15, 0.06, 0.03),
        left_elbow_pole: Vec3::new(-0.58, 0.80, 0.32),
        right_elbow_pole: Vec3::new(0.58, 0.80, 0.32),
        left_knee_pole: Vec3::new(-0.27, 0.25, 0.30),
        right_knee_pole: Vec3::new(0.27, 0.25, 0.30),
    });

    for (spec, solution) in [
        (rig.left_arm, solved.left_arm),
        (rig.right_arm, solved.right_arm),
        (rig.left_leg, solved.left_leg),
        (rig.right_leg, solved.right_leg),
    ] {
        assert_eq!(solution.root, spec.root);
        assert!((solution.joint.distance(solution.root) - spec.upper_len).abs() < 0.001);
        assert!((solution.joint.distance(solution.target) - spec.lower_len).abs() < 0.001);
    }
}

#[test]
fn idle_humanoid_targets_are_near_full_extension() {
    let motion = PlayerMotion {
        grounded: true,
        ..default()
    };
    let pose = super::player::procedural_player_pose(
        0.0,
        0.0,
        &motion,
        &PlayerJump::default(),
        false,
        0.0,
    );
    let rig = HumanoidRig::player_avatar();
    for (root, target, total) in [
        (
            rig.left_arm.root,
            pose.left_hand,
            rig.left_arm.upper_len + rig.left_arm.lower_len,
        ),
        (
            rig.right_arm.root,
            pose.right_hand,
            rig.right_arm.upper_len + rig.right_arm.lower_len,
        ),
        (
            rig.left_leg.root,
            pose.left_foot,
            rig.left_leg.upper_len + rig.left_leg.lower_len,
        ),
        (
            rig.right_leg.root,
            pose.right_foot,
            rig.right_leg.upper_len + rig.right_leg.lower_len,
        ),
    ] {
        assert!(
            root.distance(target) >= total * 0.92,
            "idle limb target is too short for a readable straight silhouette"
        );
    }
}

#[test]
fn dragon_skill_cooldown_is_reusable_after_tick() {
    let mut skill = PlayerSkillState::default();
    assert!(skill.begin(4.0));
    assert!(!skill.begin(4.0));
    skill.tick(4.0);
    assert!(skill.ready());
}

#[test]
fn interact_near_dragon_katana_equips_it() {
    let mut app = App::new();
    app.insert_resource(KeyBindings::default())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(ButtonInput::<MouseButton>::default())
        .insert_resource(LegendaryLoadout::default())
        .add_systems(Update, pickup_dragon_katana);
    app.world_mut()
        .spawn((PlayerActor, Transform::default(), SimpleWeapon::default()));
    app.world_mut()
        .spawn((DragonKatanaPickup, Transform::from_xyz(1.0, 0.0, 0.0)));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyF);
    app.update();

    assert_eq!(
        app.world().resource::<LegendaryLoadout>().equipped,
        Some(LegendaryWeapon::DragonKatana)
    );
    let (damage, pickup_count) = {
        let world = app.world_mut();
        let mut weapon_query = world.query::<&SimpleWeapon>();
        let damage = weapon_query
            .single(world)
            .expect("player weapon should remain available")
            .damage;
        let mut pickup_query = world.query::<&DragonKatanaPickup>();
        let pickup_count = pickup_query.iter(world).count();
        (damage, pickup_count)
    };
    assert_eq!(damage, 10.0);
    assert_eq!(pickup_count, 0);
}

#[test]
fn humanoid_idle_ik_keeps_a_readable_upright_silhouette() {
    let motion = PlayerMotion {
        grounded: true,
        ..default()
    };
    let pose = super::player::procedural_player_pose(
        1.0,
        0.0,
        &motion,
        &PlayerJump::default(),
        false,
        0.0,
    );
    let solved = HumanoidRig::player_avatar().solve(HumanoidTargets {
        left_hand: pose.left_hand,
        right_hand: pose.right_hand,
        left_foot: pose.left_foot,
        right_foot: pose.right_foot,
        left_elbow_pole: pose.left_elbow_pole,
        right_elbow_pole: pose.right_elbow_pole,
        left_knee_pole: pose.left_knee_pole,
        right_knee_pole: pose.right_knee_pole,
    });

    assert!(solved.left_leg.joint.z > 0.0);
    assert!(solved.right_leg.joint.z > 0.0);
    assert!(solved.left_leg.joint.y < solved.left_leg.root.y);
    assert!(solved.right_leg.joint.y < solved.right_leg.root.y);
    assert!(solved.left_arm.joint.x < solved.left_arm.root.x);
    assert!(solved.right_arm.joint.x > solved.right_arm.root.x);
}

fn assert_limb_matches_spec(spec: TwoBoneLimbSpec, solution: TwoBoneSolution) {
    assert_eq!(solution.root, spec.root);
    assert!((solution.joint.distance(solution.root) - spec.upper_len).abs() < 0.001);
    assert!((solution.joint.distance(solution.target) - spec.lower_len).abs() < 0.001);
}

#[test]
fn quadruped_rig_solves_all_rabbit_limbs() {
    let rig = QuadrupedRig::rabbit();
    let solved = rig.solve(QuadrupedTargets {
        front_left_foot: Vec3::new(-0.12, 0.02, -0.28),
        front_right_foot: Vec3::new(0.12, 0.02, -0.28),
        hind_left_foot: Vec3::new(-0.18, 0.02, 0.34),
        hind_right_foot: Vec3::new(0.18, 0.02, 0.34),
        front_left_pole: Vec3::new(-0.20, 0.20, -0.36),
        front_right_pole: Vec3::new(0.20, 0.20, -0.36),
        hind_left_pole: Vec3::new(-0.28, 0.18, 0.42),
        hind_right_pole: Vec3::new(0.28, 0.18, 0.42),
    });

    for (spec, solution) in [
        (rig.front_left, solved.front_left),
        (rig.front_right, solved.front_right),
        (rig.hind_left, solved.hind_left),
        (rig.hind_right, solved.hind_right),
    ] {
        assert_limb_matches_spec(spec, solution);
    }
}

#[test]
fn dragon_rig_solves_legs_and_wings() {
    let rig = DragonRig::hoplite_boss();
    let solved = rig.solve(DragonTargets {
        front_left_foot: Vec3::new(-0.55, 0.06, -0.78),
        front_right_foot: Vec3::new(0.55, 0.06, -0.78),
        hind_left_foot: Vec3::new(-0.70, 0.05, 0.78),
        hind_right_foot: Vec3::new(0.70, 0.05, 0.78),
        left_wing_tip: Vec3::new(-2.12, 1.34, -0.04),
        right_wing_tip: Vec3::new(2.12, 1.34, -0.04),
        front_left_pole: Vec3::new(-0.72, 0.48, -0.98),
        front_right_pole: Vec3::new(0.72, 0.48, -0.98),
        hind_left_pole: Vec3::new(-0.86, 0.42, 0.98),
        hind_right_pole: Vec3::new(0.86, 0.42, 0.98),
        left_wing_pole: Vec3::new(-1.20, 1.86, -0.42),
        right_wing_pole: Vec3::new(1.20, 1.86, -0.42),
    });

    for (spec, solution) in [
        (rig.front_left_leg, solved.front_left_leg),
        (rig.front_right_leg, solved.front_right_leg),
        (rig.hind_left_leg, solved.hind_left_leg),
        (rig.hind_right_leg, solved.hind_right_leg),
        (rig.left_wing, solved.left_wing),
        (rig.right_wing, solved.right_wing),
    ] {
        assert_limb_matches_spec(spec, solution);
    }
}

#[test]
fn rig_segment_pose_places_mesh_between_endpoints() {
    let mut transform = Transform::default();
    apply_segment_between(
        &mut transform,
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(1.0, 5.0, 3.0),
    );

    assert_eq!(transform.translation, Vec3::new(1.0, 3.5, 3.0));
    assert!((transform.scale.y - 3.0).abs() < 0.001);
}

#[test]
fn procedural_pose_walks_feet_in_opposition() {
    let motion = PlayerMotion {
        smoothed_speed: super::util::PLAYER_SPEED,
        stride_phase: std::f32::consts::FRAC_PI_2,
        grounded: true,
        ..default()
    };
    let pose = super::player::procedural_player_pose(
        1.0,
        0.0,
        &motion,
        &PlayerJump::default(),
        false,
        0.0,
    );

    assert!(pose.left_foot.z > pose.right_foot.z);
    assert!(pose.right_hand.z > pose.left_hand.z + 0.30);
    assert!(pose.right_hand.y > pose.left_hand.y);
    assert!(pose.left_foot.y <= pose.right_foot.y + 0.02);
    assert!(pose.torso_roll > 0.0);
    assert!(pose.gait > 0.95);
    assert_eq!(pose.stride_phase, std::f32::consts::FRAC_PI_2);
}

#[test]
fn procedural_pose_lifts_swing_foot_during_step() {
    let motion = PlayerMotion {
        smoothed_speed: super::util::PLAYER_SPEED,
        stride_phase: 0.0,
        grounded: true,
        ..default()
    };
    let pose = super::player::procedural_player_pose(
        1.0,
        0.0,
        &motion,
        &PlayerJump::default(),
        false,
        0.0,
    );

    assert!(pose.left_foot.y > pose.right_foot.y + 0.16);
    assert!((pose.left_foot.z - pose.right_foot.z).abs() < 0.02);
    assert!(pose.left_knee_pole.y > pose.right_knee_pole.y);
}

#[test]
fn procedural_pose_attack_extends_visual_right_hand() {
    let motion = PlayerMotion {
        grounded: true,
        ..default()
    };
    let idle =
        super::player::procedural_player_pose(1.0, 0.0, &motion, &PlayerJump::default(), true, 0.0);
    let attacking = super::player::procedural_player_pose(
        1.0,
        0.32,
        &motion,
        &PlayerJump::default(),
        true,
        0.0,
    );

    assert!(attacking.left_hand.z > idle.left_hand.z + 0.30);
    assert!(attacking.left_hand.y > idle.left_hand.y);
    assert!((attacking.right_hand.z - idle.right_hand.z).abs() < 0.01);
    assert!(attacking.attack > idle.attack);
    assert_eq!(attacking.airborne, 0.0);
}

#[test]
fn held_stick_tracks_visual_right_side() {
    let motion = PlayerMotion {
        grounded: true,
        ..default()
    };
    let pose = super::player::procedural_player_pose(
        1.0,
        0.0,
        &motion,
        &PlayerJump::default(),
        false,
        0.0,
    );
    let (start, end) = super::player::held_stick_segment(&pose);

    assert!(start.x < 0.0);
    assert!(end.x < start.x);
    assert!(start.distance(pose.left_hand) < start.distance(pose.right_hand));
    assert!(end.z > start.z);
}

#[test]
fn rabbit_ai_flees_when_player_is_close() {
    let decision = rabbit_ai_decision(
        Vec3::ZERO,
        Some(Vec3::new(1.0, 0.0, 0.0)),
        &[RabbitFoodTarget {
            position: Vec3::new(0.0, 0.0, 4.0),
            value: 2.0,
        }],
    );

    assert_eq!(decision.mood, RabbitMood::Flee);
    assert!(
        decision.target.x < -0.5,
        "target should move away from player"
    );
}

#[test]
fn rabbit_flee_target_extends_from_rabbit_position() {
    let decision = rabbit_ai_decision(
        Vec3::new(10.0, 0.0, 0.0),
        Some(Vec3::new(11.0, 0.0, 0.0)),
        &[],
    );

    assert_eq!(decision.mood, RabbitMood::Flee);
    assert_eq!(decision.target, Vec3::new(6.0, 0.0, 0.0));
}

#[test]
fn rabbit_ai_seeks_nearest_fruiting_berry_when_safe() {
    let decision = rabbit_ai_decision(
        Vec3::ZERO,
        Some(Vec3::new(8.0, 0.0, 0.0)),
        &[
            RabbitFoodTarget {
                position: Vec3::new(4.0, 0.0, 0.0),
                value: 0.0,
            },
            RabbitFoodTarget {
                position: Vec3::new(0.0, 0.0, 3.0),
                value: 1.0,
            },
            RabbitFoodTarget {
                position: Vec3::new(0.0, 0.0, 6.0),
                value: 2.0,
            },
        ],
    );

    assert_eq!(decision.mood, RabbitMood::Forage);
    assert_eq!(decision.target, Vec3::new(0.0, 0.0, 3.0));
}

#[test]
fn rabbit_ai_grazes_visible_grass_when_no_fruiting_berries() {
    let decision = rabbit_ai_decision(
        Vec3::ZERO,
        None,
        &[RabbitFoodTarget {
            position: Vec3::new(0.35, 0.0, 0.0),
            value: 0.4,
        }],
    );

    assert_eq!(decision.mood, RabbitMood::Graze);
    assert_eq!(decision.target, Vec3::new(0.35, 0.0, 0.0));
}

#[test]
fn rabbit_ai_idles_without_fruiting_berries() {
    let rabbit_pos = Vec3::new(2.0, 0.0, -1.0);
    let decision = rabbit_ai_decision(
        rabbit_pos,
        None,
        &[RabbitFoodTarget {
            position: Vec3::new(0.0, 0.0, 3.0),
            value: 0.0,
        }],
    );

    assert_eq!(decision.mood, RabbitMood::Idle);
    assert_eq!(decision.target, rabbit_pos);
}

#[test]
fn shared_ai_dragon_chases_prey_outside_attack_range() {
    let prey = [AiTarget {
        position: Vec3::new(12.0, 0.0, 0.0),
        value: 1.0,
    }];
    let intent = decide_creature_intent(
        CreatureAiProfile::dragon(),
        AiContext {
            self_pos: Vec3::ZERO,
            threat: None,
            food: &[],
            prey: &prey,
        },
    );

    assert_eq!(intent.mood, AiMood::Chase);
    assert_eq!(intent.target, prey[0].position);
}

#[test]
fn shared_ai_dragon_attacks_prey_inside_attack_range() {
    let prey = [AiTarget {
        position: Vec3::new(3.0, 0.0, 0.0),
        value: 1.0,
    }];
    let intent = decide_creature_intent(
        CreatureAiProfile::dragon(),
        AiContext {
            self_pos: Vec3::ZERO,
            threat: None,
            food: &[],
            prey: &prey,
        },
    );

    assert_eq!(intent.mood, AiMood::Attack);
    assert_eq!(intent.target, prey[0].position);
}

#[test]
fn shared_ai_wolf_chases_rabbit_prey() {
    let prey = [AiTarget {
        position: Vec3::new(4.0, 0.0, 0.0),
        value: 1.0,
    }];
    let intent = decide_creature_intent(
        CreatureAiProfile::wolf(),
        AiContext {
            self_pos: Vec3::ZERO,
            threat: None,
            food: &[],
            prey: &prey,
        },
    );

    assert_eq!(intent.mood, AiMood::Chase);
    assert_eq!(intent.target, prey[0].position);
}

#[test]
fn shared_ai_wolf_pounces_inside_attack_range() {
    let prey = [AiTarget {
        position: Vec3::new(0.65, 0.0, 0.0),
        value: 1.0,
    }];
    let intent = decide_creature_intent(
        CreatureAiProfile::wolf(),
        AiContext {
            self_pos: Vec3::ZERO,
            threat: None,
            food: &[],
            prey: &prey,
        },
    );

    assert_eq!(intent.mood, AiMood::Attack);
    assert_eq!(intent.target, prey[0].position);
}

#[test]
fn offline_nature_advances_through_the_shared_world_step() {
    let mut nature = OfflineNature::default();
    let initial = nature.snapshot.clone();

    assert_eq!(initial.atmosphere.cloud_count, 1);
    assert_eq!(initial.ecology.plant_count, 0);
    assert_eq!(initial.ecology.animal_count, 0);
    assert!(initial.detailed_ecology.berries.is_empty());
    assert!(initial.detailed_ecology.rabbits.is_empty());

    assert_eq!(nature.advance(constant::SLOW_TICK_SECS), 1);
    assert_eq!(nature.tick, 1);
    assert_eq!(nature.snapshot.tick, 1);
    assert!(nature.snapshot.is_finite());
    assert_ne!(nature.snapshot.detailed_ecology, initial.detailed_ecology);
}

#[test]
fn rain_creates_plants_then_berries_then_rabbits() {
    let mut nature = OfflineNature::default();
    let mut first_plant = None;
    let mut first_berry = None;
    let mut first_rabbit = None;

    for _ in 0..80 {
        nature.advance(constant::SLOW_TICK_SECS);
        first_plant = first_plant.or_else(|| {
            (!nature.snapshot.detailed_ecology.plants.is_empty()).then_some(nature.tick)
        });
        first_berry = first_berry.or_else(|| {
            (!nature.snapshot.detailed_ecology.berries.is_empty()).then_some(nature.tick)
        });
        first_rabbit = first_rabbit.or_else(|| {
            (!nature.snapshot.detailed_ecology.rabbits.is_empty()).then_some(nature.tick)
        });
        if first_rabbit.is_some() {
            break;
        }
    }

    let plant_tick = first_plant.expect("rain should grow plants");
    let berry_tick = first_berry.expect("plants and rain should grow berries");
    let rabbit_tick = first_rabbit.expect("berry bushes should support rabbits");
    assert!(plant_tick <= berry_tick);
    assert!(berry_tick <= rabbit_tick);
}

#[test]
fn deterministic_layout_hash_stays_in_unit_interval() {
    for index in 0..512 {
        let value = hash01(index, 17);
        assert!((0.0..=1.0).contains(&value));
        assert_eq!(value, hash01(index, 17));
    }
}
