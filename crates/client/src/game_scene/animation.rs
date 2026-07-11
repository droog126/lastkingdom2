//! Per-frame animation systems: drifting clouds and rain, growing grass and
//! berries, foraging rabbits, idle/squash boss pose, and ground darkening
//! after rain.

use bevy::prelude::*;

use super::creature_ai::{decide_creature_intent, AiContext, AiMood, AiTarget, CreatureAiProfile};
use super::offline::OfflineNature;
use super::procedural_motion::{
    heading_yaw, hop_height, idle_drift, orbit_bob, smooth_follow_alpha, wind_scale, wind_sway,
    ProceduralTreeSway,
};
use super::state::{
    BerryBush, BossActor, Cloud, GrassTuft, LivingSceneState, PlayerActor, Rabbit, RabbitAi,
    RabbitMood, RainDrop, SceneMaterials, SlashFx,
};
use super::util::smoothstep;
use lk2_core::pvp::Health;

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
    pub fruit: u32,
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
            value: target.fruit as f32,
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
    players: Query<&Transform, (With<PlayerActor>, Without<Rabbit>)>,
    mut rabbits: Query<(&Rabbit, &mut RabbitAi, &mut Transform), Without<PlayerActor>>,
) {
    let player_pos = players.single().ok().map(|transform| transform.translation);
    let food_targets = nature
        .snapshot
        .detailed_ecology
        .berries
        .iter()
        .map(|berry| RabbitFoodTarget {
            position: Vec3::new(berry.x, 0.0, berry.z),
            fruit: berry.fruit,
        })
        .collect::<Vec<_>>();

    for (rabbit, mut ai, mut transform) in &mut rabbits {
        let Some(snapshot) = nature
            .snapshot
            .detailed_ecology
            .rabbits
            .iter()
            .find(|snapshot| snapshot.id == rabbit.id)
        else {
            transform.scale = Vec3::ZERO;
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
        let flat = transform.translation.lerp(desired, alpha);
        let hop = match ai.mood {
            RabbitMood::Graze => hop_height(ai.hop_phase * 2.0, 0.05),
            RabbitMood::Idle => hop_height(ai.hop_phase, 0.06),
            RabbitMood::Forage => hop_height(ai.hop_phase, 0.16),
            RabbitMood::Flee => hop_height(ai.hop_phase, 0.24),
        };
        transform.translation = Vec3::new(flat.x, hop, flat.z);
        transform.scale = Vec3::splat(match ai.mood {
            RabbitMood::Flee => 1.02,
            RabbitMood::Graze => 0.86,
            _ => 0.92,
        });
        let yaw = heading_yaw(
            transform.translation,
            desired,
            (state.elapsed * 0.7 + rabbit.phase).sin() * 0.45,
        );
        let graze_pitch = if ai.mood == RabbitMood::Graze {
            0.35
        } else {
            0.0
        };
        transform.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(graze_pitch);
    }
}

pub fn animate_boss(
    state: Res<LivingSceneState>,
    mut bosses: Query<(&BossActor, &Health, &mut Transform), Without<SlashFx>>,
    mut slashes: Query<(Entity, &mut Transform), With<SlashFx>>,
    mut commands: Commands,
) {
    for (boss, health, mut transform) in &mut bosses {
        let idle = state.elapsed * 0.8;
        transform.translation = boss.base + orbit_bob(state.elapsed, 0.0, 0.45, 0.30, 0.12);
        let hit_pulse = if state.attack_flash > 0.0 { 1.08 } else { 1.0 };
        let health_scale = if health.is_dead() { 0.72 } else { 1.0 };
        transform.scale = Vec3::splat(0.92 * hit_pulse * health_scale);
        transform.rotation = Quat::from_rotation_y(idle.sin() * 0.18);
    }
    for (entity, mut transform) in &mut slashes {
        transform.scale *= 0.88;
        if transform.scale.max_element() < 0.10 {
            commands.entity(entity).despawn();
        }
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
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(mut material) = materials.get_mut(&scene_materials.ground) else {
        return;
    };
    let wet = smoothstep(nature.snapshot.atmosphere.cumulative_rainfall / 20.0);
    material.base_color = Color::srgb(0.28 - wet * 0.05, 0.49 + wet * 0.06, 0.22 + wet * 0.02);
    material.perceptual_roughness = 0.94 - wet * 0.16;
}
