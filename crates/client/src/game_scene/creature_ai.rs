//! Shared lightweight creature AI decisions for presentation-side actors.

use bevy::prelude::*;

pub const WOLF_ATTACK_SEPARATION: f32 = 1.05;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiMood {
    Idle,
    Forage,
    Graze,
    Flee,
    Chase,
    Attack,
}

#[derive(Clone, Copy, Debug)]
pub struct AiTarget {
    pub position: Vec3,
    pub value: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct AiContext<'a> {
    pub self_pos: Vec3,
    pub threat: Option<Vec3>,
    pub food: &'a [AiTarget],
    pub prey: &'a [AiTarget],
}

#[derive(Clone, Copy, Debug)]
pub struct CreatureAiProfile {
    pub threat_radius: f32,
    pub flee_distance: f32,
    pub forage_distance: f32,
    pub attack_range: f32,
    pub aggression: f32,
    pub feeds: bool,
}

impl CreatureAiProfile {
    pub const fn rabbit() -> Self {
        Self {
            threat_radius: 3.2,
            flee_distance: 4.0,
            forage_distance: 0.65,
            attack_range: 0.0,
            aggression: 0.0,
            feeds: true,
        }
    }

    pub const fn dragon() -> Self {
        Self {
            threat_radius: 0.0,
            flee_distance: 0.0,
            forage_distance: 0.0,
            attack_range: 5.5,
            aggression: 1.0,
            feeds: false,
        }
    }

    pub const fn wolf() -> Self {
        Self {
            threat_radius: 0.0,
            flee_distance: 0.0,
            forage_distance: 0.0,
            attack_range: WOLF_ATTACK_SEPARATION,
            aggression: 1.0,
            feeds: false,
        }
    }

    pub const fn deer() -> Self {
        Self {
            threat_radius: 5.0,
            flee_distance: 6.0,
            forage_distance: 0.9,
            attack_range: 0.0,
            aggression: 0.0,
            feeds: true,
        }
    }

    pub const fn fox() -> Self {
        Self {
            threat_radius: 0.0,
            flee_distance: 0.0,
            forage_distance: 0.0,
            attack_range: 1.25,
            aggression: 0.65,
            feeds: false,
        }
    }

    pub const fn bear() -> Self {
        Self {
            threat_radius: 0.0,
            flee_distance: 0.0,
            forage_distance: 0.9,
            attack_range: 0.0,
            aggression: 0.0,
            feeds: true,
        }
    }
}

/// Return a flat-ground target that keeps two presentation actors separated.
/// This does not replace authoritative collision or combat rules.
pub fn separated_target_position(self_pos: Vec3, target: Vec3, minimum_distance: f32) -> Vec3 {
    let minimum_distance = minimum_distance.max(0.0);
    let delta = Vec3::new(target.x - self_pos.x, 0.0, target.z - self_pos.z);
    let distance = delta.length();
    if distance <= 0.0001 {
        return target - Vec3::X * minimum_distance;
    }
    if distance >= minimum_distance {
        return target;
    }
    target - delta / distance * minimum_distance
}

#[derive(Clone, Copy, Debug)]
pub struct AiIntent {
    pub mood: AiMood,
    pub target: Vec3,
}

pub fn decide_creature_intent(profile: CreatureAiProfile, context: AiContext<'_>) -> AiIntent {
    if profile.aggression > 0.0 {
        if let Some(target) = nearest_valued_target(context.self_pos, context.prey) {
            return AiIntent {
                mood: if context.self_pos.distance(target.position) <= profile.attack_range {
                    AiMood::Attack
                } else {
                    AiMood::Chase
                },
                target: target.position,
            };
        }
    }

    if let Some(threat) = context.threat {
        let away = Vec3::new(
            context.self_pos.x - threat.x,
            0.0,
            context.self_pos.z - threat.z,
        );
        let distance = away.length();
        if distance < profile.threat_radius {
            let direction = if distance > 0.001 {
                away / distance
            } else {
                Vec3::X
            };
            return AiIntent {
                mood: AiMood::Flee,
                target: context.self_pos + direction * profile.flee_distance,
            };
        }
    }

    if profile.feeds {
        if let Some(food) = nearest_valued_target(context.self_pos, context.food) {
            return AiIntent {
                mood: if context.self_pos.distance(food.position) <= profile.forage_distance {
                    AiMood::Graze
                } else {
                    AiMood::Forage
                },
                target: food.position,
            };
        }
    }

    AiIntent {
        mood: AiMood::Idle,
        target: context.self_pos,
    }
}

fn nearest_valued_target(self_pos: Vec3, targets: &[AiTarget]) -> Option<AiTarget> {
    targets
        .iter()
        .copied()
        .filter(|target| target.value > 0.0)
        .min_by(|a, b| {
            self_pos
                .distance_squared(a.position)
                .partial_cmp(&self_pos.distance_squared(b.position))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}
