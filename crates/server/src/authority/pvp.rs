use std::collections::VecDeque;

use bevy::prelude::*;
use lightyear::prelude::PeerId;
use lk2_core::match_state::MatchClock;
use lk2_core::protection::{Protection, can_attack};
use lk2_core::protocol::ControlChannel;
use lk2_core::protocol::messages::{AttackInput, AttackResult};
use lk2_core::pvp::{
    CREATURE_HIT_INVULNERABILITY_TICKS, FixedTick, Health, Hitbox, PvpCombatant, SimpleWeapon,
    increment_fixed_tick, resolve_melee_attack,
};
use lk2_core::world::World as GameWorld;

#[path = "../los.rs"]
mod los;

use los::line_of_sight;

#[derive(Resource, Default)]
struct PendingAttacks(VecDeque<(Entity, AttackInput)>);

pub struct SimplePvpAuthorityPlugin;

impl Plugin for SimplePvpAuthorityPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AttackInput>()
            .add_message::<AttackResult>()
            .init_resource::<FixedTick>()
            .init_resource::<PendingAttacks>()
            .add_systems(
                FixedUpdate,
                (
                    increment_fixed_tick,
                    tick_combatant_cooldowns,
                    receive_attack_inputs,
                    resolve_attacks,
                    broadcast_attack_results,
                )
                    .chain(),
            );
    }
}

fn tick_combatant_cooldowns(time: Res<Time<Fixed>>, mut combatants: Query<&mut PvpCombatant>) {
    for mut combatant in &mut combatants {
        combatant.tick(time.delta_secs());
    }
}

fn receive_attack_inputs(
    mut receivers: Query<
        (
            Entity,
            &mut lightyear::prelude::MessageReceiver<AttackInput>,
        ),
        With<lightyear_connection::client_of::ClientOf>,
    >,
    mut combatants: Query<(
        Entity,
        &lightyear::prelude::ControlledBy,
        &mut PvpCombatant,
        &SimpleWeapon,
        &Health,
    )>,
    mut pending: ResMut<PendingAttacks>,
) {
    for (connection_entity, mut receiver) in &mut receivers {
        for input in receiver.receive() {
            if !input.is_finite() {
                warn!("[pvp] dropping non-finite attack input");
                continue;
            }
            let Some((attacker, _, mut combatant, weapon, health)) = combatants
                .iter_mut()
                .find(|(_, owner, _, _, _)| owner.owner == connection_entity)
            else {
                continue;
            };
            if health.is_dead() {
                continue;
            }
            if combatant.begin_attack(*weapon) {
                pending.0.push_back((attacker, input));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_attacks(
    mut pending: ResMut<PendingAttacks>,
    mut combatants: ParamSet<(
        Query<(&Transform, &SimpleWeapon), With<PvpCombatant>>,
        Query<
            (
                Entity,
                &mut Transform,
                &Hitbox,
                &mut Health,
                Option<&lightyear::prelude::ControlledBy>,
            ),
            With<PvpCombatant>,
        >,
    )>,
    tick: Res<FixedTick>,
    world: Res<GameWorld>,
    match_clock: Res<MatchClock>,
    protections: Query<&Protection>,
    remote_ids: Query<&lightyear::prelude::RemoteId>,
    mut results: MessageWriter<AttackResult>,
) {
    while let Some((attacker, input)) = pending.0.pop_front() {
        let Some((attacker_pos, weapon)) = combatants
            .p0()
            .get(attacker)
            .ok()
            .map(|(transform, weapon)| (transform.translation, *weapon))
        else {
            continue;
        };

        let mut victims = combatants.p1();
        for (victim, mut transform, hitbox, mut health, owner) in &mut victims {
            if victim == attacker || health.is_dead() {
                continue;
            }
            if !can_attack(
                protections.get(attacker).ok(),
                protections.get(victim).ok(),
                &match_clock,
            ) {
                continue;
            }

            let victim_center = transform.translation + hitbox.center_offset;
            let weapon_with_hitbox = SimpleWeapon {
                reach: weapon.reach + hitbox.radius,
                ..weapon
            };
            let Some(hit) = resolve_melee_attack(
                attacker_pos + Vec3::Y * 0.9,
                input.input_dir,
                victim_center,
                weapon_with_hitbox,
            ) else {
                continue;
            };
            if line_of_sight(&world, attacker_pos + Vec3::Y * 0.9, victim_center, 0.05).blocked {
                continue;
            }

            let damage = health.damage(hit.damage, tick.0, CREATURE_HIT_INVULNERABILITY_TICKS);
            // A blocked hit is not a confirmed contact. Do not move the
            // victim or advertise knockback when its invulnerability window
            // rejected the damage; otherwise rapid attacks feel like phantom
            // impacts and the server/client states diverge.
            if damage > 0.0 {
                transform.translation += hit.knockback * 0.08;
            }
            let victim_id = owner
                .and_then(|owner| remote_ids.get(owner.owner).ok())
                .map_or(PeerId::Server, |remote| remote.0.clone());
            results.write(AttackResult {
                victim_id,
                damage,
                new_health: health.current,
                is_dead: health.is_dead(),
                hit_pos: hit.hit_pos,
                knockback: if damage > 0.0 {
                    hit.knockback
                } else {
                    Vec3::ZERO
                },
                server_tick: tick.0,
            });
            break;
        }
    }
}

fn broadcast_attack_results(
    mut results: MessageReader<AttackResult>,
    server: Query<&lightyear::prelude::Server, With<lightyear_connection::server::Started>>,
    mut sender: lightyear::prelude::ServerMultiMessageSender<()>,
) {
    let Ok(server) = server.single() else {
        return;
    };
    for result in results.read() {
        let _ = sender.send::<_, ControlChannel>(
            result,
            server,
            &lightyear::prelude::NetworkTarget::All,
        );
    }
}
