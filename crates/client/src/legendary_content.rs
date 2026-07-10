use bevy::prelude::*;
use bevy::text::{FontSize, FontSource, FontWeight};

use crate::GameplayFeedbackToast;
use crate::render::{CameraAngles, Player};
use crate::ui::ClientRunMode;
use lk2_core::combat::Health as CombatHealth;
use lk2_core::legendary::{
    CraftLegendaryError, DragonBossState, LegendaryLoadout, LegendaryWeapon, claim_dragon_loot,
    craft_legendary, resolve_reaper_strike,
};
use lk2_core::player::PlayerState;
use lk2_core::resource::ResourceKind;
use lk2_core::status::StatusSet;
use lk2_core::world::{World as GameWorld, player_spawn_position_near};

const DRAGON_ATTACK_RANGE_SQ: f32 = 24.0 * 24.0;
const REAPER_COOLDOWN_SECS: f32 = 8.0;
const HELD_WEAPON_SCALE: f32 = 0.34;

#[derive(Component)]
pub struct DragonBossVisual {
    base: Vec3,
}

#[derive(Component)]
pub struct HeldLegendaryVisual {
    weapon: LegendaryWeapon,
}

#[derive(Component)]
pub struct LegendaryHudText;

#[derive(Resource, Debug, Clone)]
pub struct LegendaryRuntime {
    pub player_statuses: StatusSet,
    pub reaper_cooldown_secs: f32,
    hit_pulse_secs: f32,
    auto_timer_secs: f32,
    auto_reaper_used: bool,
}

impl Default for LegendaryRuntime {
    fn default() -> Self {
        Self {
            player_statuses: StatusSet::default(),
            reaper_cooldown_secs: 0.0,
            hit_pulse_secs: 0.0,
            auto_timer_secs: 0.0,
            auto_reaper_used: false,
        }
    }
}

pub fn setup_legendary_content(
    mut commands: Commands,
    run_mode: Res<ClientRunMode>,
    game_world: Res<GameWorld>,
    player: Res<PlayerState>,
    asset_server: Res<AssetServer>,
    mut dragon: ResMut<DragonBossState>,
    mut loadout: ResMut<LegendaryLoadout>,
) {
    if *run_mode != ClientRunMode::Offline {
        return;
    }

    loadout.grant_and_equip(LegendaryWeapon::ReaperScythe);
    let candidate_x = player.block_pos[0] + 9;
    let candidate_z = player.block_pos[2] - 6;
    let (spawn_pos, block_pos) =
        player_spawn_position_near(&game_world, candidate_x, candidate_z, 8, 2).unwrap_or((
            player.pos + Vec3::new(9.0, 0.0, -6.0),
            [candidate_x, player.block_pos[1], candidate_z],
        ));
    *dragon = DragonBossState::new(block_pos);

    let face_player = (player.pos.x - spawn_pos.x).atan2(player.pos.z - spawn_pos.z);
    commands.spawn((
        WorldAssetRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("procedural/pretty/hoplite_ender_dragon.glb"),
        )),
        Transform::from_translation(spawn_pos)
            .with_rotation(Quat::from_rotation_y(face_player))
            .with_scale(Vec3::splat(0.92)),
        DragonBossVisual { base: spawn_pos },
    ));
    spawn_held_weapon(&mut commands, &asset_server, LegendaryWeapon::ReaperScythe);

    commands.spawn((
        Text::new(""),
        TextFont {
            font: FontSource::SystemUi,
            font_size: FontSize::Rem(0.86),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.78, 1.0)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node {
            position_type: PositionType::Absolute,
            top: px(14),
            left: Val::Percent(18.0),
            right: Val::Percent(18.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        LegendaryHudText,
    ));
}

fn spawn_held_weapon(commands: &mut Commands, asset_server: &AssetServer, weapon: LegendaryWeapon) {
    commands.spawn((
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(weapon.model_path()))),
        Transform::from_scale(Vec3::splat(HELD_WEAPON_SCALE)),
        HeldLegendaryVisual { weapon },
    ));
}

