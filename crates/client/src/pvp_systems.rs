

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::Predicted;
use lk2_core::combat::{AttackType, CombatIntent, InputBuffer};
use lk2_core::protocol::PlayerAction;
use lk2_core::protocol::components::{CombatReady, Health};
use lk2_core::protocol::messages::{AttackInput, DamageResult, HitConfirm, KnockbackEvent};
use lk2_core::pvp::FixedTick;
use lk2_core::pvp::components::{CombatState, VisualEffectEvent};

pub fn collect_local_input(
    tick: Res<FixedTick>,

    input_manager: Option<ResMut<ActionState<PlayerAction>>>,
    player_transform: Query<&Transform, With<Camera>>,
    mut writer: MessageWriter<AttackInput>,
    combat: Query<&CombatState>,
    mut last_attack_was_sent: Local<bool>,
) {
    let Some(input_manager) = input_manager else {
        return;
    };

    let attack_pressed = input_manager.just_pressed(&PlayerAction::Attack);

    if !attack_pressed {
        *last_attack_was_sent = false;
        return;
    }

    let combat = combat.iter().next();
    let on_cooldown = combat.map(|c| c.attack_cooldown_timer > 0.0).unwrap_or(false);
    if on_cooldown || *last_attack_was_sent {
        return;
    }

    let forward: Vec3 = player_transform.iter().next().map(|t| *t.forward()).unwrap_or(Vec3::Z);

    let is_falling = true;

    let tick_val = tick.0;
    let combo_count = combat.map(|c| c.combo_count + 1).unwrap_or(0) as u8;

    writer.write(AttackInput { tick: tick_val, input_dir: forward, is_falling, combo_count });

    *last_attack_was_sent = true;
}

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

            combat.attack_cooldown_timer = 0.625;
            combat.is_attacking = true;
            combat.last_attack_tick = input.tick;
            combat.combo_count = input.combo_count;

            effect_writer.write(VisualEffectEvent::SwingSword);
        }
    }
}

pub fn on_hit_confirm(
    mut confirms: MessageReader<HitConfirm>,
    mut effect_writer: MessageWriter<VisualEffectEvent>,
) {
    for confirm in confirms.read() {

        let target =
            Entity::from_raw_u32(confirm.victim_id.to_bits() as u32).unwrap_or(Entity::PLACEHOLDER);
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

        effect_writer.write(VisualEffectEvent::ScreenShake);
    }
}

pub fn on_knockback_event(
    mut knockbacks: MessageReader<KnockbackEvent>,
    mut velocities: Query<(Entity, &mut LinearVelocity)>,
    mut effect_writer: MessageWriter<VisualEffectEvent>,
) {
    for kb in knockbacks.read() {
        let target =
            Entity::from_raw_u32(kb.victim_id.to_bits() as u32).unwrap_or(Entity::PLACEHOLDER);
        if let Ok((_, mut vel)) = velocities.get_mut(target) {

            vel.0 = kb.velocity;
        }
        effect_writer.write(VisualEffectEvent::KnockbackApplied { target, velocity: kb.velocity });
    }
}

pub fn on_damage_result(
    mut results: MessageReader<DamageResult>,
    mut healths: Query<&mut Health, With<Predicted>>,
    mut hud_text: Query<&mut Text, With<super::HealthHudMarker>>,
) {
    for result in results.read() {

        if let Ok(mut health) = healths.single_mut() {
            health.0 = result.new_health;
        }

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
                info!("⚔ 挥剑动画");
            }
            VisualEffectEvent::Hit { target, hit_pos, .. } => {
                info!("💥 命中 at {:?}", hit_pos);
                if let Ok(tf) = transforms.get(*target) {
                    spawn_hit_particle(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        tf.translation,
                        Color::srgb(1.0, 0.1, 0.1),
                    );
                }
            }
            VisualEffectEvent::CriticalHit { target, hit_pos, .. } => {
                info!("💥 暴击 at {:?}", hit_pos);
                if let Ok(tf) = transforms.get(*target) {
                    spawn_hit_particle(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        tf.translation,
                        Color::srgb(1.0, 0.85, 0.2),
                    );
                    spawn_hit_particle(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        tf.translation + Vec3::Y * 0.5,
                        Color::srgb(1.0, 0.55, 0.0),
                    );
                }
            }
            VisualEffectEvent::KnockbackApplied { target, velocity } => {
                info!("↔ 击退 applied to {:?}: {:?}", target, velocity);
            }
            VisualEffectEvent::ScreenShake => {

            }
        }
    }
}

