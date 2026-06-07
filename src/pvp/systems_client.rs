//! PvP 客户端系统（本地预测 + 视觉反馈）
//!
//! 运行在客户端 Update 中：
//!   1. collect_local_input    — 采集本地按键，生成 AttackInput 发送给服务端
//!   2. client_attack_predict  — 本地预测攻击动画和冷却（不等服务端）
//!   3. on_hit_confirm         — 收到 HitConfirm → 播放命中粒子 / 音效
//!   4. on_knockback_event     — 收到 KnockbackEvent → 应用击退
//!   5. on_damage_result       — 收到 DamageResult → 更新血量 UI
//!   6. apply_local_knockback  — 对本地 predicted entity 应用击退
//!   7. trigger_visual_effects — 播放挥剑 / 命中 / 屏幕震动

use crate::network::protocols::components::{CombatReady, Health};
use crate::network::protocols::msg::{AttackInput, DamageResult, HitConfirm, KnockbackEvent};
use crate::network::PlayerAction;
use crate::pvp::components::{CombatState, VisualEffectEvent};
use crate::pvp::FixedTick;
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use lightyear::prelude::Predicted;
use leafwing_input_manager::prelude::*;

// ---------------------------------------------------------------------------
// 1. 采集本地输入 → 发送 AttackInput
// ---------------------------------------------------------------------------

pub fn collect_local_input(
    tick: Res<FixedTick>,
    // 兜底：demo 里没人插 PlayerAction 的 InputMap，没资源时静默 no-op
    mut input_manager: Option<ResMut<ActionState<PlayerAction>>>,
    player_transform: Query<&Transform, With<Camera>>,
    mut writer: MessageWriter<AttackInput>,
    combat: Query<&CombatState>,
    mut last_attack_was_sent: Local<bool>,
) {
    let Some(mut input_manager) = input_manager else { return; };
    // 检测 Attack 按钮（鼠标左键）
    let attack_pressed = input_manager.just_pressed(&PlayerAction::Attack);

    // 只有在冷却结束且刚按下时才能发送攻击
    if !attack_pressed {
        *last_attack_was_sent = false;
        return;
    }

    let combat = combat.iter().next();
    let on_cooldown = combat.map(|c| c.attack_cooldown_timer > 0.0).unwrap_or(false);
    if on_cooldown || *last_attack_was_sent {
        return;
    }

    // 从相机朝向获取攻击方向
    // bevy 0.18: `Transform::forward()` 返回 `Dir3`（方向向量包装），不是 `Vec3`
    let forward: Vec3 = player_transform
        .iter()
        .next()
        .map(|t| *t.forward())
        .unwrap_or(Vec3::Z);

    // is_falling = 没有在地上（简化：用 Velocity.y 判断）
    let is_falling = true; // TODO: 接入 LinearVelocity.y

    let tick_val = tick.0;
    let combo_count = combat.map(|c| c.combo_count + 1).unwrap_or(0) as u8;

    writer.write(AttackInput {
        tick: tick_val,
        input_dir: forward,
        is_falling,
        combo_count,
    });

    *last_attack_was_sent = true;
}

// ---------------------------------------------------------------------------
// 2. 本地攻击预测（客户端立即响应，不等服务端）
// ---------------------------------------------------------------------------

pub fn client_attack_predict(
    mut local_attacks: MessageReader<AttackInput>,
    mut combat_states: Query<&mut CombatState, With<Predicted>>,
    mut effect_writer: MessageWriter<VisualEffectEvent>,
) {
    for input in local_attacks.read() {
        if let Ok(mut combat) = combat_states.single_mut() {
            if combat.attack_cooldown_timer > 0.0 {
                continue;
            }

            // 立刻重置冷却（本地预测）
            combat.attack_cooldown_timer = 0.625;
            combat.is_attacking = true;
            combat.last_attack_tick = input.tick;
            combat.combo_count = input.combo_count;

            // 立刻播放挥剑动画（零延迟手感）
            effect_writer.write(VisualEffectEvent::SwingSword);
        }
    }
}

// ---------------------------------------------------------------------------
// 3. 收到命中确认
// ---------------------------------------------------------------------------

pub fn on_hit_confirm(
    mut confirms: MessageReader<HitConfirm>,
    mut effect_writer: MessageWriter<VisualEffectEvent>,
) {
    for confirm in confirms.read() {
        // PeerId → Entity 转换（PeerId 的 bits 编码成 Entity 标识；不是真实 entity，仅作占位）
        let target = Entity::from_raw_u32(confirm.victim_id.to_bits() as u32)
            .unwrap_or(Entity::PLACEHOLDER);
        if confirm.is_critical {
            effect_writer.write(VisualEffectEvent::CriticalHit {
                target,
                damage: confirm.damage,
                hit_pos: confirm.hit_pos,
            });
        } else {
            effect_writer.write(VisualEffectEvent::Hit {
                target,
                damage: confirm.damage,
                is_critical: false,
                hit_pos: confirm.hit_pos,
            });
        }

        // 命中时屏幕轻微震动
        effect_writer.write(VisualEffectEvent::ScreenShake);
    }
}

