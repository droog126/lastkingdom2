//! Bevy systems that drive the combat state machine each fixed tick.

use bevy::prelude::*;

use super::attack::{AttackState, InputBuffer};
use super::events::{CombatEvent, CombatIntent};
use super::resolve::{resolve_hit, sweep_hits};
use super::state::{BlockState, Downed, Knockback, ParryWindow, Stamina, StunSource, StunState};
use crate::pvp::{FixedTick, Health, SimpleWeapon};

pub fn regen_stamina_system(
    fixed_time: Res<Time<Fixed>>,
    mut q: Query<(&mut Stamina, Option<&StunState>, Option<&Downed>)>,
) {
    let dt = fixed_time.delta_secs();
    for (mut sta, stun, downed) in q.iter_mut() {
        let in_combat =
            stun.map(|s| s.stunned).unwrap_or(false) || downed.map(|d| d.downed).unwrap_or(false);
        if in_combat {
            sta.current = (sta.current + sta.regen_per_sec * 0.5 * dt).min(sta.max);
        } else {
            sta.regen(dt);
        }
    }
}

pub fn tick_parry_window_system(fixed_time: Res<Time<Fixed>>, mut q: Query<&mut ParryWindow>) {
    let dt = fixed_time.delta_secs();
    for mut p in q.iter_mut() {
        p.tick(dt);
    }
}

pub fn tick_stun_system(fixed_time: Res<Time<Fixed>>, mut q: Query<&mut StunState>) {
    let dt = fixed_time.delta_secs();
    for mut s in q.iter_mut() {
        s.tick(dt);
    }
}

pub fn tick_knockback_system(
    fixed_time: Res<Time<Fixed>>,
    mut q: Query<(&mut Knockback, &mut Transform)>,
) {
    let dt = fixed_time.delta_secs();
    for (mut kb, mut xf) in q.iter_mut() {
        let v = kb.tick(dt);
        if v == Vec3::ZERO {
            continue;
        }
        xf.translation.x += v.x * dt;
        xf.translation.z += v.z * dt;
    }
}

pub fn tick_downed_system(fixed_time: Res<Time<Fixed>>, mut q: Query<(&mut Downed, &mut Health)>) {
    let dt = fixed_time.delta_secs();
    for (mut down, mut hp) in q.iter_mut() {
        if down.tick(dt) {
            let target = hp.max * down.revive_hp_ratio;
            let need = (target - hp.current).max(0.0);
            hp.heal(need);
        }
    }
}

pub fn tick_attack_state_system(fixed_time: Res<Time<Fixed>>, mut q: Query<&mut AttackState>) {
    let dt = fixed_time.delta_secs();
    for mut att in q.iter_mut() {
        att.tick(dt);
    }
}

pub fn process_combat_intents_system(
    fixed_tick: Res<FixedTick>,
    mut q: Query<(
        &mut InputBuffer,
        &mut AttackState,
        &mut BlockState,
        &mut ParryWindow,
        &Stamina,
        &StunState,
        &Downed,
    )>,
) {
    for (mut buf, mut att, mut block, mut parry, sta, stun, down) in q.iter_mut() {
        let intents = buf.drain_fresh(fixed_tick.0);
        for intent in intents {
            match intent {
                CombatIntent::Attack(kind) => {
                    let _ = att.try_start(kind, sta, stun, down);
                }
                CombatIntent::BlockStart => {
                    block.start();

                    parry.begin();
                }
                CombatIntent::BlockEnd => {
                    block.stop();
                }
                CombatIntent::ParryAttempt => {
                    parry.begin();
                }
            }
        }
    }
}

