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
use lk2_core::eco_cycle::EcoCycle;
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

#[derive(Component)]
pub struct TutorialOverlay {
    pub remaining_secs: f32,
}

#[derive(Component)]
pub struct HudObjectiveText;

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
    commands.insert_resource(UiFonts { cn: asset_server.load("fonts/NotoSansCJKsc-Regular.otf") });
}

pub fn setup_hud(mut commands: Commands, fonts: Res<UiFonts>) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            padding: UiRect::all(px(10)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.62)),
        children![(
            Text::new("WANGUO ORIGINS loading..."),
            TextFont { font: fonts.cn.clone(), font_size: 10.0, ..default() },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.96)),
            TextShadow { offset: Vec2::new(2.0, 2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
            HudText,
        )],
    ));

    let cross_size = 16.0;
    let cross_thickness = 2.0;
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
        TextFont { font: fonts.cn.clone(), font_size: 10.0, ..default() },
        TextColor(Color::srgb(0.95, 0.95, 0.7)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
        Node { position_type: PositionType::Absolute, bottom: px(12), left: px(12), ..default() },
        HudFooter,
    ));

    commands.spawn((
        Text::new(
            "WASD move | Mouse Left attack | E pick up\n\
             ECO: 5 rabbits eat 10 berry bushes, emit CO2, berries regrow fruit",
        ),
        TextFont { font: fonts.cn.clone(), font_size: 10.0, ..default() },
        TextColor(Color::srgba(0.95, 0.95, 0.95, 1.0)),
        TextLayout::new_with_justify(Justify::Center),
        TextShadow { offset: Vec2::new(2.0, 2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node {
            position_type: PositionType::Absolute,
            top: px(140),
            left: px(60),
            right: px(60),
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
            TextFont { font: fonts.cn.clone(), font_size: 12.0, ..default() },
            TextColor(Color::srgba(1.0, 0.9, 0.4, 0.7)),
            TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
            AnimalIndicatorText,
        )],
    ));
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
            TextFont { font: fonts.cn.clone(), font_size: 10.0, ..default() },
            TextColor(Color::srgba(1.0, 0.6, 0.4, 0.6)),
            TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
            NestIndicatorText,
        )],
    ));

    commands.spawn((
        Text::new("HP 100/100"),
        TextFont { font: fonts.cn.clone(), font_size: 16.0, ..default() },
        TextColor(Color::srgb(1.0, 0.4, 0.4)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(12), right: px(12), ..default() },
        HudHpText,
        HealthHudMarker,
    ));
    commands.spawn((
        Text::new("STA 100/100"),
        TextFont { font: fonts.cn.clone(), font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.4, 0.8, 1.0)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(38), right: px(12), ..default() },
        HudStaText,
    ));
    commands.spawn((
        Text::new("Phase: --"),
        TextFont { font: fonts.cn.clone(), font_size: 10.0, ..default() },
        TextColor(Color::srgb(0.9, 0.9, 0.5)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(60), right: px(12), ..default() },
        HudPhaseText,
    ));
    commands.spawn((
        Text::new("I/O=Light/Heavy  L=Thrust\nU=Block  Y=Parry"),
        TextFont { font: fonts.cn.clone(), font_size: 10.0, ..default() },
        TextColor(Color::srgba(0.85, 0.85, 0.85, 0.85)),
        TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, bottom: px(56), right: px(12), ..default() },
    ));
    commands.spawn((
        Text::new("Objective: -"),
        TextFont { font: fonts.cn.clone(), font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.85, 0.95, 1.0)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(106), left: px(12), ..default() },
        HudObjectiveText,
    ));
    commands.spawn((
        Text::new(""),
        TextFont { font: fonts.cn.clone(), font_size: 28.0, ..default() },
        TextColor(Color::srgba(1.0, 0.95, 0.4, 0.95)),
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
    _clock: Res<SimClock>,
    _player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    eco: Res<EcoCycle>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    objectives: Res<Objectives>,
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
    let wood = hud_state.map(|s| s.pool_wood).unwrap_or(pool.get(ResourceKind::Wood));
    let food = hud_state.map(|s| s.pool_food).unwrap_or(pool.get(ResourceKind::Food));
    let apple = hud_state.map(|s| s.pool_apple).unwrap_or(pool.get(ResourceKind::Apple));
    let soul = hud_state.map(|s| s.pool_soul).unwrap_or(pool.get(ResourceKind::Soul));
    let flags = hud_state.map(|s| s.flag_count).unwrap_or(nations.flag_count);
    let monster_count = hud_state.map(|s| s.monster_count).unwrap_or(monsters.current_individuals);

    if let Ok(mut text) = q_hud.p0().single_mut() {
        let phase_remaining = match_clock.phase_remaining_secs();
        let m = (phase_remaining / 60.0).floor() as i32;
        let s = (phase_remaining - m as f32 * 60.0).floor() as i32;
        let phase_line = format!(
            "Phase: {}  T-{:02}:{:02}",
            match_clock.phase.label_zh(),
            m,
            s
        );
        **text = format_main_hud(
            run_mode.label(),
            fps,
            &phase_line,
            wood,
            food,
            apple,
            soul,
            flags,
            8,
            monster_count,
            eco.rabbit_count(),
            eco.berry_count(),
            eco.total_fruit(),
            eco.co2,
            eco.fruit_eaten,
            eco.fruit_grown,
        );
    }

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
            ObjectiveProgress::Flag(done) => done.to_string(),
            ObjectiveProgress::Pop(n) => n.to_string(),
            ObjectiveProgress::CountU(n) => n.to_string(),
            ObjectiveProgress::AtPosition { reached } => reached.to_string(),
            ObjectiveProgress::Empty => "-".to_string(),
        };
        (
            format!("Quest: {} {}", obj.kind.short_label(), progress),
            String::new(),
        )
    } else {
        ("All quests complete".to_string(), String::new())
    };
    if let Ok(mut text) = q_hud.p1().single_mut() {
        **text = format!("{goal_text}\n{status}");
    }

    if let Ok((hp, sta, att, parry, stun, down)) = q_player_combat.single() {
        if let Ok(mut text) = q_hud.p2().single_mut() {
            **text = format!("HP {}/{}", hp.current as i32, hp.max as i32);
        }
        if let Ok(mut text) = q_hud.p3().single_mut() {
            **text = format!(
                "STA {}/{} regen {:.0}/s",
                sta.current as i32, sta.max as i32, sta.regen_per_sec
            );
        }
        if let Ok(mut text) = q_hud.p4().single_mut() {
            **text = if down.downed {
                format!("DOWNED {:.1}s", down.timer)
            } else if stun.stunned {
                format!("STUNNED {:.2}s", stun.stun_timer)
            } else if parry.parry_active {
                format!("PARRY {:.2}s", parry.parry_timer)
            } else if let Some(active) = att.current {
                let phase = match active.phase {
                    AttackPhase::Windup => "Windup",
                    AttackPhase::Active => "Active",
                    AttackPhase::Recovery => "Recovery",
                };
                format!("{:?} {} {:.2}s", active.kind, phase, active.phase_timer)
            } else {
                "ready".to_string()
            };
        }
    }

    if let Ok(mut text) = q_hud.p6().single_mut() {
        **text = format!(
            "ECO LOOP: rabbits {}/5 -> eat fruit -> CO2 {:.1} -> berries grow fruit {}",
            eco.rabbit_count(),
            eco.co2,
            eco.total_fruit()
        );
    }

    let now_secs = time.elapsed_secs();
    for ev in completed_events.read() {
        if let Ok((mut text, mut flash)) = q_hud.p5().single_mut() {
            flash.shown_at_secs = now_secs;
            flash.text = format!("Done: {}", ev.kind.short_label());
            **text = flash.text.clone();
        }
    }
    if let Ok((mut text, mut flash)) = q_hud.p5().single_mut() {
        if now_secs - flash.shown_at_secs > 2.5 && !flash.text.is_empty() {
            **text = String::new();
            flash.text.clear();
        }
    }
}

