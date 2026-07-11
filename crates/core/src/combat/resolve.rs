//! Pure combat resolution helpers.
//!
//! Free functions consumed by both the combat systems and direct callers
//! (e.g. tests, scripted encounters). These do not read or mutate any Bevy
//! world state; they only operate on the [`BlockState`], [`ParryWindow`],
//! [`StunState`], [`Stamina`], and [`Knockback`] values passed in.

use bevy::prelude::*;

use super::attack::AttackType;
use super::events::CombatEvent;
use super::state::{BlockState, Knockback, ParryWindow, Stamina, StunSource, StunState};

pub fn sweep_hits(
    attacker_pos: Vec3,
    attacker_forward: Vec3,
    target_pos: Vec3,
    reach: f32,
    sweep_half_angle_deg: f32,
) -> bool {
    let delta = target_pos - attacker_pos;
    let dist = Vec3::new(delta.x, 0.0, delta.z).length();
    if dist > reach {
        return false;
    }

    if dist < 0.001 {
        return true;
    }
    let to_target = Vec3::new(delta.x, 0.0, delta.z).normalize();
    let forward = Vec3::new(attacker_forward.x, 0.0, attacker_forward.z).normalize_or_zero();
    if forward == Vec3::ZERO {
        return false;
    }
    let dot = forward.dot(to_target).clamp(-1.0, 1.0);
    let angle_rad = dot.acos();
    let half_angle_rad = sweep_half_angle_deg.to_radians();
    angle_rad <= half_angle_rad
}

#[allow(clippy::too_many_arguments)]
pub fn resolve_hit(
    attacker: Entity,
    defender: Entity,
    attack: AttackType,
    weapon_damage: f32,
    defender_parry: &mut ParryWindow,
    defender_block: &mut BlockState,
    defender_stun: &mut StunState,
    defender_stamina: &mut Stamina,
    defender_knockback: &mut Knockback,
    attacker_pos: Vec3,
    _attacker_forward: Vec3,
    defender_pos: Vec3,
    _reach: f32,
) -> (Vec<CombatEvent>, f32) {
    let mut events = Vec::new();
    let raw = weapon_damage * attack.damage_multiplier();

    if defender_parry.try_parry() {
        defender_parry.consume_on_success();

        events.push(CombatEvent::Parried { attacker, defender });
        events.push(CombatEvent::Stunned {
            entity: attacker,
            source: StunSource::Parried,
            duration_secs: 0.6,
        });

        return (events, 0.0);
    }

    if defender_block.blocking {
        let actual = defender_block.apply_hit(raw, defender_stamina);
        events.push(CombatEvent::Blocked { attacker, defender });
        events.push(CombatEvent::DamageDealt {
            attacker,
            victim: defender,
            attack,
            damage: actual,
        });

        if matches!(attack, AttackType::Heavy) {
            let dir = (defender_pos - attacker_pos).normalize_or_zero();
            defender_knockback.apply(dir, 0.5, 0.20);
            events.push(CombatEvent::Knockback {
                victim: defender,
                magnitude: 0.5,
            });
        }
        return (events, actual);
    }

    events.push(CombatEvent::DamageDealt {
        attacker,
        victim: defender,
        attack,
        damage: raw,
    });

    if attack.knockback_strength() > 0.0 {
        let dir = (defender_pos - attacker_pos).normalize_or_zero();
        defender_knockback.apply(dir, attack.knockback_strength() * weapon_damage * 0.5, 0.30);
        events.push(CombatEvent::Knockback {
            victim: defender,
            magnitude: attack.knockback_strength() * weapon_damage * 0.5,
        });
    }

    if matches!(attack, AttackType::Heavy) {
        defender_stun.apply(0.4, StunSource::HeavyHit);
        events.push(CombatEvent::Stunned {
            entity: defender,
            source: StunSource::HeavyHit,
            duration_secs: 0.4,
        });
    }

    (events, raw)
}
