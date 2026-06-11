use bevy::prelude::*;

use crate::pvp_systems::HealthHudMarker;
use crate::render::AnimalIndicatorText;
use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::diagnostics::SnapshotRole;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::protocol::components::GameplayHudState;
use lk2_core::resource::{GlobalResourcePool, ResourceKind};

#[derive(Component)]
pub struct HudText;

#[derive(Component)]
pub struct HudFooter;

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
        TextFont { font: fonts.cn.clone(), font_size: 22.0, ..default() },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
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
            Text::new("[scanning...]"),
            TextFont { font: fonts.cn.clone(), font_size: 24.0, ..default() },
            TextColor(Color::srgb(1.0, 0.9, 0.4)),
            TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
            AnimalIndicatorText,
        )],
    ));

    commands.spawn((
        Text::new("HP 20 / 20"),
        TextFont { font: fonts.cn.clone(), font_size: 20.0, ..default() },
        TextColor(Color::srgb(1.0, 0.4, 0.4)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(12), right: px(12), ..default() },
        HealthHudMarker,
    ));
}

pub fn update_hud(
    mut q_top: Query<&mut Text, With<HudText>>,
    mut q_bot: Query<&mut Text, (With<HudFooter>, Without<HudText>)>,
    clock: Res<SimClock>,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    obs: Res<TickObserver>,
    time: Res<Time>,
    run_mode: Res<ClientRunMode>,
    hud_state_q: Query<&GameplayHudState>,
) {
    let fps = (1.0 / time.delta_secs().max(0.001)).round() as i32;
    let hud_state = hud_state_q.iter().next();
    let tick_value = hud_state.map(|s| s.tick).unwrap_or(clock.tick);
    let block_pos = hud_state.map(|s| s.player_block_pos).unwrap_or(player.block_pos);
    let wood = hud_state.map(|s| s.pool_wood).unwrap_or(pool.get(ResourceKind::Wood));
    let food = hud_state.map(|s| s.pool_food).unwrap_or(pool.get(ResourceKind::Food));
    let apple = hud_state.map(|s| s.pool_apple).unwrap_or(pool.get(ResourceKind::Apple));
    let soul = hud_state.map(|s| s.pool_soul).unwrap_or(pool.get(ResourceKind::Soul));
    let flags = hud_state.map(|s| s.flag_count).unwrap_or(nations.flag_count);
    let monster_count =
        hud_state.map(|s| s.monster_count).unwrap_or(monsters.current_individuals);
    let anomalies = hud_state
        .map(|s| s.observer_anomalies as usize)
        .unwrap_or(obs.anomalies.len());
    let invariants = hud_state
        .map(|s| s.observer_invariant_violations)
        .unwrap_or(0);

    if let Ok(mut text) = q_top.single_mut() {
        **text = format!(
            "WANGUO ORIGINS v0.4  [{fps} fps]  {}\n\
             tick {} ({:.1}s)\n\
             player @ {:?}\n\
             Wood={}  Food={}  Apple={}  Soul={}\n\
             flags={}/8  monsters={}\n\
             anomalies={}  invariants={}",
            run_mode.label(),
            tick_value,
            time.elapsed_secs(),
            block_pos,
            wood,
            food,
            apple,
            soul,
            flags,
            monster_count,
            anomalies,
            invariants,
        );
    }

    let goal = 10;
    let progress_bar = {
        let pct = (wood as f32 / goal as f32).clamp(0.0, 1.0);
        let filled = (pct * 16.0) as usize;
        format!("{}{}", "#".repeat(filled), "-".repeat(16 - filled))
    };
    let status = if let Some(state) = hud_state {
        state.status_line.as_str()
    } else if wood >= goal {
        "*** WIN! 10 wood collected. Try Found Nation (F) ***"
    } else {
        ""
    };
    if let Ok(mut text) = q_bot.single_mut() {
        **text = format!(
            "[WASD] move  [Space] jump  [Shift] sneak  [G] gather  [P] place  [H] craft  [F] found  [J/K] hit  [Esc] quit\n\
             Goal: gather 10 wood    {wood}/{goal}  {progress_bar}\n\
             {status}",
        );
    }
}
