use bevy::prelude::*;

use crate::pvp_systems::HealthHudMarker;
use crate::render::{AnimalIndicatorText, NestIndicatorText, Player};
use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::combat::{
    AttackPhase, AttackState as CombatAttackState, Downed as CombatDowned,
    Health as CombatHealth, ParryWindow as CombatParryWindow, Stamina as CombatStamina,
    StunState as CombatStunState,
};
use lk2_core::diagnostics::SnapshotRole;
use lk2_core::match_state::MatchClock;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::protocol::components::GameplayHudState;
use lk2_core::resource::{GlobalResourcePool, ResourceKind};

#[derive(Component)]
pub struct HudText;

#[derive(Component)]
pub struct HudFooter;

#[derive(Component)]
pub struct HudHpText;

#[derive(Component)]
pub struct HudStaText;

#[derive(Component)]
pub struct HudPhaseText;

#[derive(Resource)]
pub struct UiFonts {
    pub cn: Handle<Font>,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientRunMode {
    Offline,
    Online,
}

impl ClientRunMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Offline => "OFFLINE",
            Self::Online => "ONLINE",
        }
    }

    pub fn state_role(self) -> &'static str {
        self.snapshot_role().as_str()
    }

    pub fn snapshot_role(self) -> SnapshotRole {
        match self {
            Self::Offline => SnapshotRole::ClientOffline,
            Self::Online => SnapshotRole::ClientOnline,
        }
    }
}

pub fn setup_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    let cn = asset_server.load("fonts/NotoSansCJKsc-Regular.otf");
    commands.insert_resource(UiFonts { cn });
}

pub fn setup_hud(mut commands: Commands, fonts: Res<UiFonts>) {
    commands.spawn((
        Text::new("WANGUO ORIGINS v0.4  loading..."),
        TextFont { font: fonts.cn.clone(), font_size: 16.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.92)),
        TextShadow { offset: Vec2::new(2.0, 2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
        Node { position_type: PositionType::Absolute, top: px(12), left: px(12), ..default() },
        HudText,
    ));

    let cross_size = 16.0_f32;
    let cross_thickness = 2.0_f32;
    let cross_offset = -cross_size / 2.0;
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: px(cross_size),
            height: px(cross_thickness),
            margin: UiRect {
                left: Val::Px(cross_offset),
                top: Val::Px(cross_offset + (cross_size - cross_thickness) / 2.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
            },
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: px(cross_thickness),
            height: px(cross_size),
            margin: UiRect {
                left: Val::Px(cross_offset + (cross_size - cross_thickness) / 2.0),
                top: Val::Px(cross_offset),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
            },
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
    ));

    commands.spawn((
        Text::new(""),
        TextFont { font: fonts.cn.clone(), font_size: 18.0, ..default() },
        TextColor(Color::srgb(0.95, 0.95, 0.7)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
        Node { position_type: PositionType::Absolute, bottom: px(12), left: px(12), ..default() },
        HudFooter,
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(56),
            left: px(0),
            right: px(0),
            height: px(36),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            Text::new(""),
            TextFont { font: fonts.cn.clone(), font_size: 14.0, ..default() },
            TextColor(Color::srgba(1.0, 0.9, 0.4, 0.7)), // 半透明,降低抢戏
            TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
            AnimalIndicatorText,
        )],
    ));

    // nest 方向指示器（紧贴动物指示器下方，y+36 → top=92，y 偏移 36 不重叠）
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(76),
            left: px(0),
            right: px(0),
            height: px(24),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            Text::new(""),
            TextFont { font: fonts.cn.clone(), font_size: 13.0, ..default() },
            TextColor(Color::srgba(1.0, 0.6, 0.4, 0.6)), // 半透明橙红
            TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
            NestIndicatorText,
        )],
    ));

    commands.spawn((
        Text::new("HP 100/100"),
        TextFont { font: fonts.cn.clone(), font_size: 20.0, ..default() },
        TextColor(Color::srgb(1.0, 0.4, 0.4)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(12), right: px(12), ..default() },
        HudHpText,
        HealthHudMarker,
    ));

    // STA 文本 (HP 下方, 24px 间距)
    commands.spawn((
        Text::new("STA 100/100"),
        TextFont { font: fonts.cn.clone(), font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.4, 0.8, 1.0)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(38), right: px(12), ..default() },
        HudStaText,
    ));

    // Phase 指示器 (STA 下方)
    commands.spawn((
        Text::new("Phase: --"),
        TextFont { font: fonts.cn.clone(), font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.9, 0.9, 0.5)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(60), right: px(12), ..default() },
        HudPhaseText,
    ));

    // 控制提示 (右下角, 避免与 HP/STA/Phase 重叠)
    commands.spawn((
        Text::new("I/O=Light/Heavy  L=Thrust\nU=Block  Y=Parry"),
        TextFont { font: fonts.cn.clone(), font_size: 11.0, ..default() },
        TextColor(Color::srgba(0.85, 0.85, 0.85, 0.85)),
        TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, bottom: px(56), right: px(12), ..default() },
    ));
}

