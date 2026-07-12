//! Per-frame animation systems: drifting clouds and rain, growing grass and
//! berries, foraging rabbits, idle/squash boss pose, and ground darkening
//! after rain.

use avian3d::prelude::{LinearVelocity, Rotation};
use bevy::prelude::*;

use super::content_visuals::ExportedContentVisual;
use super::creature_ai::{AiContext, AiMood, AiTarget, CreatureAiProfile, decide_creature_intent};
use super::offline::OfflineNature;
use super::procedural_motion::{
    ProceduralTreeSway, heading_yaw, hop_height, idle_drift, orbit_bob, smooth_follow_alpha,
    wind_scale, wind_sway,
};
use super::procedural_rig::{DragonRig, DragonTargets, QuadrupedRig, QuadrupedTargets};
use super::state::{
    BerryBush, BossActor, Cloud, GrassTuft, HitImpactFx, HitReaction, LivingSceneState, LivingSun,
    PlayerActor, ProceduralTerrainSurface, Rabbit, RabbitAi, RabbitMood, RainDrop, SceneMaterials,
    SlashFx, Wolf, WolfAi, WolfMood,
};
use super::stylized_material::StylizedTerrainMaterial;
use super::util::smoothstep;
use lk2_core::pvp::Health;

const SUN_ORBIT_SECS: f32 = 56.0;
const SUN_ORBIT_RADIUS: f32 = 30.0;
const SUN_HEIGHT: f32 = 38.0;
const SUN_INITIAL_PHASE: f32 = 2.498_091_5;

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
    players: Query<&Transform, (With<PlayerActor>, Without<Rabbit>)>,
    grass: Query<&Transform, (With<GrassTuft>, Without<PlayerActor>, Without<Rabbit>)>,
    mut rabbits: Query<
        (
            &Rabbit,
            &mut RabbitAi,
            &mut Transform,
            &mut LinearVelocity,
            &mut Rotation,
        ),
        (Without<PlayerActor>, Without<GrassTuft>),
    >,
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

    for (rabbit, mut ai, mut transform, mut velocity, mut rotation) in &mut rabbits {
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
        let virtual_limbs =
            QuadrupedRig::rabbit().solve(rabbit_quadruped_targets(ai.hop_phase, ai.mood));
        let hind_tuck =
            ((virtual_limbs.hind_left.joint.y + virtual_limbs.hind_right.joint.y) * 0.5 - 0.17)
                .clamp(-0.05, 0.05);
        let ground_y = terrain.as_ref().map_or(0.0, |terrain| {
            terrain.ground_height(Vec3::new(flat.x, 0.0, flat.z))
        });
        let target_position = Vec3::new(flat.x, ground_y + hop, flat.z);
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
        rotation.0 = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(graze_pitch);
    }
}

pub fn animate_wolves(
    time: Res<Time>,
    state: Res<LivingSceneState>,
    nature: Res<OfflineNature>,
    terrain: Option<Res<ProceduralTerrainSurface>>,
    rabbits: Query<&Transform, (With<Rabbit>, Without<Wolf>)>,
    mut wolves: Query<
        (
            &Wolf,
            &mut WolfAi,
            &mut Transform,
            &mut LinearVelocity,
            &mut Rotation,
        ),
        (With<Wolf>, Without<Rabbit>),
    >,
) {
    let prey = rabbits
        .iter()
        .map(|transform| AiTarget {
            position: Vec3::new(transform.translation.x, 0.0, transform.translation.z),
            value: 1.0,
        })
        .collect::<Vec<_>>();

    for (wolf, mut ai, mut transform, mut velocity, mut rotation) in &mut wolves {
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
        let rig = QuadrupedRig::wolf().solve(wolf_quadruped_targets(ai.run_phase, ai.mood));
        let shoulder_drive =
            ((rig.front_left.joint.y + rig.front_right.joint.y) * 0.5 - 0.28).clamp(-0.08, 0.08);
        let ground_y = terrain.as_ref().map_or(0.0, |terrain| {
            terrain.ground_height(Vec3::new(flat.x, 0.0, flat.z))
        });
        let target_position = Vec3::new(flat.x, ground_y + lope, flat.z);
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
        rotation.0 = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pounce_pitch);
    }
}

