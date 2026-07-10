use lk2_core::match_state::MatchClock;
use lk2_core::protection::{Protection, can_attack};
use lk2_core::protocol::ControlChannel;
use lk2_core::protocol::components::{CombatReady, Health, KnockbackImmunity};
use lk2_core::protocol::messages::{AttackInput, DamageResult, HitConfirm, KnockbackEvent};
use lk2_core::pvp::FixedTick;
use lk2_core::pvp::{
    CombatState, DamageEvent, Hitbox, PositionHistory, PositionSnapshot, WeaponStats,
};
use lk2_core::world::World as GameWorld;

use crate::los::line_of_sight;

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use lightyear::prelude::PeerId;

use std::collections::VecDeque;

pub struct ServerPvPPlugin;

impl Plugin for ServerPvPPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (record_position_history,));
    }
}

pub fn record_position_history(
    tick: Res<FixedTick>,
    transforms: Query<(Entity, &Transform, &LinearVelocity)>,
    mut histories: Query<(Entity, &mut PositionHistory)>,
) {
    let tick_val = tick.0;
    for (entity, transform, velocity) in transforms.iter() {
        if let Ok((_, mut hist)) = histories.get_mut(entity) {
            hist.push(PositionSnapshot {
                tick: tick_val,
                translation: transform.translation,
                rotation: transform.rotation,
                velocity: velocity.0,
            });
        }
    }
}

pub fn read_attack_inputs(
    tick: Res<FixedTick>,
    mut receivers: Query<
        &mut lightyear::prelude::MessageReceiver<AttackInput>,
        With<lightyear_connection::client_of::ClientOf>,
    >,
    mut combat_states: Query<(Entity, &mut CombatState)>,
    mut attack_queue: Local<VecDeque<(Entity, AttackInput)>>,
) {
    let tick_val = tick.0;

    for mut receiver in &mut receivers {
        for input in receiver.receive() {
            if !input.is_finite() {
                warn!("ignoring non-finite AttackInput");
                continue;
            }
            for (entity, mut combat) in combat_states.iter_mut() {
                if combat.attack_cooldown_timer > 0.0 {
                    continue;
                }
                combat.is_attacking = true;
                combat.last_attack_tick = tick_val;
                combat.attack_cooldown_timer = 0.625;
                combat.combo_count = input.combo_count;

                attack_queue.push_back((entity, input.clone()));
            }
        }
    }
}