pub fn update_hud(
    mut q_hud: ParamSet<(
        Query<&mut Text, With<HudText>>,
        Query<&mut Text, (With<HudFooter>, Without<HudText>)>,
        Query<&mut Text, With<HudHpText>>,
        Query<&mut Text, With<HudStaText>>,
        Query<&mut Text, With<HudPhaseText>>,
    )>,
    clock: Res<SimClock>,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    _obs: Res<TickObserver>,
    time: Res<Time>,
    run_mode: Res<ClientRunMode>,
    hud_state_q: Query<&GameplayHudState>,
    match_clock: Res<MatchClock>,
    q_player_combat: Query<(&CombatHealth, &CombatStamina, &CombatAttackState, &CombatParryWindow, &CombatStunState, &CombatDowned), With<Player>>,
) {
    let fps = (1.0 / time.delta_secs().max(0.001)).round() as i32;
    let hud_state = hud_state_q.iter().next();
    let _tick_value = hud_state.map(|s| s.tick).unwrap_or(clock.tick);
    let _block_pos = hud_state.map(|s| s.player_block_pos).unwrap_or(player.block_pos);
    let wood = hud_state.map(|s| s.pool_wood).unwrap_or(pool.get(ResourceKind::Wood));
    let food = hud_state.map(|s| s.pool_food).unwrap_or(pool.get(ResourceKind::Food));
    let apple = hud_state.map(|s| s.pool_apple).unwrap_or(pool.get(ResourceKind::Apple));
    let soul = hud_state.map(|s| s.pool_soul).unwrap_or(pool.get(ResourceKind::Soul));
    let flags = hud_state.map(|s| s.flag_count).unwrap_or(nations.flag_count);
    let monster_count = hud_state.map(|s| s.monster_count).unwrap_or(monsters.current_individuals);
    let _anomalies =
        hud_state.map(|s| s.observer_anomalies as usize).unwrap_or(_obs.anomalies.len());
    let _invariants = hud_state.map(|s| s.observer_invariant_violations).unwrap_or(0);

    if let Ok(mut text) = q_hud.p0().single_mut() {
        // 收敛 HUD：版本号 + fps + 资源 + 状态 + V2 阶段(quick win 验收)
        // 阶段: 4 段, 显示中文 + 距离本阶段结束的秒数 mm:ss
        let phase_label = match_clock.phase.label_zh();
        let phase_remaining = match_clock.phase_remaining_secs();
        let m = (phase_remaining / 60.0).floor() as i32;
        let s = (phase_remaining - m as f32 * 60.0).floor() as i32;
        let phase_line = format!("Phase: {}  T-{:02}:{:02}", phase_label, m, s);
        **text = format!(
            "WANGUO ORIGINS v0.4  ·  {}  ·  {fps} fps  ·  {phase_line}\n\
             Wood {}  Food {}  Apple {}  Soul {}\n\
             flags {}/{}  monsters {}",
            run_mode.label(),
            wood,
            food,
            apple,
            soul,
            flags,
            8,
            monster_count,
        );
    }

    let goal = 10;
    let status = if let Some(state) = hud_state {
        state.status_line.as_str()
    } else if wood >= goal {
        "press F to found nation"
    } else {
        ""
    };
    if let Ok(mut text) = q_hud.p1().single_mut() {
        **text = format!(
            "Goal: 10 wood   {wood}/{goal}\n\
             {status}",
        );
    }

    // ---- V2 战斗 HUD: HP / STA / Phase 实时显示 ----
    if let Ok((hp, sta, att, parry, stun, down)) = q_player_combat.single() {
        if let Ok(mut text) = q_hud.p2().single_mut() {
            let filled = (hp.ratio() * 10.0).clamp(0.0, 10.0) as i32;
            let bar = format!("{}{}",
                "\u{2588}".repeat(filled as usize),
                "\u{2591}".repeat((10 - filled) as usize));
            **text = format!("HP {}/{}  {}",
                hp.current as i32, hp.max as i32, bar);
        }
        if let Ok(mut text) = q_hud.p3().single_mut() {
            let filled = (sta.ratio() * 10.0).clamp(0.0, 10.0) as i32;
            let bar = format!("{}{}",
                "\u{2588}".repeat(filled as usize),
                "\u{2591}".repeat((10 - filled) as usize));
            **text = format!("STA {}/{}  {}  regen {:.0}/s",
                sta.current as i32, sta.max as i32, bar, sta.regen_per_sec);
        }
        if let Ok(mut text) = q_hud.p4().single_mut() {
            **text = if down.downed {
                format!("DOWNED  {:.1}s", down.timer)
            } else if stun.stunned {
                format!("STUNNED  {:.2}s", stun.stun_timer)
            } else if parry.parry_active {
                format!("PARRY {:.2}s", parry.parry_timer)
            } else if let Some(active) = att.current {
                let phase_str = match active.phase {
                    AttackPhase::Windup => "Windup",
                    AttackPhase::Active => "Active",
                    AttackPhase::Recovery => "Recovery",
                };
                format!("{:?} {} {:.2}s", active.kind, phase_str, active.phase_timer)
            } else {
                "ready".into()
            };
        }
    } else {
        if let Ok(mut text) = q_hud.p2().single_mut() { **text = "HP --/--".into(); }
        if let Ok(mut text) = q_hud.p3().single_mut() { **text = "STA --/--".into(); }
        if let Ok(mut text) = q_hud.p4().single_mut() { **text = "Phase: --".into(); }
    }
}
