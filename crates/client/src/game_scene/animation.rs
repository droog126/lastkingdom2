//! Per-frame animation systems: drifting clouds and rain, growing grass and
//! berries, foraging rabbits, idle/squash boss pose, and ground darkening
//! after rain.

use avian3d::prelude::{LinearVelocity, Rotation};
use bevy::prelude::*;

use super::content_visuals::ExportedContentVisual;
use super::creature_ai::{AiContext, AiMood, AiTarget, CreatureAiProfile, decide_creature_intent};
use super::offline::OfflineNature;
use super::procedural_motion::{
    GrassWind, ProceduralTreeSway, WindField, alternating_step_lift, alternating_step_offset,
    grass_wind_rotation, heading_yaw, hop_height, idle_drift, orbit_bob, smooth_follow_alpha,
    wind_scale, wind_sway,
};
use super::procedural_rig::{
    DragonRig, DragonSolvedRig, DragonTargets, QuadrupedRig, QuadrupedSolvedRig,
    QuadrupedTargets,
};
use super::state::{
    BerryBush, BossActor, Cloud, GrassTuft, HitImpactFx, HitReaction, LivingSceneState, LivingSun,
    PlantNode, PlayerActor, ProceduralTerrainSurface, Rabbit, RabbitAi, RabbitMood, RainDrop,
    SceneMaterials, SlashFx, WildlifeAi, WildlifeAnimal, Wolf, WolfAi, WolfMood,
};
use super::stylized_material::StylizedTerrainMaterial;
use super::util::smoothstep;
use lk2_core::ecology::{ResourceNodeKind, WildlifeKind};
use lk2_core::pvp::Health;

#[derive(Component)]
pub(super) struct QuadrupedVisualBinding;