pub fn process_attack_hits_system(
    fixed_tick: Res<FixedTick>,
    mut queries: ParamSet<(
        Query<(
            Entity,
            &Transform,
            &mut AttackState,
            &mut Stamina,
            &mut StunState,
            &SimpleWeapon,
        )>,
        Query<(
            Entity,
            &Transform,
            &mut Health,
            &mut Stamina,
            &mut BlockState,
            &mut ParryWindow,
            &mut StunState,
            &mut Knockback,
        )>,
    )>,
) {
    let attackers: Vec<Entity> = {
        let q0 = queries.p0();
        q0.iter()
            .filter(|(_, _, att, _, _, _)| att.is_active())
            .map(|(e, _, _, _, _, _)| e)
            .collect()
    };

    for attacker_entity in attackers {
        let attacker_info = {
            let q0 = queries.p0();
            let Ok((_, xf, att, sta, _, weapon)) = q0.get(attacker_entity) else {
                continue;
            };
            if !att.is_active() {
                continue;
            }
            let kind = match att.current_kind() {
                Some(k) => k,
                None => continue,
            };
            Some((
                xf.translation,
                (*xf.forward()).into(),
                weapon.damage,
                weapon.reach,
                weapon.sweep_angle_deg * 0.5,
                kind,
                sta.current,
            ))
        };
        let (
            atk_pos,
            atk_forward,
            weapon_damage,
            weapon_reach,
            sweep_half_angle,
            attack_kind,
            sta_now,
        ) = match attacker_info {
            Some(v) => v,
            None => continue,
        };

        let atk_cost = attack_kind.stamina_cost();

        {
            let mut q0 = queries.p0();
            let Ok((_, _, mut att, mut sta, _, _)) = q0.get_mut(attacker_entity) else {
                continue;
            };
            if sta_now < atk_cost {
                att.current = None;
                continue;
            }
            if let Some(a) = att.current.as_mut() {
                if a.hit_apps == 0 {
                    sta.consume(atk_cost);
                    a.hit_apps = 1;
                }
            }
        }

        let mut attacker_stun_apply: Option<(StunSource, f32)> = None;

        {
            let mut q1 = queries.p1();
            for (
                target_entity,
                tgt_xf,
                mut tgt_hp,
                mut tgt_sta,
                mut tgt_block,
                mut tgt_parry,
                mut tgt_stun,
                mut tgt_kb,
            ) in q1.iter_mut()
            {
                if target_entity == attacker_entity {
                    continue;
                }
                let delta = tgt_xf.translation - atk_pos;
                let dist_sq = delta.x * delta.x + delta.z * delta.z;
                if dist_sq > 64.0 {
                    continue;
                }
                if !sweep_hits(
                    atk_pos,
                    atk_forward,
                    tgt_xf.translation,
                    weapon_reach,
                    sweep_half_angle,
                ) {
                    continue;
                }

                let (events, actual_damage) = resolve_hit(
                    attacker_entity,
                    target_entity,
                    attack_kind,
                    weapon_damage,
                    &mut tgt_parry,
                    &mut tgt_block,
                    &mut tgt_stun,
                    &mut tgt_sta,
                    &mut tgt_kb,
                    atk_pos,
                    atk_forward,
                    tgt_xf.translation,
                    weapon_reach,
                );

                if actual_damage > 0.0 {
                    tgt_hp.damage(actual_damage, fixed_tick.0, 6);
                }

                for evt in &events {
                    if let CombatEvent::Stunned {
                        entity,
                        source,
                        duration_secs,
                    } = evt
                    {
                        if *entity == attacker_entity {
                            attacker_stun_apply = Some((*source, *duration_secs));
                        }
                    }
                }

                {
                    let mut q0 = queries.p0();
                    if let Ok((_, _, mut att, _, _, _)) = q0.get_mut(attacker_entity) {
                        if let Some(a) = att.current.as_mut() {
                            if a.hit_apps <= 1 {
                                a.hit_apps = 2;
                            }
                        }
                    }
                }

                break;
            }
        }

        if let Some((source, secs)) = attacker_stun_apply {
            let mut q0 = queries.p0();
            if let Ok((_, _, _, _, mut stun, _)) = q0.get_mut(attacker_entity) {
                stun.apply(secs, source);
            }
        }
    }
}

pub fn emit_combat_events_system(_: MessageWriter<CombatEvent>) {}