pub fn update_tutorial_overlay(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut TutorialOverlay, &mut TextColor)>,
) {
    for (entity, mut overlay, mut color) in q.iter_mut() {
        overlay.remaining_secs -= time.delta_secs();
        if overlay.remaining_secs <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = if overlay.remaining_secs > 1.0 {
            1.0
        } else {
            overlay.remaining_secs.max(0.0)
        };
        color.0 = color.0.with_alpha(alpha);
    }
}

pub fn format_main_hud(
    run_mode_label: &str,
    fps: i32,
    phase_line: &str,
    wood: i64,
    food: i64,
    apple: i64,
    soul: i64,
    flags: u32,
    flag_cap: u32,
    monsters: u32,
    rabbits: usize,
    berry_bushes: usize,
    fruit: u32,
    co2: f32,
    fruit_eaten: u64,
    fruit_grown: u64,
) -> String {
    format!(
        "> WANGUO ORIGINS v0.4 | {run_mode_label} | {fps} fps | {phase_line}\n\
         > RES wood {wood} food {food} apple {apple} soul {soul}\n\
         > WORLD flags {flags}/{flag_cap} monsters {monsters}\n\
         > ECO rabbits {rabbits}/5 berries {berry_bushes}/10 fruit {fruit} CO2 {co2:.1} eat/grow {fruit_eaten}/{fruit_grown}",
    )
}

#[cfg(test)]
mod tests {
    use super::format_main_hud;

    #[test]
    fn format_main_hud_includes_eco_cycle() {
        let s = format_main_hud(
            "OFFLINE", 60, "Phase: x", 1, 2, 3, 4, 1, 8, 60, 5, 10, 7, 0.8, 11, 12,
        );
        assert_eq!(s.matches('\n').count(), 3);
        assert!(s.contains("rabbits 5/5"));
        assert!(s.contains("berries 10/10"));
        assert!(s.contains("CO2 0.8"));
        assert!(s.contains("eat/grow 11/12"));
    }
}