pub fn broadcast_pvp_messages(
    mut hit_confirms: MessageReader<HitConfirm>,
    mut knockbacks: MessageReader<KnockbackEvent>,
    mut damage_results: MessageReader<DamageResult>,
    server_q: Query<&lightyear::prelude::Server, With<lightyear_connection::server::Started>>,
    mut sender: lightyear::prelude::ServerMultiMessageSender<()>,
) {
    let Ok(server) = server_q.single() else {
        return;
    };
    let target = lightyear::prelude::NetworkTarget::All;
    for message in hit_confirms.read() {
        let _ = sender.send::<_, ControlChannel>(message, server, &target);
    }
    for message in knockbacks.read() {
        let _ = sender.send::<_, ControlChannel>(message, server, &target);
    }
    for message in damage_results.read() {
        let _ = sender.send::<_, ControlChannel>(message, server, &target);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn melee_hit_registration(
    mut attack_queue: Local<VecDeque<(Entity, AttackInput)>>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut hit_confirms: MessageWriter<HitConfirm>,
    mut knockback_events: MessageWriter<KnockbackEvent>,
    attackers: Query<(&Transform, &WeaponStats, Entity), With<CombatState>>,
    victims: Query<
        (
            Entity,
            &Transform,
            &Hitbox,
            &Health,
            &PositionHistory,
            Option<&KnockbackImmunity>,
        ),
        Without<CombatState>,
    >,
    voxel_world: Res<GameWorld>,
    tick: Res<FixedTick>,
    match_clock: Res<MatchClock>,
    protections: Query<&Protection>,
    client_id_map: Local<std::collections::HashMap<Entity, PeerId>>,
) {
    if attack_queue.is_empty() {
        return;
    }

    let tick_val = tick.0;
    let queue: Vec<_> = attack_queue.drain(..).collect();

    for (attacker_entity, attack_input) in queue {
        let Ok((attacker_tf, weapon, _)) = attackers.get(attacker_entity) else {
            continue;
        };

        let eye_pos = attacker_tf.translation + Vec3::Y * 1.62;
        let forward = attack_input.input_dir.normalize();

        for (victim_entity, _, hitbox, health, history, _kb_immune) in victims.iter() {
            if !can_attack(
                protections.get(attacker_entity).ok(),
                protections.get(victim_entity).ok(),
                &match_clock,
            ) {
                continue;
            }
            if health.0 <= 0.0 {
                continue;
            }

            let Some(victim_snap) = history.query(attack_input.tick) else {
                continue;
            };

            let victim_center = victim_snap.translation + hitbox.offset;

            let dist = eye_pos.distance(victim_center);
            let reach_limit = weapon.reach + hitbox.half_extents.length();
            if dist > reach_limit {
                continue;
            }

            let dir_to_victim = (victim_center - eye_pos).normalize();
            let angle = forward.angle_between(dir_to_victim);
            let half_sweep = weapon.sweep_angle_deg.to_radians() / 2.0;
            if angle > half_sweep {
                continue;
            }

            if !voxel_world.in_bounds(
                victim_center.x as i32,
                victim_center.y as i32,
                victim_center.z as i32,
            ) {
                continue;
            }
            let los = line_of_sight(&voxel_world, eye_pos, victim_center, 0.05);
            if los.blocked {
                continue;
            }

            let height_diff = attacker_tf.translation.y - victim_snap.translation.y;
            let is_critical = height_diff > 0.5 && attack_input.is_falling;
            let damage = if is_critical {
                weapon.damage * 1.5
            } else {
                weapon.damage
            };

            let kb_dir = (victim_snap.translation - attacker_tf.translation).normalize_or_zero();
            let kb_horizontal = Vec3::new(kb_dir.x, 0.0, kb_dir.z);
            let knockback = kb_horizontal * weapon.knockback + Vec3::Y * 0.4;

            damage_events.write(DamageEvent {
                attacker: attacker_entity,
                victim: victim_entity,
                damage,
                knockback,
                is_critical,
                hit_location: victim_center,
                server_tick: tick_val,
            });

            let victim_client_id =
                client_id_map.get(&victim_entity).copied().unwrap_or(PeerId::Server);
            hit_confirms.write(HitConfirm {
                victim_id: victim_client_id,
                damage,
                is_critical,
                hit_pos: victim_center,
                server_tick: tick_val,
            });

            knockback_events.write(KnockbackEvent {
                victim_id: victim_client_id,
                velocity: knockback,
                server_tick: tick_val,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn apply_damage_and_knockback(
    mut damage_reader: MessageReader<DamageEvent>,
    mut healths: Query<(Entity, &mut Health)>,
    mut velocities: Query<(Entity, &mut LinearVelocity)>,
    mut kb_immunity: Query<(Entity, &mut KnockbackImmunity)>,
    mut damage_results: MessageWriter<DamageResult>,
    tick: Res<FixedTick>,
    match_clock: Res<MatchClock>,
    protections: Query<&Protection>,
    client_id_map: Local<std::collections::HashMap<Entity, PeerId>>,
) {
    let tick_val = tick.0;

    for event in damage_reader.read() {
        if !can_attack(
            protections.get(event.attacker).ok(),
            protections.get(event.victim).ok(),
            &match_clock,
        ) {
            continue;
        }
        if let Ok((_, mut health)) = healths.get_mut(event.victim) {
            health.0 = (health.0 - event.damage).max(0.0);
        }

        let is_immune = kb_immunity.get(event.victim).map(|(_, k)| k.0 > 0.0).unwrap_or(false);

        if !is_immune {
            if let Ok((_, mut vel)) = velocities.get_mut(event.victim) {
                vel.0 += event.knockback;
            }
            if let Ok((_, mut kbi)) = kb_immunity.get_mut(event.victim) {
                kbi.0 = 0.3;
            }
        }

        if let Ok((_, health)) = healths.get(event.victim) {
            let victim_client_id =
                client_id_map.get(&event.victim).copied().unwrap_or(PeerId::Server);
            damage_results.write(DamageResult {
                victim_id: victim_client_id,
                new_health: health.0,
                is_dead: health.0 <= 0.0,
                server_tick: tick_val,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn damage_test_app() -> App {
        let mut app = App::new();
        app.add_message::<DamageEvent>()
            .add_message::<DamageResult>()
            .init_resource::<FixedTick>()
            .init_resource::<MatchClock>()
            .add_systems(Update, apply_damage_and_knockback);
        app.world_mut().resource_mut::<MatchClock>().wall_secs = 300.0;
        app.world_mut().resource_mut::<MatchClock>().refresh_phase();
        app
    }

    fn send_damage(app: &mut App, attacker: Entity, victim: Entity) {
        app.world_mut().write_message(DamageEvent {
            attacker,
            victim,
            damage: 25.0,
            knockback: Vec3::ZERO,
            is_critical: false,
            hit_location: Vec3::ZERO,
            server_tick: 1,
        });
        app.update();
    }

    #[test]
    fn protected_victim_does_not_take_authoritative_damage() {
        let mut app = damage_test_app();
        let attacker = app.world_mut().spawn_empty().id();
        let victim = app.world_mut().spawn((Health(100.0), Protection::mid_join())).id();
        send_damage(&mut app, attacker, victim);

        assert_eq!(app.world().get::<Health>(victim).unwrap().0, 100.0);
    }

    #[test]
    fn protected_attacker_does_not_deal_authoritative_damage() {
        let mut app = damage_test_app();
        let attacker = app.world_mut().spawn(Protection::mid_join()).id();
        let victim = app.world_mut().spawn(Health(100.0)).id();
        send_damage(&mut app, attacker, victim);

        assert_eq!(app.world().get::<Health>(victim).unwrap().0, 100.0);
    }

    #[test]
    fn unprotected_attack_deals_authoritative_damage_after_opening_phase() {
        let mut app = damage_test_app();
        let attacker = app.world_mut().spawn_empty().id();
        let victim = app.world_mut().spawn(Health(100.0)).id();
        send_damage(&mut app, attacker, victim);

        assert_eq!(app.world().get::<Health>(victim).unwrap().0, 75.0);
    }
}

pub fn expire_knockback_immunity(time: Res<Time>, mut kb_immunity: Query<&mut KnockbackImmunity>) {
    let dt = time.delta_secs();
    for mut kbi in kb_immunity.iter_mut() {
        kbi.0 = (kbi.0 - dt).max(0.0);
    }
}

pub fn tick_combat_cooldowns(time: Res<Time>, mut combat: Query<&mut CombatState>) {
    let dt = time.delta_secs();
    for mut c in combat.iter_mut() {
        c.attack_cooldown_timer = (c.attack_cooldown_timer - dt).max(0.0);
        c.is_attacking = c.attack_cooldown_timer > 0.0;
    }
}