fn attack_distance_sq(player: Vec3, dragon: &DragonBossState) -> f32 {
    let dragon_pos = Vec3::new(
        dragon.block_pos[0] as f32 + 0.5,
        dragon.block_pos[1] as f32,
        dragon.block_pos[2] as f32 + 0.5,
    );
    Vec2::new(player.x - dragon_pos.x, player.z - dragon_pos.z).length_squared()
}

#[allow(clippy::too_many_arguments)]
pub fn legendary_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player: ResMut<PlayerState>,
    mut dragon: ResMut<DragonBossState>,
    mut loadout: ResMut<LegendaryLoadout>,
    mut runtime: ResMut<LegendaryRuntime>,
    mut toast: ResMut<GameplayFeedbackToast>,
    mut player_health: Query<&mut CombatHealth, With<Player>>,
) {
    runtime.reaper_cooldown_secs = (runtime.reaper_cooldown_secs - time.delta_secs()).max(0.0);
    runtime.hit_pulse_secs = (runtime.hit_pulse_secs - time.delta_secs()).max(0.0);
    runtime.player_statuses.tick(time.delta_secs());

    let auto_demo = std::env::args().any(|argument| argument == "--auto-demo");
    runtime.auto_timer_secs += time.delta_secs();
    let auto_attack = auto_demo && runtime.auto_timer_secs >= 1.0 && dragon.is_alive();
    if auto_attack {
        runtime.auto_timer_secs = 0.0;
    }
    let auto_reaper = auto_demo && !runtime.auto_reaper_used && dragon.is_alive();
    let in_range = attack_distance_sq(player.pos, &dragon) <= DRAGON_ATTACK_RANGE_SQ;

    if (keys.just_pressed(KeyCode::KeyO) || auto_reaper)
        && dragon.is_alive()
        && in_range
        && loadout.equipped == Some(LegendaryWeapon::ReaperScythe)
        && runtime.reaper_cooldown_secs <= 0.0
    {
        if let Ok(mut health) = player_health.single_mut() {
            let target_statuses = dragon.statuses.clone();
            let outcome = resolve_reaper_strike(
                &mut health,
                &mut runtime.player_statuses,
                &mut dragon.health,
                &target_statuses,
                0,
            );
            runtime.reaper_cooldown_secs = REAPER_COOLDOWN_SECS;
            runtime.hit_pulse_secs = 0.22;
            runtime.auto_reaper_used = true;
            show_toast(
                &mut toast,
                &time,
                true,
                format!(
                    "镰刀吸取 {:.0} 生命，复制 {}",
                    outcome.health_restored,
                    runtime.player_statuses.summary_zh()
                ),
            );
        }
    }

    if (keys.just_pressed(KeyCode::KeyI) || auto_attack) && dragon.is_alive() && in_range {
        let damage = if loadout.equipped == Some(LegendaryWeapon::DragonKatana) {
            38.0
        } else {
            26.0
        };
        let outcome = dragon.damage(damage);
        runtime.hit_pulse_secs = 0.18;
        show_toast(
            &mut toast,
            &time,
            true,
            format!(
                "命中末影龙 {:.0}，剩余 {:.0}",
                outcome.damage_dealt, outcome.remaining_health
            ),
        );
    }

    if !dragon.is_alive() && !dragon.loot_claimed {
        if let Ok(loot) = claim_dragon_loot(&mut dragon, &mut player.inventory) {
            show_toast(
                &mut toast,
                &time,
                true,
                format!(
                    "获得龙心 {}、龙鳞 {}",
                    loot.dragon_hearts, loot.dragon_scales
                ),
            );
        }
    }

    let auto_craft =
        auto_demo && !dragon.is_alive() && loadout.equipped != Some(LegendaryWeapon::DragonKatana);
    if keys.just_pressed(KeyCode::KeyH) || auto_craft {
        match craft_legendary(
            LegendaryWeapon::DragonKatana,
            &mut player.inventory,
            &mut loadout,
        ) {
            Ok(()) => show_toast(&mut toast, &time, true, "龙武士刀锻造完成".to_string()),
            Err(CraftLegendaryError::Missing(kind, amount)) => show_toast(
                &mut toast,
                &time,
                false,
                format!("缺少 {} x{}", kind.label_zh(), amount),
            ),
        }
    }
}