fn spawn_hit_particle(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    color: Color,
) {

    let linear = color.to_linear();
    let emissive = bevy::color::LinearRgba::new(
        linear.red * 0.5,
        linear.green * 0.5,
        linear.blue * 0.5,
        linear.alpha,
    );
    let mesh_handle = meshes.add(bevy::prelude::Cuboid::new(0.15, 0.15, 0.15));
    let material_handle =
        materials.add(bevy::prelude::StandardMaterial { base_color: color, emissive, ..default() });
    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::from_translation(pos),
    ));
}

use crate::render::Player;
use crate::ui::ClientRunMode;

pub fn collect_combat_input_offline(
    keys: Res<ButtonInput<KeyCode>>,
    tick: Res<FixedTick>,
    run_mode: Res<ClientRunMode>,
    mut q_player: Query<&mut InputBuffer, With<Player>>,
) {
    if *run_mode != ClientRunMode::Offline {
        return;
    }
    let Ok(mut buf) = q_player.single_mut() else {
        return;
    };
    let tick = tick.0;

    if keys.just_pressed(KeyCode::KeyI) {
        buf.push(CombatIntent::Attack(AttackType::Light), tick);
    }

    if keys.just_pressed(KeyCode::KeyO) {
        buf.push(CombatIntent::Attack(AttackType::Heavy), tick);
    }

    if keys.just_pressed(KeyCode::KeyL) {
        buf.push(CombatIntent::Attack(AttackType::Thrust), tick);
    }

    if keys.just_pressed(KeyCode::KeyU) {
        buf.push(CombatIntent::BlockStart, tick);
    }
    if keys.just_released(KeyCode::KeyU) {
        buf.push(CombatIntent::BlockEnd, tick);
    }

    if keys.just_pressed(KeyCode::KeyY) {
        buf.push(CombatIntent::ParryAttempt, tick);
    }
}

use lk2_core::nation::{NationId, NationRegistry};
use lk2_core::objectives::{ObjectiveKind, Objectives};
use lk2_core::player::PlayerState;
use lk2_core::resource::GlobalResourcePool;

pub fn offline_found_nation_input(
    keys: Res<ButtonInput<KeyCode>>,
    run_mode: Res<ClientRunMode>,
    objectives: Res<Objectives>,
    mut player: ResMut<PlayerState>,
    mut nations: ResMut<NationRegistry>,
    mut pool: ResMut<GlobalResourcePool>,
) {
    if *run_mode != ClientRunMode::Offline {
        return;
    }
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }

    let wants_found = objectives
        .current()
        .map(|o| matches!(o.kind, ObjectiveKind::FoundNation) && !o.done)
        .unwrap_or(false);
    if !wants_found {
        return;
    }
    if player.nation_id.is_some() {
        return;
    }
    if !nations.can_found_new() {
        info!("[F] 国旗已满 8,无法再创国");
        return;
    }
    let cost = nations.next_flag_cost() as i64;
    let flag_count = nations.flag_count;
    match nations.found(
        &mut pool,
        0,
        format!("PlayerNation#{}", flag_count + 1),
        player.block_pos,
        0,
    ) {
        Ok(NationId(id)) => {
            player.nation_id = Some(NationId(id));
            player.nations_founded += 1;
            info!("[F] 创国成功 id={} cost={}", id, cost);
        }
        Err(e) => {
            warn!("[F] 创国失败: {}", e);
        }
    }
}