#[derive(Component, Clone, Copy)]
pub(super) struct QuadrupedVisualPart {
    owner: Entity,
    kind: QuadrupedVisualPartKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum QuadrupedVisualPartKind {
    RabbitFrontLeft,
    RabbitFrontRight,
    RabbitHindLeft,
    RabbitHindRight,
    RabbitBrownFrontLeftUpper,
    RabbitBrownFrontLeftLower,
    RabbitBrownFrontRightUpper,
    RabbitBrownFrontRightLower,
    RabbitBrownHindLeftUpper,
    RabbitBrownHindLeftLower,
    RabbitBrownHindRightUpper,
    RabbitBrownHindRightLower,
    WolfFrontLeftUpper,
    WolfFrontLeftLower,
    WolfFrontRightUpper,
    WolfFrontRightLower,
    WolfHindLeftUpper,
    WolfHindLeftLower,
    WolfHindRightUpper,
    WolfHindRightLower,
    DragonFrontLeftUpper,
    DragonFrontLeftLower,
    DragonFrontRightUpper,
    DragonFrontRightLower,
}

pub fn bind_quadruped_visual_parts(
    mut commands: Commands,
    mut roots: ParamSet<(
        Query<(Entity, &Children, Option<&QuadrupedVisualBinding>), With<Rabbit>>,
        Query<(Entity, &Children, Option<&QuadrupedVisualBinding>), With<Wolf>>,
        Query<(Entity, &Children, Option<&QuadrupedVisualBinding>), With<BossActor>>,
    )>,
    names: Query<&Name>,
    children: Query<&Children>,
) {
    let mut bind_root = |root: Entity, root_children: &Children, bound: bool, dragon: bool| {
        if bound {
            return;
        }
        let mut found = 0;
        for child in root_children.iter() {
            found += collect_quadruped_parts(
                child,
                root,
                dragon,
                &names,
                &children,
                &mut commands,
            );
        }
        if found > 0 {
            commands.entity(root).insert(QuadrupedVisualBinding);
        }
    };
    for (root, children, bound) in &mut roots.p0() {
        bind_root(root, children, bound.is_some(), false);
    }
    for (root, children, bound) in &mut roots.p1() {
        bind_root(root, children, bound.is_some(), false);
    }
    for (root, children, bound) in &mut roots.p2() {
        bind_root(root, children, bound.is_some(), true);
    }
}

fn collect_quadruped_parts(
    entity: Entity,
    owner: Entity,
    dragon: bool,
    names: &Query<&Name>,
    children: &Query<&Children>,
    commands: &mut Commands,
) -> usize {
    let mut found = 0;
    if let Ok(name) = names.get(entity) {
        if let Some(kind) = quadruped_part_kind(name.as_str(), dragon) {
            commands
                .entity(entity)
                .insert(QuadrupedVisualPart { owner, kind });
            found += 1;
        }
    }
    if let Ok(child_list) = children.get(entity) {
        for child in child_list.iter() {
            found += collect_quadruped_parts(child, owner, dragon, names, children, commands);
        }
    }
    found
}

fn quadruped_part_kind(name: &str, dragon: bool) -> Option<QuadrupedVisualPartKind> {
    if dragon {
        return if name.contains("dragon_leg_-1.0") {
            Some(if name.ends_with("upper") {
                QuadrupedVisualPartKind::DragonFrontLeftUpper
            } else {
                QuadrupedVisualPartKind::DragonFrontLeftLower
            })
        } else if name.contains("dragon_leg_1.0") {
            Some(if name.ends_with("upper") {
                QuadrupedVisualPartKind::DragonFrontRightUpper
            } else {
                QuadrupedVisualPartKind::DragonFrontRightLower
            })
        } else {
            None
        };
    }
    if name == "rabbit_brown_front_leg_-1_upper" {
        Some(QuadrupedVisualPartKind::RabbitBrownFrontLeftUpper)
    } else if name == "rabbit_brown_front_leg_-1_lower" {
        Some(QuadrupedVisualPartKind::RabbitBrownFrontLeftLower)
    } else if name == "rabbit_brown_front_leg_1_upper" {
        Some(QuadrupedVisualPartKind::RabbitBrownFrontRightUpper)
    } else if name == "rabbit_brown_front_leg_1_lower" {
        Some(QuadrupedVisualPartKind::RabbitBrownFrontRightLower)
    } else if name == "rabbit_brown_hind_leg_-1_upper" {
        Some(QuadrupedVisualPartKind::RabbitBrownHindLeftUpper)
    } else if name == "rabbit_brown_hind_leg_-1_lower" {
        Some(QuadrupedVisualPartKind::RabbitBrownHindLeftLower)
    } else if name == "rabbit_brown_hind_leg_1_upper" {
        Some(QuadrupedVisualPartKind::RabbitBrownHindRightUpper)
    } else if name == "rabbit_brown_hind_leg_1_lower" {
        Some(QuadrupedVisualPartKind::RabbitBrownHindRightLower)
    } else if name.starts_with("front_paw_-1") {
        Some(QuadrupedVisualPartKind::RabbitFrontLeft)
    } else if name.starts_with("front_paw_1") {
        Some(QuadrupedVisualPartKind::RabbitFrontRight)
    } else if name.starts_with("hind_paw_-1") {
        Some(QuadrupedVisualPartKind::RabbitHindLeft)
    } else if name.starts_with("hind_paw_1") {
        Some(QuadrupedVisualPartKind::RabbitHindRight)
    } else if name == "leg_front_left_upper" {
        Some(QuadrupedVisualPartKind::WolfFrontLeftUpper)
    } else if name == "leg_front_left_lower" {
        Some(QuadrupedVisualPartKind::WolfFrontLeftLower)
    } else if name == "leg_front_right_upper" {
        Some(QuadrupedVisualPartKind::WolfFrontRightUpper)
    } else if name == "leg_front_right_lower" {
        Some(QuadrupedVisualPartKind::WolfFrontRightLower)
    } else if name == "leg_hind_left_upper" {
        Some(QuadrupedVisualPartKind::WolfHindLeftUpper)
    } else if name == "leg_hind_left_lower" {
        Some(QuadrupedVisualPartKind::WolfHindLeftLower)
    } else if name == "leg_hind_right_upper" {
        Some(QuadrupedVisualPartKind::WolfHindRightUpper)
    } else if name == "leg_hind_right_lower" {
        Some(QuadrupedVisualPartKind::WolfHindRightLower)
    } else {
        None
    }
}

fn apply_rabbit_visual_parts(
    parts: &mut Query<(&QuadrupedVisualPart, &mut Transform), Without<Rabbit>>,
    owner: Entity,
    targets: QuadrupedTargets,
    rig: QuadrupedSolvedRig,
) {
    for (part, mut transform) in parts.iter_mut() {
        if part.owner != owner {
            continue;
        }
        match part.kind {
            QuadrupedVisualPartKind::RabbitFrontLeft => {
                transform.translation = targets.front_left_foot;
            }
            QuadrupedVisualPartKind::RabbitFrontRight => {
                transform.translation = targets.front_right_foot;
            }
            QuadrupedVisualPartKind::RabbitHindLeft => {
                transform.translation = targets.hind_left_foot;
            }
            QuadrupedVisualPartKind::RabbitHindRight => {
                transform.translation = targets.hind_right_foot;
            }
            QuadrupedVisualPartKind::RabbitBrownFrontLeftUpper => apply_visual_segment(
                &mut transform,
                rig.front_left.root,
                rig.front_left.joint,
                Vec3::Z,
                0.17,
            ),
            QuadrupedVisualPartKind::RabbitBrownFrontLeftLower => apply_visual_segment(
                &mut transform,
                rig.front_left.joint,
                rig.front_left.target,
                Vec3::Z,
                0.13,
            ),
            QuadrupedVisualPartKind::RabbitBrownFrontRightUpper => apply_visual_segment(
                &mut transform,
                rig.front_right.root,
                rig.front_right.joint,
                Vec3::Z,
                0.17,
            ),
            QuadrupedVisualPartKind::RabbitBrownFrontRightLower => apply_visual_segment(
                &mut transform,
                rig.front_right.joint,
                rig.front_right.target,
                Vec3::Z,
                0.13,
            ),
            QuadrupedVisualPartKind::RabbitBrownHindLeftUpper => apply_visual_segment(
                &mut transform,
                rig.hind_left.root,
                rig.hind_left.joint,
                Vec3::Z,
                0.21,
            ),
            QuadrupedVisualPartKind::RabbitBrownHindLeftLower => apply_visual_segment(
                &mut transform,
                rig.hind_left.joint,
                rig.hind_left.target,
                Vec3::Z,
                0.17,
            ),
            QuadrupedVisualPartKind::RabbitBrownHindRightUpper => apply_visual_segment(
                &mut transform,
                rig.hind_right.root,
                rig.hind_right.joint,
                Vec3::Z,
                0.21,
            ),
            QuadrupedVisualPartKind::RabbitBrownHindRightLower => apply_visual_segment(
                &mut transform,
                rig.hind_right.joint,
                rig.hind_right.target,
                Vec3::Z,
                0.17,
            ),
            _ => continue,
        }
    }
}

fn apply_wolf_visual_parts(
    parts: &mut Query<(&QuadrupedVisualPart, &mut Transform), Without<Wolf>>,
    owner: Entity,
    rig: QuadrupedSolvedRig,
) {
    for (part, mut transform) in parts.iter_mut() {
        if part.owner != owner {
            continue;
        }
        let (start, target) = match part.kind {
            QuadrupedVisualPartKind::WolfFrontLeftUpper => {
                (rig.front_left.root, rig.front_left.joint)
            }
            QuadrupedVisualPartKind::WolfFrontLeftLower => {
                (rig.front_left.joint, rig.front_left.target)
            }
            QuadrupedVisualPartKind::WolfFrontRightUpper => {
                (rig.front_right.root, rig.front_right.joint)
            }
            QuadrupedVisualPartKind::WolfFrontRightLower => {
                (rig.front_right.joint, rig.front_right.target)
            }
            QuadrupedVisualPartKind::WolfHindLeftUpper => {
                (rig.hind_left.root, rig.hind_left.joint)
            }
            QuadrupedVisualPartKind::WolfHindLeftLower => {
                (rig.hind_left.joint, rig.hind_left.target)
            }
            QuadrupedVisualPartKind::WolfHindRightUpper => {
                (rig.hind_right.root, rig.hind_right.joint)
            }
            QuadrupedVisualPartKind::WolfHindRightLower => {
                (rig.hind_right.joint, rig.hind_right.target)
            }
            _ => continue,
        };
        apply_visual_segment(&mut transform, start, target, Vec3::Y, 0.20);
    }
}

fn apply_dragon_visual_parts(
    parts: &mut Query<(&QuadrupedVisualPart, &mut Transform), Without<BossActor>>,
    owner: Entity,
    rig: DragonSolvedRig,
) {
    for (part, mut transform) in parts.iter_mut() {
        if part.owner != owner {
            continue;
        }
        let (start, target) = match part.kind {
            QuadrupedVisualPartKind::DragonFrontLeftUpper => {
                (rig.front_left_leg.root, rig.front_left_leg.joint)
            }
            QuadrupedVisualPartKind::DragonFrontLeftLower => {
                (rig.front_left_leg.joint, rig.front_left_leg.target)
            }
            QuadrupedVisualPartKind::DragonFrontRightUpper => {
                (rig.front_right_leg.root, rig.front_right_leg.joint)
            }
            QuadrupedVisualPartKind::DragonFrontRightLower => {
                (rig.front_right_leg.joint, rig.front_right_leg.target)
            }
            _ => continue,
        };
        apply_visual_segment(&mut transform, start, target, Vec3::Z, 0.55);
    }
}

fn apply_visual_segment(
    transform: &mut Transform,
    start: Vec3,
    target: Vec3,
    axis: Vec3,
    source_length: f32,
) {
    let delta = target - start;
    if delta.length_squared() <= 0.0001 {
        return;
    }
    transform.translation = start + delta * 0.5;
    transform.rotation = Quat::from_rotation_arc(axis, delta.normalize());
    let length_ratio = delta.length() / source_length.max(0.001);
    if axis.y.abs() > 0.5 {
        transform.scale.y = length_ratio;
    } else {
        transform.scale.z = length_ratio;
    }
}

const SUN_ORBIT_SECS: f32 = 56.0;
const SUN_ORBIT_RADIUS: f32 = 30.0;
const SUN_HEIGHT: f32 = 38.0;
const SUN_INITIAL_PHASE: f32 = 2.498_091_5;
const DRAGON_CRUISE_ALTITUDE: f32 = 5.8;
const DRAGON_ATTACK_ALTITUDE: f32 = 3.8;
const DRAGON_CRUISE_RADIUS: f32 = 5.5;
const DRAGON_ATTACK_RADIUS: f32 = 3.2;

pub fn animate_sun(state: Res<LivingSceneState>, mut suns: Query<&mut Transform, With<LivingSun>>) {
    let transform = living_sun_transform(state.elapsed);
    for mut sun in &mut suns {
        *sun = transform;
    }
}

pub fn animate_exported_content_visuals(
    time: Res<Time>,
    mut visuals: Query<(&ExportedContentVisual, &mut Transform)>,
) {
    for (visual, mut transform) in &mut visuals {
        transform.rotation = Quat::from_rotation_y(
            time.elapsed_secs()
                * if visual.status == lk2_core::content::ContentStatus::Planned {
                    0.12
                } else {
                    0.08
                }
                + visual.phase * 0.01,
        );
    }
}

pub(crate) fn living_sun_transform(elapsed: f32) -> Transform {
    let phase = SUN_INITIAL_PHASE + elapsed.max(0.0) / SUN_ORBIT_SECS * std::f32::consts::TAU;
    let position = Vec3::new(
        phase.cos() * SUN_ORBIT_RADIUS,
        SUN_HEIGHT,
        phase.sin() * SUN_ORBIT_RADIUS,
    );
    Transform::from_translation(position).looking_at(Vec3::ZERO, Vec3::Y)
}

pub fn animate_clouds_and_rain(
    time: Res<Time>,
    state: Res<LivingSceneState>,
    nature: Res<OfflineNature>,
    // Cloud and RainDrop are mutually exclusive marker components, but Bevy
    // 0.19's query disjoint check still treats two `&mut Transform` queries as
    // conflicting even when paired with `Without<OtherMarker>` filters. A
    // `ParamSet` makes the mutual exclusion explicit and silences B0001.
    mut cloud_and_drop_transforms: ParamSet<(
        Query<(&Cloud, &mut Transform), Without<RainDrop>>,
        Query<(&RainDrop, &mut Transform), Without<Cloud>>,
    )>,
) {
    let mut cloud_transforms = vec![Vec3::ZERO; nature.snapshot.detailed_ecology.clouds.len()];
    for (cloud, mut transform) in &mut cloud_and_drop_transforms.p0() {
        let Some(snapshot) = nature
            .snapshot
            .detailed_ecology
            .clouds
            .get(cloud.snapshot_index)
        else {
            transform.scale = Vec3::ZERO;
            continue;
        };
        let phase = state.elapsed * 0.12 + cloud.phase;
        let base = Vec3::new(snapshot.x, 11.0, snapshot.z);
        transform.translation =
            base + Vec3::new(phase.sin() * 1.6, phase.cos() * 0.22, phase.cos() * 0.8);
        transform.scale = Vec3::splat(1.6);
        cloud_transforms[cloud.snapshot_index] = transform.translation;
    }
    for (drop, mut transform) in &mut cloud_and_drop_transforms.p1() {
        let Some(base) = cloud_transforms.get(drop.cloud_index) else {
            transform.scale = Vec3::ZERO;
            continue;
        };
        let rain_strength = nature
            .snapshot
            .detailed_ecology
            .clouds
            .get(drop.cloud_index)
            .map_or(0.0, |cloud| cloud.rain.clamp(0.0, 1.0));
        let fall = (state.elapsed * 10.5 + drop.phase).rem_euclid(11.0);
        transform.translation = *base + drop.local - Vec3::Y * fall;
        transform.rotation = Quat::from_rotation_z(-0.10);
        transform.scale = Vec3::new(1.0, rain_strength, 1.0);
        let gust = (time.elapsed_secs() * 1.7 + drop.phase).sin() * 0.12;
        transform.translation.x += gust;
    }
}

pub fn grow_grass(
    state: Res<LivingSceneState>,
    nature: Res<OfflineNature>,
    mut grass: Query<(&GrassTuft, &mut Transform)>,
) {
    let plant_units = nature.snapshot.ecology.plant_units as f32;
    for (tuft, mut transform) in &mut grass {
        let growth = smoothstep((plant_units - tuft.growth_threshold) / 2.0);
        transform.scale = tuft.mature_scale * growth;
        transform.rotation *=
            Quat::from_rotation_z((state.elapsed * 1.2 + tuft.growth_threshold).sin() * 0.002);
    }
}

/// Animate grass as a shared wind field with per-tuft phase variation. The
/// growth system owns scale; this system only changes the presentation tilt.
pub fn animate_grass_wind(
    state: Res<LivingSceneState>,
    wind: Res<WindField>,
    mut scene_queries: ParamSet<(
        Query<
            &Transform,
            (With<super::state::LivingSceneCamera>, Without<GrassWind>),
        >,
        Query<(&GrassWind, &mut Transform)>,
    )>,
) {
    let camera = {
        let cameras = scene_queries.p0();
        cameras.single().ok().map(|transform| transform.translation)
    };
    for (sway, mut transform) in scene_queries.p1().iter_mut() {
        let mut sway = *sway;
        // Grass cards are camera-facing around Y. The wind tilt is applied
        // after billboard alignment, so the silhouette stays readable.
        if let Some(camera) = camera {
            let to_camera = camera - transform.translation;
            if to_camera.xz().length_squared() > 0.001 {
                sway.base_yaw = to_camera.x.atan2(to_camera.z);
            }
        }
        transform.rotation = grass_wind_rotation(state.elapsed, *wind, sway);
    }
}

pub fn grow_berries(nature: Res<OfflineNature>, mut berries: Query<(&BerryBush, &mut Transform)>) {
    for (berry, mut transform) in &mut berries {
        let Some(snapshot) = nature
            .snapshot
            .detailed_ecology
            .berries
            .iter()
            .find(|snapshot| snapshot.id == berry.id)
        else {
            transform.scale = Vec3::ZERO;
            continue;
        };
        transform.translation = Vec3::new(snapshot.x, 0.0, snapshot.z);
        let growth = smoothstep(snapshot.fruit as f32 / 2.0);
        transform.scale = berry.mature_scale * growth;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RabbitFoodTarget {
    pub position: Vec3,
    pub value: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct RabbitAiDecision {
    pub mood: RabbitMood,
    pub target: Vec3,
}

pub fn rabbit_ai_decision(
    rabbit_pos: Vec3,
    player_pos: Option<Vec3>,
    food_targets: &[RabbitFoodTarget],
) -> RabbitAiDecision {
    let food = food_targets
        .iter()
        .map(|target| AiTarget {
            position: target.position,
            value: target.value,
        })
        .collect::<Vec<_>>();
    let intent = decide_creature_intent(
        CreatureAiProfile::rabbit(),
        AiContext {
            self_pos: rabbit_pos,
            threat: player_pos,
            food: &food,
            prey: &[],
        },
    );
    RabbitAiDecision {
        mood: match intent.mood {
            AiMood::Flee => RabbitMood::Flee,
            AiMood::Forage => RabbitMood::Forage,
            AiMood::Graze => RabbitMood::Graze,
            _ => RabbitMood::Idle,
        },
        target: intent.target,
    }
}

pub fn animate_rabbits(
    time: Res<Time>,
    state: Res<LivingSceneState>,
    nature: Res<OfflineNature>,
    terrain: Option<Res<ProceduralTerrainSurface>>,
    players: Query<
        &Transform,
        (
            With<PlayerActor>,
            Without<Rabbit>,
            Without<QuadrupedVisualPart>,
        ),
    >,
    grass: Query<
        &Transform,
        (
            With<GrassTuft>,
            Without<PlayerActor>,
            Without<Rabbit>,
            Without<QuadrupedVisualPart>,
        ),
    >,
    mut rabbits: Query<
        (
            Entity,
            &Rabbit,
            &mut RabbitAi,
            &mut Transform,
            &mut LinearVelocity,
            &mut Rotation,
        ),
        (
            Without<PlayerActor>,
            Without<GrassTuft>,
            Without<QuadrupedVisualPart>,
        ),
    >,
    mut visual_parts: Query<(&QuadrupedVisualPart, &mut Transform), Without<Rabbit>>,
) {
    let player_pos = players.single().ok().map(|transform| transform.translation);
    let mut food_targets = nature
        .snapshot
        .detailed_ecology
        .berries
        .iter()
        .map(|berry| RabbitFoodTarget {
            position: Vec3::new(berry.x, 0.0, berry.z),
            value: berry.fruit as f32,
        })
        .collect::<Vec<_>>();
    food_targets.extend(grass.iter().filter_map(|transform| {
        let value = transform.scale.max_element().clamp(0.0, 1.0);
        (value > 0.08).then_some(RabbitFoodTarget {
            position: Vec3::new(transform.translation.x, 0.0, transform.translation.z),
            value: value * 0.45,
        })
    }));

    for (entity, rabbit, mut ai, mut transform, mut velocity, mut rotation) in &mut rabbits {
        let Some(snapshot) = nature
            .snapshot
            .detailed_ecology
            .rabbits
            .iter()
            .find(|snapshot| snapshot.id == rabbit.id)
        else {
            transform.scale = Vec3::ZERO;
            velocity.0 = Vec3::ZERO;
            continue;
        };
        let authoritative_pos = Vec3::new(snapshot.x, 0.0, snapshot.z);
        let visual_pos = if transform.scale.max_element() > 0.0 {
            transform.translation
        } else {
            authoritative_pos
        };
        let decision = rabbit_ai_decision(visual_pos, player_pos, &food_targets);
        ai.mood = decision.mood;
        ai.target = decision.target;
        let dt = time.delta_secs();
        let speed = match ai.mood {
            RabbitMood::Idle => 1.8,
            RabbitMood::Graze => 2.4,
            RabbitMood::Forage => 3.6,
            RabbitMood::Flee => 8.0,
        };
        ai.hop_phase = (ai.hop_phase + dt * (3.0 + speed) + rabbit.phase * 0.002)
            .rem_euclid(std::f32::consts::TAU);
        let local_idle = idle_drift(state.elapsed, rabbit.phase, 0.18, 0.12);
        let desired = match ai.mood {
            RabbitMood::Idle => authoritative_pos + local_idle,
            RabbitMood::Graze => ai.target,
            RabbitMood::Forage => ai.target,
            RabbitMood::Flee => ai.target,
        };
        let alpha = smooth_follow_alpha(dt, speed);
        let current_position = transform.translation;
        let flat = current_position.lerp(desired, alpha);
        let hop = match ai.mood {
            RabbitMood::Graze => hop_height(ai.hop_phase * 2.0, 0.05),
            RabbitMood::Idle => hop_height(ai.hop_phase, 0.06),
            RabbitMood::Forage => hop_height(ai.hop_phase, 0.16),
            RabbitMood::Flee => hop_height(ai.hop_phase, 0.24),
        };
        let ground_y = terrain.as_ref().map_or(0.0, |terrain| {
            terrain.ground_height(Vec3::new(flat.x, 0.0, flat.z))
        });
        let target_position = Vec3::new(flat.x, ground_y + hop, flat.z);
        let yaw = heading_yaw(
            current_position,
            target_position,
            (state.elapsed * 0.7 + rabbit.phase).sin() * 0.45,
        );
        let graze_pitch = if ai.mood == RabbitMood::Graze {
            0.35
        } else {
            0.0
        };
        let ik_body = Transform {
            rotation: Quat::from_rotation_y(yaw) * Quat::from_rotation_x(graze_pitch),
            ..*transform
        };
        let rabbit_targets = terrain.as_deref().map_or(
            rabbit_quadruped_targets(ai.hop_phase, ai.mood),
            |terrain| {
                ground_quadruped_targets(
                    &ik_body,
                    terrain,
                    rabbit_quadruped_targets(ai.hop_phase, ai.mood),
                    0.02,
                )
            },
        );
        let virtual_limbs = QuadrupedRig::rabbit().solve(rabbit_targets);
        apply_rabbit_visual_parts(&mut visual_parts, entity, rabbit_targets, virtual_limbs);
        let hind_tuck =
            ((virtual_limbs.hind_left.joint.y + virtual_limbs.hind_right.joint.y) * 0.5 - 0.17)
                .clamp(-0.05, 0.05);
        velocity.0 = (target_position - current_position) / dt.max(0.0001);
        let base_scale = match ai.mood {
            RabbitMood::Flee => 1.02,
            RabbitMood::Graze => 0.86,
            _ => 0.92,
        };
        transform.scale = Vec3::splat(base_scale)
            * Vec3::new(
                1.0 + hind_tuck * 0.35,
                1.0 - hind_tuck * 0.20,
                1.0 + hind_tuck * 0.24,
            );
        rotation.0 = ik_body.rotation;
    }
}

pub fn animate_wolves(
    time: Res<Time>,
    state: Res<LivingSceneState>,
    nature: Res<OfflineNature>,
    terrain: Option<Res<ProceduralTerrainSurface>>,
    rabbits: Query<
        &Transform,
        (With<Rabbit>, Without<Wolf>, Without<QuadrupedVisualPart>),
    >,
    mut wolves: Query<
        (
            Entity,
            &Wolf,
            &mut WolfAi,
            &mut Transform,
            &mut LinearVelocity,
            &mut Rotation,
        ),
        (
            With<Wolf>,
            Without<Rabbit>,
            Without<QuadrupedVisualPart>,
        ),
    >,
    mut visual_parts: Query<(&QuadrupedVisualPart, &mut Transform), Without<Wolf>>,
) {
    let prey = rabbits
        .iter()
        .map(|transform| AiTarget {
            position: Vec3::new(transform.translation.x, 0.0, transform.translation.z),
            value: 1.0,
        })
        .collect::<Vec<_>>();

    for (entity, wolf, mut ai, mut transform, mut velocity, mut rotation) in &mut wolves {
        let Some(snapshot) = nature
            .snapshot
            .detailed_ecology
            .wildlife
            .iter()
            .find(|snapshot| snapshot.id == wolf.id)
        else {
            transform.scale = Vec3::ZERO;
            velocity.0 = Vec3::ZERO;
            continue;
        };
        let authoritative_pos = Vec3::new(snapshot.x, 0.0, snapshot.z);
        let visual_pos = if transform.scale.max_element() > 0.0 {
            transform.translation
        } else {
            authoritative_pos
        };
        let intent = decide_creature_intent(
            CreatureAiProfile::wolf(),
            AiContext {
                self_pos: visual_pos,
                threat: None,
                food: &[],
                prey: &prey,
            },
        );
        ai.mood = match intent.mood {
            AiMood::Attack => WolfMood::Pounce,
            AiMood::Chase => WolfMood::Chase,
            _ => WolfMood::Idle,
        };
        ai.target = intent.target;

        let dt = time.delta_secs();
        let speed = match ai.mood {
            WolfMood::Idle => 1.4,
            WolfMood::Chase => 5.6,
            WolfMood::Pounce => 3.2,
        };
        ai.run_phase = (ai.run_phase + dt * (3.2 + speed) + wolf.phase * 0.001)
            .rem_euclid(std::f32::consts::TAU);
        let desired = match ai.mood {
            WolfMood::Idle => authoritative_pos + idle_drift(state.elapsed, wolf.phase, 0.28, 0.22),
            WolfMood::Chase | WolfMood::Pounce => ai.target,
        };
        let current_position = transform.translation;
        let flat = current_position.lerp(desired, smooth_follow_alpha(dt, speed));
        let lope = match ai.mood {
            WolfMood::Idle => hop_height(ai.run_phase, 0.03),
            WolfMood::Chase => hop_height(ai.run_phase, 0.11),
            WolfMood::Pounce => hop_height(ai.run_phase * 0.8, 0.16),
        };
        let ground_y = terrain.as_ref().map_or(0.0, |terrain| {
            terrain.ground_height(Vec3::new(flat.x, 0.0, flat.z))
        });
        let target_position = Vec3::new(flat.x, ground_y + lope, flat.z);
        let yaw = heading_yaw(
            current_position,
            target_position,
            (state.elapsed * 0.5 + wolf.phase).sin() * 0.35,
        );
        let pounce_pitch = if ai.mood == WolfMood::Pounce {
            -0.18
        } else {
            0.0
        };
        let ik_body = Transform {
            rotation: Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pounce_pitch),
            ..*transform
        };
        let wolf_targets = terrain.as_deref().map_or(
            wolf_quadruped_targets(ai.run_phase, ai.mood),
            |terrain| {
                ground_quadruped_targets(
                    &ik_body,
                    terrain,
                    wolf_quadruped_targets(ai.run_phase, ai.mood),
                    0.04,
                )
            },
        );
        let rig = QuadrupedRig::wolf().solve(wolf_targets);
        apply_wolf_visual_parts(&mut visual_parts, entity, rig);
        let shoulder_drive =
            ((rig.front_left.joint.y + rig.front_right.joint.y) * 0.5 - 0.28).clamp(-0.08, 0.08);
        velocity.0 = (target_position - current_position) / dt.max(0.0001);
        transform.scale = Vec3::splat(match ai.mood {
            WolfMood::Idle => 0.62,
            WolfMood::Chase => 0.68,
            WolfMood::Pounce => 0.72,
        }) * Vec3::new(
            1.0 + shoulder_drive * 0.35,
            1.0 - shoulder_drive * 0.15,
            1.0,
        );
        rotation.0 = ik_body.rotation;
    }
}

pub fn animate_wildlife(
    time: Res<Time>,
    state: Res<LivingSceneState>,
    nature: Res<OfflineNature>,
    terrain: Option<Res<ProceduralTerrainSurface>>,
    players: Query<&Transform, (With<PlayerActor>, Without<WildlifeAnimal>)>,
    rabbits: Query<&Transform, (With<Rabbit>, Without<WildlifeAnimal>)>,
    grass: Query<&Transform, (With<GrassTuft>, Without<WildlifeAnimal>, Without<Rabbit>)>,
    plants: Query<(&PlantNode, &Transform), Without<WildlifeAnimal>>,
    mut animals: Query<
        (
            &WildlifeAnimal,
            &mut WildlifeAi,
            &mut Transform,
            &mut LinearVelocity,
            &mut Rotation,
        ),
        (Without<PlayerActor>, Without<Rabbit>, Without<Wolf>),
    >,
) {
    let player_pos = players.single().ok().map(|transform| transform.translation);
    let prey = rabbits
        .iter()
        .map(|transform| AiTarget {
            position: Vec3::new(transform.translation.x, 0.0, transform.translation.z),
            value: 1.0,
        })
        .collect::<Vec<_>>();
    let mut food = nature
        .snapshot
        .detailed_ecology
        .berries
        .iter()
        .map(|berry| AiTarget {
            position: Vec3::new(berry.x, 0.0, berry.z),
            value: berry.fruit as f32,
        })
        .collect::<Vec<_>>();
    food.extend(grass.iter().filter_map(|transform| {
        let value = transform.scale.max_element().clamp(0.0, 1.0);
        (value > 0.08).then_some(AiTarget {
            position: Vec3::new(transform.translation.x, 0.0, transform.translation.z),
            value: value * 0.45,
        })
    }));
    food.extend(plants.iter().filter_map(|(plant, transform)| {
        let snapshot = nature
            .snapshot
            .detailed_ecology
            .plants
            .iter()
            .find(|snapshot| snapshot.id == plant.id)?;
        let edible = matches!(
            ResourceNodeKind::from_u8(snapshot.kind),
            ResourceNodeKind::MushroomRed
                | ResourceNodeKind::MushroomBrown
                | ResourceNodeKind::Flower
        );
        edible.then_some(AiTarget {
            position: Vec3::new(transform.translation.x, 0.0, transform.translation.z),
            value: snapshot.stock as f32,
        })
    }));

    for (animal, mut ai, mut transform, mut velocity, mut rotation) in &mut animals {
        let Some(snapshot) = nature
            .snapshot
            .detailed_ecology
            .wildlife
            .iter()
            .find(|snapshot| snapshot.id == animal.id)
        else {
            transform.scale = Vec3::ZERO;
            velocity.0 = Vec3::ZERO;
            continue;
        };

        let kind = WildlifeKind::from_u8(snapshot.kind);
        let profile = match kind {
            WildlifeKind::Deer | WildlifeKind::Rabbit => CreatureAiProfile::deer(),
            WildlifeKind::Fox => CreatureAiProfile::fox(),
            WildlifeKind::Bear | WildlifeKind::Wolf => CreatureAiProfile::bear(),
        };
        let authoritative_pos = Vec3::new(snapshot.x, 0.0, snapshot.z);
        let visual_pos = if transform.scale.max_element() > 0.0 {
            transform.translation
        } else {
            authoritative_pos
        };
        let intent = decide_creature_intent(
            profile,
            AiContext {
                self_pos: visual_pos,
                threat: player_pos,
                food: &food,
                prey: &prey,
            },
        );
        ai.mood = intent.mood;
        ai.target = intent.target;

        let dt = time.delta_secs();
        let base_speed = match kind {
            WildlifeKind::Deer | WildlifeKind::Rabbit => 1.2,
            WildlifeKind::Fox => 1.5,
            WildlifeKind::Bear | WildlifeKind::Wolf => 0.9,
        };
        let speed = match intent.mood {
            AiMood::Flee => 7.0,
            AiMood::Forage => base_speed * 1.8,
            AiMood::Graze => base_speed * 1.25,
            AiMood::Chase => base_speed * 3.5,
            AiMood::Attack => base_speed * 2.2,
            AiMood::Idle => base_speed,
        };
        ai.phase = (ai.phase + dt * (2.5 + speed)).rem_euclid(std::f32::consts::TAU);
        let desired = match intent.mood {
            AiMood::Idle => {
                let (x_radius, z_radius) = match kind {
                    WildlifeKind::Deer | WildlifeKind::Rabbit => (0.22, 0.18),
                    WildlifeKind::Fox => (0.28, 0.22),
                    WildlifeKind::Bear | WildlifeKind::Wolf => (0.34, 0.26),
                };
                authoritative_pos + idle_drift(state.elapsed, ai.phase, x_radius, z_radius)
            }
            _ => ai.target,
        };
        let current_position = transform.translation;
        let flat = current_position.lerp(desired, smooth_follow_alpha(dt, speed));
        let hop = match kind {
            WildlifeKind::Deer => hop_height(ai.phase, 0.07),
            WildlifeKind::Fox => hop_height(ai.phase, 0.09),
            WildlifeKind::Bear | WildlifeKind::Rabbit | WildlifeKind::Wolf => {
                hop_height(ai.phase, 0.04)
            }
        };
        let ground_y = terrain.as_ref().map_or(0.0, |terrain| {
            terrain.ground_height(Vec3::new(flat.x, 0.0, flat.z))
        });
        let target_position = Vec3::new(flat.x, ground_y + hop, flat.z);
        velocity.0 = (target_position - current_position) / dt.max(0.0001);
        let base_scale = match kind {
            WildlifeKind::Deer => 0.62,
            WildlifeKind::Fox => 0.56,
            WildlifeKind::Bear => 0.78,
            WildlifeKind::Rabbit | WildlifeKind::Wolf => 0.62,
        };
        transform.scale = Vec3::splat(base_scale);
        rotation.0 = Quat::from_rotation_y(heading_yaw(
            current_position,
            target_position,
            (state.elapsed * 0.45 + ai.phase).sin() * 0.28,
        ));
    }
}

pub fn animate_boss(
    time: Res<Time>,
    state: Res<LivingSceneState>,
    players: Query<
        &Transform,
        (
            With<PlayerActor>,
            Without<BossActor>,
            Without<QuadrupedVisualPart>,
            Without<SlashFx>,
            Without<HitImpactFx>,
        ),
    >,
    mut bosses: Query<
        (
            Entity,
            &BossActor,
            &Health,
            &mut Transform,
            Option<&mut HitReaction>,
        ),
        (
            Without<SlashFx>,
            Without<HitImpactFx>,
            Without<QuadrupedVisualPart>,
            Without<PlayerActor>,
        ),
    >,
    mut visual_parts: Query<(&QuadrupedVisualPart, &mut Transform), Without<BossActor>>,
    mut slashes: Query<
        (Entity, &mut Transform),
        (
            With<SlashFx>,
            Without<HitImpactFx>,
            Without<BossActor>,
            Without<QuadrupedVisualPart>,
            Without<PlayerActor>,
        ),
    >,
    mut impacts: Query<
        (Entity, &mut Transform, &mut HitImpactFx),
        (
            With<HitImpactFx>,
            Without<BossActor>,
            Without<SlashFx>,
            Without<QuadrupedVisualPart>,
            Without<PlayerActor>,
        ),
    >,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let player_pos = players.single().ok().map(|transform| transform.translation);
    for (entity, boss, health, mut transform, reaction) in &mut bosses {
        // The dragon's AI is presentation-side in this vertical slice, but it
        // still needs a real target. Previously this list was always empty, so
        // the dragon could never leave its spawn point regardless of its AI
        // profile.
        let intent = if let Some(player) = player_pos {
            let target = AiTarget {
                position: Vec3::new(player.x, transform.translation.y, player.z),
                value: 1.0,
            };
            decide_creature_intent(
                CreatureAiProfile::dragon(),
                AiContext {
                    self_pos: transform.translation,
                    threat: None,
                    food: &[],
                    prey: std::slice::from_ref(&target),
                },
            )
        } else {
            decide_creature_intent(
                CreatureAiProfile::dragon(),
                AiContext {
                    self_pos: transform.translation,
                    threat: None,
                    food: &[],
                    prey: &[],
                },
            )
        };
        let flight_target = dragon_flight_target(boss.base, player_pos, intent.mood, state.elapsed);
        let target_position = if dt > 0.0 {
            transform
                .translation
                .lerp(flight_target, smooth_follow_alpha(dt, 3.2))
        } else {
            flight_target
        };
        let idle = state.elapsed * 4.6;
        let dragon_targets = dragon_boss_targets(idle);
        let virtual_rig = DragonRig::hoplite_boss().solve(dragon_targets);
        apply_dragon_visual_parts(&mut visual_parts, entity, virtual_rig);
        let wing_lift = ((virtual_rig.left_wing.joint.y + virtual_rig.right_wing.joint.y) * 0.5
            - 1.18)
            .clamp(-0.22, 0.22);
        let leg_drive = ((virtual_rig.front_left_leg.joint.y
            + virtual_rig.front_right_leg.joint.y
            + virtual_rig.hind_left_leg.joint.y
            + virtual_rig.hind_right_leg.joint.y)
            * 0.25
            - 0.36)
            .clamp(-0.10, 0.10);
        let (hit_offset, hit_pulse, hit_tilt) = reaction
            .map(|mut reaction| {
                let progress = (reaction.timer / 0.22).clamp(0.0, 1.0);
                reaction.timer = (reaction.timer - dt).max(0.0);
                let recoil = progress.powf(0.65);
                (
                    reaction.direction * reaction.strength * recoil,
                    1.0 + recoil * 0.12,
                    reaction.direction.x * recoil * 0.10,
                )
            })
            .unwrap_or((Vec3::ZERO, 1.0, 0.0));
        let current_position = transform.translation;
        transform.translation = target_position + hit_offset;
        let health_scale = if health.is_dead() { 0.72 } else { 1.0 };
        transform.scale = Vec3::splat(0.92 * hit_pulse * health_scale)
            * Vec3::new(1.0, 1.0 + leg_drive * 0.08, 1.0 + leg_drive * 0.05);
        let flight_direction = target_position - current_position;
        let yaw = heading_yaw(current_position, target_position, 0.0);
        let horizontal_distance = Vec2::new(flight_direction.x, flight_direction.z).length();
        let pitch = flight_direction.y.atan2(horizontal_distance.max(0.001));
        transform.rotation = Quat::from_rotation_y(yaw)
            * Quat::from_rotation_x(-pitch * 0.35)
            * Quat::from_rotation_z(wing_lift * 0.04 + hit_tilt);
    }
    for (entity, mut transform) in &mut slashes {
        transform.scale *= 0.88;
        if transform.scale.max_element() < 0.10 {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut transform, mut impact) in &mut impacts {
        impact.age += dt;
        let progress = (impact.age / impact.lifetime).clamp(0.0, 1.0);
        let growth = 0.24 + progress * 0.92;
        transform.scale = Vec3::new(growth, 0.22 + progress * 0.18, growth);
        transform.rotation *= Quat::from_rotation_y(dt * 8.0);
        if progress >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn dragon_flight_target(
    base: Vec3,
    player: Option<Vec3>,
    mood: AiMood,
    elapsed: f32,
) -> Vec3 {
    let center = player.unwrap_or(base);
    let attacking = mood == AiMood::Attack;
    let radius = if attacking {
        DRAGON_ATTACK_RADIUS
    } else {
        DRAGON_CRUISE_RADIUS
    };
    let altitude = if attacking {
        DRAGON_ATTACK_ALTITUDE
    } else {
        DRAGON_CRUISE_ALTITUDE
    };
    let orbit_speed = if attacking { 1.25 } else { 0.72 };
    let orbit = orbit_bob(
        elapsed * orbit_speed / 0.8,
        0.8,
        radius,
        radius,
        if attacking { 0.35 } else { 0.65 },
    );
    Vec3::new(center.x + orbit.x, altitude + orbit.y, center.z + orbit.z)
}

fn ground_quadruped_targets(
    body: &Transform,
    terrain: &ProceduralTerrainSurface,
    mut targets: QuadrupedTargets,
    contact_y: f32,
) -> QuadrupedTargets {
    let project = |local: Vec3| {
        let world = body.translation + body.rotation * Vec3::new(local.x, 0.0, local.z);
        let ground_y = terrain.ground_height(Vec3::new(world.x, 0.0, world.z));
        let lift = (local.y - contact_y).max(0.0);
        body.rotation.inverse()
            * (Vec3::new(world.x, ground_y + 0.03 + lift, world.z) - body.translation)
    };
    targets.front_left_foot = project(targets.front_left_foot);
    targets.front_right_foot = project(targets.front_right_foot);
    targets.hind_left_foot = project(targets.hind_left_foot);
    targets.hind_right_foot = project(targets.hind_right_foot);
    targets
}

fn rabbit_quadruped_targets(phase: f32, mood: RabbitMood) -> QuadrupedTargets {
    let stride = match mood {
        RabbitMood::Flee => 0.16,
        RabbitMood::Forage => 0.12,
        RabbitMood::Graze => 0.04,
        RabbitMood::Idle => 0.06,
    };
    let left_step = alternating_step_offset(phase, stride, false);
    let right_step = alternating_step_offset(phase, stride, true);
    let left_lift = alternating_step_lift(phase, 0.08, false);
    let right_lift = alternating_step_lift(phase, 0.08, true);
    QuadrupedTargets {
        front_left_foot: Vec3::new(-0.12, 0.02 + left_lift * 0.35, -0.30 + left_step),
        front_right_foot: Vec3::new(0.12, 0.02 + right_lift * 0.35, -0.30 + right_step),
        hind_left_foot: Vec3::new(-0.18, 0.02 + right_lift * 0.30, 0.34 + right_step),
        hind_right_foot: Vec3::new(0.18, 0.02 + left_lift * 0.45, 0.34 + left_step),
        front_left_pole: Vec3::new(-0.20, 0.20, -0.36),
        front_right_pole: Vec3::new(0.20, 0.20, -0.36),
        hind_left_pole: Vec3::new(-0.28, 0.18, 0.42),
        hind_right_pole: Vec3::new(0.28, 0.18, 0.42),
    }
}

fn wolf_quadruped_targets(phase: f32, mood: WolfMood) -> QuadrupedTargets {
    let stride = match mood {
        WolfMood::Idle => 0.08,
        WolfMood::Chase => 0.26,
        WolfMood::Pounce => 0.34,
    };
    let lift = match mood {
        WolfMood::Idle => 0.02,
        WolfMood::Chase => 0.10,
        WolfMood::Pounce => 0.16,
    };
    let left_step = alternating_step_offset(phase, stride, false);
    let right_step = alternating_step_offset(phase, stride, true);
    let left_lift = alternating_step_lift(phase, lift, false);
    let right_lift = alternating_step_lift(phase, lift, true);
    QuadrupedTargets {
        front_left_foot: Vec3::new(-0.20, 0.04 + left_lift, -0.50 + left_step),
        front_right_foot: Vec3::new(0.20, 0.04 + right_lift, -0.50 + right_step),
        hind_left_foot: Vec3::new(-0.22, 0.04 + right_lift, 0.44 + right_step),
        hind_right_foot: Vec3::new(0.22, 0.04 + left_lift, 0.44 + left_step),
        front_left_pole: Vec3::new(-0.34, 0.34, -0.58),
        front_right_pole: Vec3::new(0.34, 0.34, -0.58),
        hind_left_pole: Vec3::new(-0.38, 0.32, 0.58),
        hind_right_pole: Vec3::new(0.38, 0.32, 0.58),
    }
}

fn dragon_boss_targets(phase: f32) -> DragonTargets {
    let wing = phase.sin() * 0.28;
    let left_step = alternating_step_offset(phase, 0.10, false);
    let right_step = alternating_step_offset(phase, 0.10, true);
    let left_lift = alternating_step_lift(phase, 0.06, false);
    let right_lift = alternating_step_lift(phase, 0.06, true);
    DragonTargets {
        front_left_foot: Vec3::new(-0.55, 0.06 + left_lift, -0.78 + left_step),
        front_right_foot: Vec3::new(0.55, 0.06 + right_lift, -0.78 + right_step),
        hind_left_foot: Vec3::new(-0.70, 0.05 + right_lift, 0.78 + right_step),
        hind_right_foot: Vec3::new(0.70, 0.05 + left_lift, 0.78 + left_step),
        left_wing_tip: Vec3::new(-2.12, 1.34 + wing, -0.04),
        right_wing_tip: Vec3::new(2.12, 1.34 + wing, -0.04),
        front_left_pole: Vec3::new(-0.72, 0.48, -0.98),
        front_right_pole: Vec3::new(0.72, 0.48, -0.98),
        hind_left_pole: Vec3::new(-0.86, 0.42, 0.98),
        hind_right_pole: Vec3::new(0.86, 0.42, 0.98),
        left_wing_pole: Vec3::new(-1.20, 1.86 + wing * 0.35, -0.42),
        right_wing_pole: Vec3::new(1.20, 1.86 + wing * 0.35, -0.42),
    }
}

pub fn animate_tree_sway(
    state: Res<LivingSceneState>,
    mut trees: Query<(&ProceduralTreeSway, &mut Transform)>,
) {
    for (sway, mut transform) in &mut trees {
        let root_settle = idle_drift(state.elapsed, sway.phase, 0.018, 0.012);
        transform.translation = sway.base_translation + root_settle;
        transform.rotation = wind_sway(state.elapsed, sway);
        transform.scale = wind_scale(state.elapsed, sway);
    }
}

pub fn update_ground_after_rain(
    nature: Res<OfflineNature>,
    scene_materials: Res<SceneMaterials>,
    mut materials: ResMut<Assets<StylizedTerrainMaterial>>,
) {
    let Some(mut material) = materials.get_mut(&scene_materials.ground) else {
        return;
    };
    let wet = smoothstep(nature.snapshot.atmosphere.cumulative_rainfall / 20.0);
    material.base.base_color = Color::srgb(0.28 - wet * 0.05, 0.49 + wet * 0.06, 0.22 + wet * 0.02);
    material.base.perceptual_roughness = 0.94 - wet * 0.16;
}
