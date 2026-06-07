//! PvP 服务端系统（权威判定）
//!
//! 运行在服务端 FixedUpdate 中，所有伤害 / 击退必须经过这里。
//!
//! 执行顺序（已在 main.rs 中通过 .chain() 保证）：
//!   1. record_position_history   — 记录玩家位置历史
//!   2. read_attack_inputs       — 读取客户端 AttackInput
//!   3. melee_hit_registration    — 权威命中判定
//!   4. apply_damage_and_knockback — 应用伤害与击退
//!   5. expire_knockback_immunity — 清除击退免疫

use crate::network::protocols::components::{CombatReady, Health, KnockbackImmunity};
use crate::network::protocols::msg::{AttackInput, DamageResult, HitConfirm, KnockbackEvent};
use crate::pvp::components::{
    CombatState, DamageEvent, Hitbox, Ping, PositionHistory, PositionSnapshot, WeaponStats,
};
use crate::pvp::los::line_of_sight;
use crate::pvp::FixedTick;
use crate::world::World as GameWorld;
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use lightyear::prelude::PeerId;
use std::collections::VecDeque;

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
        // 找到对应的玩家 entity
        // 这里简化：直接遍历所有带 CombatState 的 entity，假设只有一个玩家
        for (entity, mut combat) in combat_states.iter_mut() {
            // 冷却检查
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

pub fn melee_hit_registration(
    mut attack_queue: Local<VecDeque<(Entity, AttackInput)>>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut hit_confirms: MessageWriter<HitConfirm>,
    mut knockback_events: MessageWriter<KnockbackEvent>,
    // 攻击者
    attackers: Query<(&Transform, &WeaponStats, Entity), With<CombatState>>,
    // 受害者（不包含攻击者自己）
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

    // 取出本帧所有攻击
    let queue: Vec<_> = attack_queue.drain(..).collect();

    for (attacker_entity, attack_input) in queue {
        let Ok((attacker_tf, weapon, _)) = attackers.get(attacker_entity) else {
            continue;
        };

        // 攻击者眼睛高度（体素游戏标准 1.62m）
        let eye_pos = attacker_tf.translation + Vec3::Y * 1.62;
        let forward = attack_input.input_dir.normalize();

        for (victim_entity, _, hitbox, health, history, kb_immune) in victims.iter() {
            if health.0 <= 0.0 {
                continue; // 已死
            }

            // --- 延迟补偿：回滚受害者到攻击时刻 ---
            // 简化：取最近快照（完整实现见下方的 `rollback_to_tick`）
            let Some(victim_snap) = history.query(attack_input.tick) else {
                continue; // 还没历史，先跳过
            };

            let victim_center = victim_snap.translation + hitbox.offset;

            // --- Reach 检查 ---
            let dist = eye_pos.distance(victim_center);
            let reach_limit = weapon.reach + hitbox.half_extents.length();
            if dist > reach_limit {
                continue;
            }

            // --- 扇形角度检查 ---
            let dir_to_victim = (victim_center - eye_pos).normalize();
            let angle = forward.angle_between(dir_to_victim);
            let half_sweep = weapon.sweep_angle_deg.to_radians() / 2.0;
            if angle > half_sweep {
                continue;
            }

            // --- 视线检查（DDA） ---
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

            // --- 跳劈判定（攻击者高于受害者 0.5m 且正在下落） ---
            let height_diff = attacker_tf.translation.y - victim_snap.translation.y;
            let is_critical = height_diff > 0.5 && attack_input.is_falling;
            let damage = if is_critical {
                weapon.damage * 1.5
            } else {
                weapon.damage
            };

            // --- 击退方向：水平远离攻击者 ---
            let kb_dir = (victim_snap.translation - attacker_tf.translation)
                .normalize_or_zero();
            let kb_horizontal = Vec3::new(kb_dir.x, 0.0, kb_dir.z);
            let knockback = kb_horizontal * weapon.knockback + Vec3::Y * 0.4;

            // --- 发送 DamageEvent（给 apply_damage_and_knockback） ---
            damage_events.write(DamageEvent {
                attacker: attacker_entity,
                victim: victim_entity,
                damage,
                knockback,
                is_critical,
                hit_location: victim_center,
                server_tick: tick_val,
            });

            // --- 发送 HitConfirm（给客户端，Unreliable） ---
            // PeerId 没有 Default impl；这里用 `PeerId::Server` 作为单 demo 下的占位受害者。
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

            // --- 发送 KnockbackEvent（给客户端，Unreliable + 立即发送） ---
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
        // 应用伤害
        if let Ok((_, mut health)) = healths.get_mut(event.victim) {
            health.0 = (health.0 - event.damage).max(0.0);
        }

        // 应用击退（只有没有免疫的才吃击退）
        let is_immune = kb_immunity
            .get(event.victim)
            .map(|(_, k)| k.0 > 0.0)
            .unwrap_or(false);

        if !is_immune {
            if let Ok((_, mut vel)) = velocities.get_mut(event.victim) {
                vel.0 += event.knockback;
            }
            // 写入 0.3s 击退免疫
            if let Ok((_, mut kbi)) = kb_immunity.get_mut(event.victim) {
                kbi.0 = 0.3;
            }
        }

        // 发送 DamageResult（Reliable）
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
// 辅助：冷却计时器衰减
// ---------------------------------------------------------------------------

pub fn tick_combat_cooldowns(time: Res<Time>, mut combat: Query<&mut CombatState>) {
    let dt = time.delta_secs();
    for mut c in combat.iter_mut() {
        c.attack_cooldown_timer = (c.attack_cooldown_timer - dt).max(0.0);
        if !c.is_attacking {
            // 攻击动画在 0.2s 后自动结束
        }
        c.is_attacking = c.attack_cooldown_timer > 0.0;
    }
}
