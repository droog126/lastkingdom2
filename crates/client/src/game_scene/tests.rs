//! Tests for the offline authority and the cloud/rain animation schedule.

use std::path::PathBuf;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use super::animation::{
    animate_boss, animate_clouds_and_rain, animate_rabbits, animate_tree_sway, rabbit_ai_decision,
    RabbitFoodTarget,
};
use super::content_visuals::{
    content_cell_position, living_content_layout_from_args, parse_content_profile,
    parse_content_seed, DEFAULT_LIVING_CONTENT_SEED, LIVING_CONTENT_DIMENSIONS,
};
use super::creature_ai::{decide_creature_intent, AiContext, AiMood, AiTarget, CreatureAiProfile};
use super::keybindings::KeyBindings;
use super::offline::OfflineNature;
use super::player::camera_look_input;
use super::procedural_motion::{heading_yaw, hop_height, smooth_follow_alpha, ProceduralTreeSway};
use super::procedural_rig::{apply_segment_between, TwoBoneLimb};
use super::state::{
    BossActor, Cloud, LivingCameraRig, LivingSceneState, PlayerActor, PlayerIkPart,
    PlayerIkPartKind, PlayerJump, PlayerMotion, Rabbit, RabbitAi, RabbitMood, RainDrop, SlashFx,
};
use super::util::hash01;
use lk2_core::constant;
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
fn content_layout_maps_volume_cells_to_scene_space() {
    let center = content_cell_position(LIVING_CONTENT_DIMENSIONS, [2, 1, 2]);
    let corner = content_cell_position(LIVING_CONTENT_DIMENSIONS, [0, 1, 0]);

    assert_eq!(center, Vec3::new(0.0, 0.04, 0.0));
    assert!(corner.x < center.x);
    assert!(corner.z < center.z);
}

#[test]
fn living_content_layout_uses_spice_profiles() {
    let args = vec![
        "lk2-client".to_string(),
        "--content-seed=99".to_string(),
        "--content-profile=monster-march".to_string(),
    ];
    let layout = living_content_layout_from_args(&args);

    assert_eq!(layout.volume.get([2, 1, 2]), Some(GAME_CONTENT_SETTLEMENT));
    assert!(
        layout.volume.count(GAME_CONTENT_MONSTER_TERRITORY) > 4,
        "monster profile should create a readable monster territory"
    );
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

    app.update();
}

#[test]
fn space_jumps_without_attacking_and_left_click_attacks() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .insert_resource(test_scene_state())
        .insert_resource(super::state::SceneMaterials {
            ground: Handle::default(),
            slash_mesh: Handle::default(),
            slash_material: Handle::default(),
        })
        .insert_resource(LivingCameraRig::default())
        .insert_resource(KeyBindings::default())
        .insert_resource(ButtonInput::<MouseButton>::default())
        .add_systems(Update, super::player::player_controls);
    app.world_mut().spawn((
        PlayerActor,
        PlayerJump::default(),
        PlayerMotion::default(),
        PvpCombatant::default(),
        SimpleWeapon::default(),
        Transform::default(),
    ));

    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::Space);
    app.insert_resource(keys);
    app.update();

    let (jump, combatant) = app
        .world_mut()
        .query::<(&PlayerJump, &PvpCombatant)>()
        .single(app.world())
        .expect("player should remain spawned");
    assert!(jump.vertical_velocity > 0.0);
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
fn c_toggles_camera_mode() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .insert_resource(LivingCameraRig::default())
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
    assert!(pose.left_foot.y > pose.right_foot.y);
    assert!(pose.torso_roll > 0.0);
    assert!(pose.gait > 0.95);
    assert_eq!(pose.stride_phase, std::f32::consts::FRAC_PI_2);
}

#[test]
fn procedural_pose_attack_extends_right_hand() {
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

    assert!(attacking.right_hand.z < idle.right_hand.z - 0.30);
    assert!(attacking.right_hand.y > idle.right_hand.y);
    assert!(attacking.attack > idle.attack);
    assert_eq!(attacking.airborne, 0.0);
}

#[test]
fn rabbit_ai_flees_when_player_is_close() {
    let decision = rabbit_ai_decision(
        Vec3::ZERO,
        Some(Vec3::new(1.0, 0.0, 0.0)),
        &[RabbitFoodTarget {
            position: Vec3::new(0.0, 0.0, 4.0),
            fruit: 2,
        }],
    );

    assert_eq!(decision.mood, RabbitMood::Flee);
    assert!(
        decision.target.x < -0.5,
        "target should move away from player"
    );
}

#[test]
fn rabbit_ai_seeks_nearest_fruiting_berry_when_safe() {
    let decision = rabbit_ai_decision(
        Vec3::ZERO,
        Some(Vec3::new(8.0, 0.0, 0.0)),
        &[
            RabbitFoodTarget {
                position: Vec3::new(4.0, 0.0, 0.0),
                fruit: 0,
            },
            RabbitFoodTarget {
                position: Vec3::new(0.0, 0.0, 3.0),
                fruit: 1,
            },
            RabbitFoodTarget {
                position: Vec3::new(0.0, 0.0, 6.0),
                fruit: 2,
            },
        ],
    );

    assert_eq!(decision.mood, RabbitMood::Forage);
    assert_eq!(decision.target, Vec3::new(0.0, 0.0, 3.0));
}

#[test]
fn rabbit_ai_idles_without_fruiting_berries() {
    let rabbit_pos = Vec3::new(2.0, 0.0, -1.0);
    let decision = rabbit_ai_decision(
        rabbit_pos,
        None,
        &[RabbitFoodTarget {
            position: Vec3::new(0.0, 0.0, 3.0),
            fruit: 0,
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