// ---------------------------------------------------------------------------
// 4. 收到击退事件（对 interpolated 实体立即应用）
// ---------------------------------------------------------------------------

pub fn on_knockback_event(
    mut knockbacks: MessageReader<KnockbackEvent>,
    mut velocities: Query<(Entity, &mut LinearVelocity)>,
    mut effect_writer: MessageWriter<VisualEffectEvent>,
) {
    for kb in knockbacks.read() {
        let target = Entity::from_raw_u32(kb.victim_id.to_bits() as u32)
            .unwrap_or(Entity::PLACEHOLDER);
        if let Ok((_, mut vel)) = velocities.get_mut(target) {
            // 服务端已算过，这里直接覆盖（客户端相信服务端）
            vel.0 = kb.velocity;
        }
        effect_writer.write(VisualEffectEvent::KnockbackApplied {
            target,
            velocity: kb.velocity,
        });
    }
}

// ---------------------------------------------------------------------------
// 5. 收到伤害结果（可靠，更新血量 UI）
// ---------------------------------------------------------------------------

pub fn on_damage_result(
    mut results: MessageReader<DamageResult>,
    mut healths: Query<&mut Health, With<Predicted>>,
    mut hud_text: Query<&mut Text, With<super::HealthHudMarker>>,
) {
    for result in results.read() {
        // 更新本地 predicted 血量
        if let Ok(mut health) = healths.single_mut() {
            health.0 = result.new_health;
        }

        // 更新 HUD
        if let Ok(mut text) = hud_text.single_mut() {
            let hp_str = if result.is_dead {
                "☠ DEAD".to_string()
            } else {
                format!("❤ {:.0} / 20", result.new_health)
            };
            text.0 = hp_str;
        }
    }
}

// ---------------------------------------------------------------------------
// 6. 视觉特效播放系统
// ---------------------------------------------------------------------------

/// HUD 血量文字 marker
#[derive(Component)]
pub struct HealthHudMarker;

pub fn trigger_visual_effects(
    mut effects: MessageReader<VisualEffectEvent>,
    mut commands: Commands,
    transforms: Query<&Transform, Without<Camera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for effect in effects.read() {
        match effect {
            VisualEffectEvent::SwingSword => {
                // TODO: 播放挥剑动画 entity（剑模型旋转）
                info!("⚔ 挥剑动画");
            }
            VisualEffectEvent::Hit { target, hit_pos, .. } => {
                // TODO: spawn 命中粒子（红色方块，0.3s 后自动 despawn）
                info!("💥 命中 at {:?}", hit_pos);
                if let Ok(tf) = transforms.get(*target) {
                    spawn_hit_particle(&mut commands, &mut meshes, &mut materials, tf.translation, Color::srgb(1.0, 0.1, 0.1));
                }
            }
            VisualEffectEvent::CriticalHit { target, hit_pos, .. } => {
                info!("💥 暴击 at {:?}", hit_pos);
                if let Ok(tf) = transforms.get(*target) {
                    spawn_hit_particle(&mut commands, &mut meshes, &mut materials, tf.translation, Color::srgb(1.0, 0.85, 0.2));
                    spawn_hit_particle(&mut commands, &mut meshes, &mut materials, tf.translation + Vec3::Y * 0.5, Color::srgb(1.0, 0.55, 0.0));
                }
            }
            VisualEffectEvent::KnockbackApplied { target, velocity } => {
                info!("↔ 击退 applied to {:?}: {:?}", target, velocity);
            }
            VisualEffectEvent::ScreenShake => {
                // 屏幕震动：bevy 0.18 严格 ParamSet 检查导致 Query<&mut Transform, With<Camera>>
                // 跟 Without<Camera> 在同一系统里冲突。先禁掉以防 panic，后续再独立 system 处理。
                // 视觉损失：挥剑屏幕不抖
            }
        }
    }
}

/// Spawn 一个命中粒子方块（红/金色，0.3s 后消失）
fn spawn_hit_particle(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    color: Color,
) {
    // Color 自身不实现 Mul<f32>；先把通道算好再转 LinearRgba
    let linear = color.to_linear();
    let emissive = bevy::color::LinearRgba::new(
        linear.red * 0.5,
        linear.green * 0.5,
        linear.blue * 0.5,
        linear.alpha,
    );
    let mesh_handle = meshes.add(bevy::prelude::Cuboid::new(0.15, 0.15, 0.15));
    let material_handle = materials.add(bevy::prelude::StandardMaterial {
        base_color: color,
        emissive,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::from_translation(pos),
    ));
}
