//! PvP 服务端系统（权威判定）
//!
//! 本文件是 server crate 的副本 (从 src/pvp/systems_server.rs 迁出)。
//! 全部 import 走 lk2_core (不再依赖 src/ 里的旧 components / FixedTick)。
//!
//! 运行在服务端 FixedUpdate 中，所有伤害 / 击退必须经过这里。
//!
//! 执行顺序（已在 crates/server/src/main.rs 中通过 .chain() 保证）：
//!   1. record_position_history   — 记录玩家位置历史
//!   2. read_attack_inputs       — 读取客户端 AttackInput
//!   3. melee_hit_registration    — 权威命中判定
//!   4. apply_damage_and_knockback — 应用伤害与击退
//!   5. expire_knockback_immunity — 清除击退免疫
//!   6. tick_combat_cooldowns    — 冷却衰减

use lk2_core::pvp::{
    CombatState, DamageEvent, Hitbox, PositionHistory, PositionSnapshot, WeaponStats,
};
use lk2_core::pvp::FixedTick;
use lk2_core::protocol::components::{CombatReady, Health, KnockbackImmunity};
use lk2_core::protocol::messages::{AttackInput, DamageResult, HitConfirm, KnockbackEvent};
use lk2_core::world::World as GameWorld;

use crate::los::line_of_sight;

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use lightyear::prelude::PeerId;

use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// 0. ServerPvPPlugin
// ---------------------------------------------------------------------------

pub struct ServerPvPPlugin;

impl Plugin for ServerPvPPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                record_position_history,
            ),
        );
        // 其余 4 个 system 由 main.rs 显式 .chain() 注册
    }
}

// ---------------------------------------------------------------------------
// 1. 记录位置历史（每 FixedUpdate 必跑）
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 2. 读取攻击输入
// ---------------------------------------------------------------------------

pub fn read_attack_inputs(
    tick: Res<FixedTick>,
    mut events: MessageReader<AttackInput>,
    mut combat_states: Query<(Entity, &mut CombatState)>,
    mut attack_queue: Local<VecDeque<(Entity, AttackInput)>>,
) {
    let tick_val = tick.0;

    for input in events.read() {
        for (entity, mut combat) in combat_states.iter_mut() {
            if combat.attack_cooldown_timer > 0.0 {
                continue;
            }
            combat.is_attacking = true;
            combat.last_attack_tick = tick_val;
            combat.attack_cooldown_timer = 0.625; // 铁剑速度
            combat.combo_count = input.combo_count;

            attack_queue.push_back((
                entity,
                AttackInput {
                    tick: input.tick,
                    input_dir: input.input_dir,
                    is_falling: input.is_falling,
                    combo_count: input.combo_count,
                },
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// 3. 权威命中判定（核心）
// ---------------------------------------------------------------------------

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

        for (victim_entity, _, hitbox, health, history, kb_immune) in victims.iter() {
            if health.0 <= 0.0 {
                continue;
            }

            // 延迟补偿
            let Some(victim_snap) = history.query(attack_input.tick) else {
                continue;
            };

            let victim_center = victim_snap.translation + hitbox.offset;

            // Reach 检查
            let dist = eye_pos.distance(victim_center);
            let reach_limit = weapon.reach + hitbox.half_extents.length();
            if dist > reach_limit {
                continue;
            }

            // 扇形角度检查
            let dir_to_victim = (victim_center - eye_pos).normalize();
            let angle = forward.angle_between(dir_to_victim);
            let half_sweep = weapon.sweep_angle_deg.to_radians() / 2.0;
            if angle > half_sweep {
                continue;
            }

            // 视线检查
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

            // 跳劈判定
            let height_diff = attacker_tf.translation.y - victim_snap.translation.y;
            let is_critical = height_diff > 0.5 && attack_input.is_falling;
            let damage = if is_critical { weapon.damage * 1.5 } else { weapon.damage };

            // 击退方向
            let kb_dir = (victim_snap.translation - attacker_tf.translation).normalize_or_zero();
            let kb_horizontal = Vec3::new(kb_dir.x, 0.0, kb_dir.z);
            let knockback = kb_horizontal * weapon.knockback + Vec3::Y * 0.4;

            // 写 DamageEvent
            damage_events.write(DamageEvent {
                attacker: attacker_entity,
                victim: victim_entity,
                damage,
                knockback,
                is_critical,
                hit_location: victim_center,
                server_tick: tick_val,
            });

            // 写 HitConfirm (Unreliable, 给 client 看击打特效)
            let victim_client_id = client_id_map
                .get(&victim_entity)
                .copied()
                .unwrap_or(PeerId::Server);
            hit_confirms.write(HitConfirm {
                victim_id: victim_client_id,
                damage,
                is_critical,
                hit_pos: victim_center,
                server_tick: tick_val,
            });

            // 写 KnockbackEvent
            knockback_events.write(KnockbackEvent {
                victim_id: victim_client_id,
                velocity: knockback,
                server_tick: tick_val,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// 4. 应用伤害与击退
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn apply_damage_and_knockback(
    mut damage_reader: MessageReader<DamageEvent>,
    mut healths: Query<(Entity, &mut Health)>,
    mut velocities: Query<(Entity, &mut LinearVelocity)>,
    mut kb_immunity: Query<(Entity, &mut KnockbackImmunity)>,
    mut damage_results: MessageWriter<DamageResult>,
    tick: Res<FixedTick>,
    client_id_map: Local<std::collections::HashMap<Entity, PeerId>>,
) {
    let tick_val = tick.0;

    for event in damage_reader.read() {
        if let Ok((_, mut health)) = healths.get_mut(event.victim) {
            health.0 = (health.0 - event.damage).max(0.0);
        }

        let is_immune = kb_immunity
            .get(event.victim)
            .map(|(_, k)| k.0 > 0.0)
            .unwrap_or(false);

        if !is_immune {
            if let Ok((_, mut vel)) = velocities.get_mut(event.victim) {
                vel.0 += event.knockback;
            }
            if let Ok((_, mut kbi)) = kb_immunity.get_mut(event.victim) {
                kbi.0 = 0.3;
            }
        }

        if let Ok((_, health)) = healths.get(event.victim) {
            let victim_client_id = client_id_map
                .get(&event.victim)
                .copied()
                .unwrap_or(PeerId::Server);
            damage_results.write(DamageResult {
                victim_id: victim_client_id,
                new_health: health.0,
                is_dead: health.0 <= 0.0,
                server_tick: tick_val,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// 5. 击退免疫消退
// ---------------------------------------------------------------------------

pub fn expire_knockback_immunity(
    time: Res<Time>,
    mut kb_immunity: Query<&mut KnockbackImmunity>,
) {
    let dt = time.delta_secs();
    for mut kbi in kb_immunity.iter_mut() {
        kbi.0 = (kbi.0 - dt).max(0.0);
    }
}

// ---------------------------------------------------------------------------
// 6. 冷却衰减
// ---------------------------------------------------------------------------

pub fn tick_combat_cooldowns(time: Res<Time>, mut combat: Query<&mut CombatState>) {
    let dt = time.delta_secs();
    for mut c in combat.iter_mut() {
        c.attack_cooldown_timer = (c.attack_cooldown_timer - dt).max(0.0);
        c.is_attacking = c.attack_cooldown_timer > 0.0;
    }
}
