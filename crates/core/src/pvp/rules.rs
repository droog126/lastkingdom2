use bevy::prelude::*;

use super::components::SimpleWeapon;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeleeHit {
    pub damage: f32,
    pub knockback: Vec3,
    pub hit_pos: Vec3,
}

pub fn resolve_melee_attack(
    attacker_pos: Vec3,
    attack_direction: Vec3,
    victim_pos: Vec3,
    weapon: SimpleWeapon,
) -> Option<MeleeHit> {
    if !attacker_pos.is_finite()
        || !attack_direction.is_finite()
        || !victim_pos.is_finite()
        || !weapon.reach.is_finite()
        || !weapon.damage.is_finite()
        || !weapon.knockback.is_finite()
        || weapon.reach <= 0.0
        || weapon.damage <= 0.0
    {
        return None;
    }

    let forward = Vec3::new(attack_direction.x, 0.0, attack_direction.z).normalize_or_zero();
    let to_victim = Vec3::new(
        victim_pos.x - attacker_pos.x,
        0.0,
        victim_pos.z - attacker_pos.z,
    );
    let distance = to_victim.length();
    if forward == Vec3::ZERO || distance <= f32::EPSILON || distance > weapon.reach {
        return None;
    }

    let direction = to_victim / distance;
    let half_sweep_cos = (weapon.sweep_angle_deg.to_radians() * 0.5).cos();
    if forward.dot(direction) < half_sweep_cos {
        return None;
    }

    Some(MeleeHit {
        damage: weapon.damage,
        knockback: direction * weapon.knockback + Vec3::Y * (weapon.knockback * 0.1),
        hit_pos: victim_pos,
    })
}

pub fn apply_damage(current_health: f32, damage: f32) -> f32 {
    if !current_health.is_finite() || !damage.is_finite() {
        return current_health;
    }
    (current_health - damage.max(0.0)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_melee_attack_hits_inside_reach_and_sweep() {
        let weapon = SimpleWeapon::default();
        let hit = resolve_melee_attack(Vec3::ZERO, Vec3::Z, Vec3::new(0.5, 0.0, 2.5), weapon)
            .expect("target inside the basic melee arc should be hit");

        assert_eq!(hit.damage, weapon.damage);
        assert!(hit.knockback.z > 0.0);
    }

    #[test]
    fn simple_melee_attack_rejects_targets_outside_arc_or_reach() {
        let weapon = SimpleWeapon::default();

        assert!(
            resolve_melee_attack(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, 4.0), weapon).is_none()
        );
        assert!(
            resolve_melee_attack(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, -1.0), weapon).is_none()
        );
    }

    #[test]
    fn authoritative_damage_is_clamped_at_zero() {
        assert_eq!(apply_damage(5.0, 7.0), 0.0);
        assert_eq!(apply_damage(5.0, 2.0), 3.0);
    }
}
