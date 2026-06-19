use bevy::prelude::*;

use crate::pvp_systems::HealthHudMarker;
use crate::render::{AnimalIndicatorText, NestIndicatorText, Player};
use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::combat::{
    AttackPhase, AttackState as CombatAttackState, Downed as CombatDowned, Health as CombatHealth,
    ParryWindow as CombatParryWindow, Stamina as CombatStamina, StunState as CombatStunState,
};
use lk2_core::diagnostics::SnapshotRole;
use lk2_core::match_state::MatchClock;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::objectives::{ObjectiveKind, ObjectiveProgress, Objectives};
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

/// Tutorial overlay (顶部中央), 5s 后淡出
#[derive(Component)]
pub struct TutorialOverlay {
    pub remaining_secs: f32,
}

/// 当前 quest chain 的激活任务 + 进度（左侧面板，紧贴左上角 HUD 下方）
#[derive(Component)]
pub struct HudObjectiveText;

/// 新完成任务的 flash 提示（屏幕中央，0.8 秒淡出）
#[derive(Component)]
pub struct HudObjectiveFlashText {
    pub shown_at_secs: f32,
    pub text: String,
}

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

    // ---- Tutorial Overlay (5s 后淡出, 顶部居中) ----
    // 玩家第一次进游戏立刻看见控制 + 目标, 5s 后淡出 (alpha 1→0 over 1s)
    commands.spawn((
        Text::new(
            "WASD 移动  ·  鼠标左键 挖/攻击  ·  E 拾取\n\
             F 建国家  ·  鼠标右键 防御  ·  1/2/3 换工具\n\
             目标: 砍 10 木 → 建国家 → 收 30 食物 → 升人口 → 杀 5 怪 → 到山顶",
        ),
        TextFont { font: fonts.cn.clone(), font_size: 18.0, ..default() },
        TextColor(Color::srgba(0.95, 0.95, 0.95, 1.0)),
        TextLayout::new_with_justify(Justify::Center), // ← 关键: TextLayout 让 text 居中 (bevy 0.18 Justify)
        TextShadow { offset: Vec2::new(2.0, 2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node {
            position_type: PositionType::Absolute,
            top: px(140),
            left: px(60),  // 让出左上 HUD 区域
            right: px(60), // 文字 100% width 居中显示
            ..default()
        },
        TutorialOverlay { remaining_secs: 5.0 },
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

    // 当前 Objective 文本（左侧 HUD 顶部下方 76px，紧贴 phase 倒数指示器）
    commands.spawn((
        Text::new("Objective: -"),
        TextFont { font: fonts.cn.clone(), font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.85, 0.95, 1.0)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(86), left: px(12), ..default() },
        HudObjectiveText,
    ));

    // Objective 完成 flash（屏幕中央偏上，2.5s 后自动清空文本）
    commands.spawn((
        Text::new(""),
        TextFont { font: fonts.cn.clone(), font_size: 28.0, ..default() },
        TextColor(Color::srgba(1.0, 0.95, 0.4, 0.95)), // 高 alpha,事件触发时显文本,2.5s 后清空
        TextShadow { offset: Vec2::new(2.0, 2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(35.0),
            left: Val::Percent(0.0),
            right: Val::Percent(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        HudObjectiveFlashText { shown_at_secs: -100.0, text: String::new() },
    ));
}

pub fn update_hud(
    mut q_hud: ParamSet<(
        Query<&mut Text, With<HudText>>,
        Query<&mut Text, (With<HudFooter>, Without<HudText>)>,
        Query<&mut Text, With<HudHpText>>,
        Query<&mut Text, With<HudStaText>>,
        Query<&mut Text, With<HudPhaseText>>,
        Query<(&mut Text, &mut HudObjectiveFlashText), Without<HudText>>,
        Query<&mut Text, (With<HudObjectiveText>, Without<HudObjectiveFlashText>)>,
    )>,
    clock: Res<SimClock>,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    objectives: Res<Objectives>, // T6 quest chain: 用 current objective 替代写死 "Goal: 10 wood"
    mut completed_events: MessageReader<lk2_core::objectives::ObjectiveCompleted>,
    _obs: Res<TickObserver>,
    time: Res<Time>,
    run_mode: Res<ClientRunMode>,
    hud_state_q: Query<&GameplayHudState>,
    match_clock: Res<MatchClock>,
    q_player_combat: Query<
        (
            &CombatHealth,
            &CombatStamina,
            &CombatAttackState,
            &CombatParryWindow,
            &CombatStunState,
            &CombatDowned,
        ),
        With<Player>,
    >,
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
    // ---- T6 任务链：用 current objective 替代写死 "Goal: 10 wood" ----
    // 玩家立刻能看见要做什么、进度多少 → "啥也玩不了" → "知道要干啥"
    let (goal_text, status) = if let Some(obj) = objectives.current() {
        let progress = match &obj.progress {
            ObjectiveProgress::Count(n) => format!(
                "{}/{}",
                n,
                match &obj.kind {
                    ObjectiveKind::GatherResource { count, .. } => *count,
                    _ => 0,
                }
            ),
            ObjectiveProgress::Flag(b) => {
                if *b {
                    "✓".to_string()
                } else {
                    "○".to_string()
                }
            }
            ObjectiveProgress::Pop(n) => format!(
                "{}/{}",
                n,
                match &obj.kind {
                    ObjectiveKind::UpgradePop { target } => *target as i64,
                    _ => 0,
                }
            ),
            ObjectiveProgress::CountU(n) => format!(
                "{}/{}",
                n,
                match &obj.kind {
                    ObjectiveKind::KillMonsters { count } => *count as i64,
                    _ => 0,
                }
            ),
            ObjectiveProgress::AtPosition { reached } => {
                if *reached {
                    "✓".to_string()
                } else {
                    "○".to_string()
                }
            }
            ObjectiveProgress::Empty => "?".to_string(),
        };
        let label = obj.kind.short_label();
        if obj.done {
            (
                format!("✓ {} [完成]", label),
                "press F to advance".to_string(),
            )
        } else {
            (
                format!("Quest: {}  {}", label, progress),
                match &obj.kind {
                    ObjectiveKind::GatherResource { kind: ResourceKind::Wood, .. }
                        if wood >= 10 =>
                    {
                        "press F to found nation".to_string()
                    }
                    ObjectiveKind::FoundNation => {
                        if player.nation_id.is_some() {
                            "国已创".to_string()
                        } else {
                            "press F to found".to_string()
                        }
                    }
                    _ => "".to_string(),
                },
            )
        }
    } else {
        (
            format!("Goal: 10 wood   {wood}/{goal}"),
            "press F to found nation".to_string(),
        )
    };
    if let Ok(mut text) = q_hud.p1().single_mut() {
        **text = format!(
            "{goal_text}\n\
             {status}",
        );
    }

    // ---- V2 战斗 HUD: HP / STA / Phase 实时显示 ----
    if let Ok((hp, sta, att, parry, stun, down)) = q_player_combat.single() {
        if let Ok(mut text) = q_hud.p2().single_mut() {
            let filled = (hp.ratio() * 10.0).clamp(0.0, 10.0) as i32;
            let bar = format!(
                "{}{}",
                "\u{2588}".repeat(filled as usize),
                "\u{2591}".repeat((10 - filled) as usize)
            );
            **text = format!("HP {}/{}  {}", hp.current as i32, hp.max as i32, bar);
        }
        if let Ok(mut text) = q_hud.p3().single_mut() {
            let filled = (sta.ratio() * 10.0).clamp(0.0, 10.0) as i32;
            let bar = format!(
                "{}{}",
                "\u{2588}".repeat(filled as usize),
                "\u{2591}".repeat((10 - filled) as usize)
            );
            **text = format!(
                "STA {}/{}  {}  regen {:.0}/s",
                sta.current as i32, sta.max as i32, bar, sta.regen_per_sec
            );
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
        if let Ok(mut text) = q_hud.p2().single_mut() {
            **text = "HP --/--".into();
        }
        if let Ok(mut text) = q_hud.p3().single_mut() {
            **text = "STA --/--".into();
        }
        if let Ok(mut text) = q_hud.p4().single_mut() {
            **text = "Phase: --".into();
        }
    }

    // ---- T6 左侧面板：当前 Objective（短标签 + 进度条）----
    if let Ok(mut text) = q_hud.p6().single_mut() {
        if let Some(obj) = objectives.current() {
            let progress = obj.kind.progress_str(&obj.progress);
            **text = format!("📜 {}  {}", obj.kind.short_label(), progress);
        } else {
            // 全部完成
            let done = objectives.all.iter().filter(|o| o.done).count();
            **text = format!("🏆 全部完成 ({}/{})", done, objectives.all.len());
        }
    }

    // ---- T6 完成 flash：监听 ObjectiveCompleted → 屏幕中央显示 2.5s,后清空 ----
    let now_secs = time.elapsed_secs();
    for ev in completed_events.read() {
        if let Ok((mut text, mut flash)) = q_hud.p5().single_mut() {
            flash.shown_at_secs = now_secs;
            flash.text = format!("✓ {}", ev.kind.short_label());
            **text = flash.text.clone();
        }
    }
    if let Ok((mut text, mut flash)) = q_hud.p5().single_mut() {
        let since = now_secs - flash.shown_at_secs;
        if since > 2.5 && !flash.text.is_empty() {
            // 超时清空 → text 空 = TextColor 高 alpha 也看不见
            **text = String::new();
            flash.text.clear();
        }
    }
}

/// Tutorial overlay 倒计时 + 淡出系统
/// 5s 内保持 alpha 1.0, 然后 1s 内淡出到 0, 然后 despawn
pub fn update_tutorial_overlay(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut TutorialOverlay, &mut TextColor)>,
) {
    for (entity, mut overlay, mut color) in q.iter_mut() {
        overlay.remaining_secs -= time.delta_secs();
        if overlay.remaining_secs <= 0.0 {
            // 完全淡出后 despawn
            commands.entity(entity).despawn();
            continue;
        }
        // 5s 前 alpha=1.0, 1s 淡出
        let alpha = if overlay.remaining_secs > 1.0 {
            1.0
        } else {
            overlay.remaining_secs.max(0.0)
        };
        color.0 = color.0.with_alpha(alpha);
    }
}