fn show_toast(toast: &mut GameplayFeedbackToast, time: &Time, ok: bool, summary: String) {
    toast.ok = ok;
    toast.summary = summary;
    toast.shown_at_secs = time.elapsed_secs();
}

pub fn sync_held_legendary_model(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loadout: Res<LegendaryLoadout>,
    visuals: Query<(Entity, &HeldLegendaryVisual)>,
) {
    let Some(equipped) = loadout.equipped else {
        for (entity, _) in &visuals {
            commands.entity(entity).despawn();
        }
        return;
    };
    if visuals.iter().any(|(_, visual)| visual.weapon == equipped) {
        return;
    }
    for (entity, _) in &visuals {
        commands.entity(entity).despawn();
    }
    spawn_held_weapon(&mut commands, &asset_server, equipped);
}

pub fn follow_held_legendary(
    player: Res<PlayerState>,
    angles: Res<CameraAngles>,
    mut visuals: Query<&mut Transform, With<HeldLegendaryVisual>>,
) {
    let (sin_yaw, cos_yaw) = angles.yaw.sin_cos();
    let right = Vec3::new(cos_yaw, 0.0, sin_yaw);
    let forward = Vec3::new(sin_yaw, 0.0, -cos_yaw);
    for mut transform in &mut visuals {
        transform.translation = player.pos + Vec3::Y * 0.38 + right * 0.48 + forward * 0.10;
        transform.rotation =
            Quat::from_rotation_y(angles.yaw) * Quat::from_rotation_z(-18.0_f32.to_radians());
        transform.scale = Vec3::splat(HELD_WEAPON_SCALE);
    }
}

pub fn animate_dragon_boss(
    time: Res<Time>,
    dragon: Res<DragonBossState>,
    runtime: Res<LegendaryRuntime>,
    mut visuals: Query<(&DragonBossVisual, &mut Transform)>,
) {
    for (visual, mut transform) in &mut visuals {
        if dragon.is_alive() {
            transform.translation.y = visual.base.y + (time.elapsed_secs() * 1.8).sin() * 0.10;
            let pulse = if runtime.hit_pulse_secs > 0.0 {
                1.06
            } else {
                1.0
            };
            transform.scale = Vec3::splat(0.92 * pulse);
        } else {
            transform.translation.y = visual.base.y - 0.45;
            transform.rotation = Quat::from_rotation_z(78.0_f32.to_radians());
            transform.scale = Vec3::splat(0.92);
        }
    }
}

pub fn update_legendary_hud(
    dragon: Res<DragonBossState>,
    player: Res<PlayerState>,
    loadout: Res<LegendaryLoadout>,
    runtime: Res<LegendaryRuntime>,
    mut text: Query<&mut Text, With<LegendaryHudText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let weapon = loadout.equipped.map_or("空手", LegendaryWeapon::label);
    let hearts = player.inventory.get(&ResourceKind::DragonHeart).copied().unwrap_or(0);
    let scales = player.inventory.get(&ResourceKind::DragonScale).copied().unwrap_or(0);
    text.0 = if dragon.is_alive() {
        format!(
            "末影龙  HP {:.0}/{:.0}  |  {}  |  状态 {}  |  自身 {}",
            dragon.health.current,
            dragon.health.max,
            weapon,
            dragon.statuses.summary_zh(),
            runtime.player_statuses.summary_zh(),
        )
    } else {
        format!("末影龙已击败  |  龙心 {hearts}  龙鳞 {scales}  |  {weapon}")
    };
}