pub fn animate_boss(
    time: Res<Time>,
    state: Res<LivingSceneState>,
    mut bosses: Query<
        (
            &BossActor,
            &Health,
            &mut Transform,
            Option<&mut HitReaction>,
        ),
        Without<SlashFx>,
    >,
    mut slashes: Query<(Entity, &mut Transform), With<SlashFx>>,
    mut impacts: Query<
        (Entity, &mut Transform, &mut HitImpactFx),
        (With<HitImpactFx>, Without<BossActor>, Without<SlashFx>),
    >,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (boss, health, mut transform, reaction) in &mut bosses {
        let idle = state.elapsed * 0.8;
        let _intent = decide_creature_intent(
            CreatureAiProfile::dragon(),
            AiContext {
                self_pos: transform.translation,
                threat: None,
                food: &[],
                prey: &[],
            },
        );
        let virtual_rig = DragonRig::hoplite_boss().solve(dragon_boss_targets(idle));
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
        transform.translation =
            boss.base + orbit_bob(state.elapsed, 0.0, 0.45, 0.30, 0.12) + hit_offset;
        let health_scale = if health.is_dead() { 0.72 } else { 1.0 };
        transform.scale = Vec3::splat(0.92 * hit_pulse * health_scale)
            * Vec3::new(1.0, 1.0 + leg_drive * 0.08, 1.0 + leg_drive * 0.05);
        transform.rotation = Quat::from_rotation_y(idle.sin() * 0.18)
            * Quat::from_rotation_x(wing_lift * 0.04 + hit_tilt);
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

fn rabbit_quadruped_targets(phase: f32, mood: RabbitMood) -> QuadrupedTargets {
    let stride = match mood {
        RabbitMood::Flee => 0.16,
        RabbitMood::Forage => 0.12,
        RabbitMood::Graze => 0.04,
        RabbitMood::Idle => 0.06,
    };
    let front = phase.sin() * stride;
    let hind = -phase.sin() * stride;
    let lift = hop_height(phase, 0.08);
    QuadrupedTargets {
        front_left_foot: Vec3::new(-0.12, 0.02 + lift * 0.35, -0.30 + front),
        front_right_foot: Vec3::new(0.12, 0.02, -0.30 - front),
        hind_left_foot: Vec3::new(-0.18, 0.02, 0.34 + hind),
        hind_right_foot: Vec3::new(0.18, 0.02 + lift * 0.45, 0.34 - hind),
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
    let front = phase.sin() * stride;
    let hind = -phase.sin() * stride;
    let lift = match mood {
        WolfMood::Idle => 0.02,
        WolfMood::Chase => 0.10,
        WolfMood::Pounce => 0.16,
    };
    QuadrupedTargets {
        front_left_foot: Vec3::new(-0.20, 0.04 + front.max(0.0) * lift, -0.50 + front),
        front_right_foot: Vec3::new(0.20, 0.04 + (-front).max(0.0) * lift, -0.50 - front),
        hind_left_foot: Vec3::new(-0.22, 0.04 + hind.max(0.0) * lift, 0.44 + hind),
        hind_right_foot: Vec3::new(0.22, 0.04 + (-hind).max(0.0) * lift, 0.44 - hind),
        front_left_pole: Vec3::new(-0.34, 0.34, -0.58),
        front_right_pole: Vec3::new(0.34, 0.34, -0.58),
        hind_left_pole: Vec3::new(-0.38, 0.32, 0.58),
        hind_right_pole: Vec3::new(0.38, 0.32, 0.58),
    }
}

fn dragon_boss_targets(phase: f32) -> DragonTargets {
    let wing = phase.sin() * 0.28;
    DragonTargets {
        front_left_foot: Vec3::new(-0.55, 0.06, -0.78),
        front_right_foot: Vec3::new(0.55, 0.06, -0.78),
        hind_left_foot: Vec3::new(-0.70, 0.05, 0.78),
        hind_right_foot: Vec3::new(0.70, 0.05, 0.78),
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
