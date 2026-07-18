//! Compact always-visible HUD for the living forest scene.

use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use lk2_core::resource::ResourceKind;

use super::keybindings::{GameAction, KeyBindings};
use super::offline::{OfflineNature, OfflineQuestPhase};
use super::settlement::settlement_hud_text;
use super::state::{
    BuildingExplorationState, BuildingPrompt, DimensionId, DimensionTravelState, LivingSceneState,
    StarfallProgress, TerrainRebuildState,
};
use super::targeting::TargetingState;
use super::ui_drag::{UiDragHandle, UiDragPanel};

#[derive(Component)]
pub(crate) struct GameplayHudTick;

#[derive(Component)]
pub(crate) struct GameplayHudResourceValue(ResourceKind);

#[derive(Component)]
pub(crate) struct GameplayHudHints;

#[derive(Component)]
pub(crate) struct GameplayHudCombat;

#[derive(Component)]
pub(crate) struct GameplayHudCrosshair;

#[derive(Component)]
pub(crate) struct GameplayHudTarget;

#[derive(Component)]
pub(crate) struct GameplayHudSettlement;

#[derive(Component)]
pub(crate) struct GameplayHudObjective;

pub fn setup_gameplay_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            top: percent(50),
            width: px(24),
            height: px(24),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        UiTransform::from_translation(Val2::px(-12.0, -12.0)),
        Text::new("·"),
        TextFont {
            font: FontSource::UiSansSerif,
            font_size: FontSize::Px(18.0),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(Color::srgba(0.98, 0.90, 0.66, 0.86)),
        GameplayHudCrosshair,
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            top: percent(50),
            max_width: px(260),
            padding: UiRect::axes(px(8), px(4)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        UiTransform::from_translation(Val2::px(-130.0, 22.0)),
        BackgroundColor(Color::srgba(0.04, 0.07, 0.08, 0.88)),
        BorderColor::all(Color::srgba(0.95, 0.62, 0.16, 0.90)),
        Text::new(""),
        TextFont {
            font: FontSource::UiSansSerif,
            font_size: FontSize::Px(12.0),
            weight: FontWeight::SEMIBOLD,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.82, 0.38)),
        Visibility::Hidden,
        GameplayHudTarget,
    ));
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(14),
                right: px(14),
                // A little more horizontal room keeps the shortcut and
                // objective lines readable instead of collapsing into a
                // dense stack over the playfield.
                width: px(232),
                max_width: percent(44),
                max_height: percent(92),
                padding: UiRect::all(px(10)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                overflow: Overflow::clip(),
                ..default()
            },
            // Leather-and-bronze framing gives the HUD a guild-board feel.
            BackgroundColor(Color::srgba(0.075, 0.045, 0.028, 0.88)),
            BorderColor::all(Color::srgba(0.62, 0.42, 0.18, 0.82)),
            UiTransform::from_translation(Val2::px(0.0, 0.0)),
            UiDragPanel,
            ZIndex(0),
        ))
        .id();
    commands.entity(root).with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                Interaction::None,
                FocusPolicy::Block,
                UiDragHandle(root),
            ))
            .with_children(|header| {
                header.spawn((
                    Text::new("生命森林"),
                    TextFont {
                        font: FontSource::UiSansSerif,
                        font_size: FontSize::Px(18.0),
                        weight: FontWeight::BOLD,
                        ..default()
                    },
                    TextColor(Color::srgb(0.97, 0.86, 0.62)),
                ));
                header.spawn((
                    Text::new("离线世界"),
                    TextFont {
                        font: FontSource::UiSansSerif,
                        font_size: FontSize::Px(10.0),
                        weight: FontWeight::SEMIBOLD,
                        ..default()
                    },
                    TextColor(Color::srgb(0.78, 0.61, 0.30)),
                ));
            });
        parent.spawn((
            Text::new("资源总览"),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(11.0),
                weight: FontWeight::SEMIBOLD,
                ..default()
            },
            TextColor(Color::srgb(0.78, 0.61, 0.30)),
        ));
        spawn_resource_row(
            parent,
            (ResourceKind::Wood, "木材", Color::srgb(0.72, 0.45, 0.24)),
            (ResourceKind::Stone, "石头", Color::srgb(0.58, 0.66, 0.70)),
        );
        spawn_resource_row(
            parent,
            (
                ResourceKind::WheatSeeds,
                "小麦种子",
                Color::srgb(0.90, 0.73, 0.28),
            ),
            (
                ResourceKind::Carrot,
                "胡萝卜",
                Color::srgb(0.95, 0.42, 0.22),
            ),
        );
        spawn_resource_row(
            parent,
            (ResourceKind::Potato, "土豆", Color::srgb(0.74, 0.57, 0.32)),
            (ResourceKind::Food, "食物", Color::srgb(0.84, 0.35, 0.38)),
        );
        parent
            .spawn((
                Node {
                    width: percent(100),
                    padding: UiRect::all(px(9)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(5)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.075, 0.042, 0.84)),
                BorderColor::all(Color::srgba(0.38, 0.25, 0.12, 0.76)),
            ))
            .with_children(|footer| {
                footer.spawn((
                    Text::new("快捷操作"),
                    TextFont {
                        font: FontSource::UiSansSerif,
                        font_size: FontSize::Px(10.0),
                        weight: FontWeight::SEMIBOLD,
                        ..default()
                    },
                    TextColor(Color::srgb(0.72, 0.54, 0.27)),
                ));
                footer.spawn((
                    Text::new(""),
                    TextFont {
                        font: FontSource::UiSansSerif,
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgba(0.88, 0.80, 0.66, 0.96)),
                    GameplayHudHints,
                ));
                footer.spawn((
                    Text::new(""),
                    TextFont {
                        font: FontSource::UiSansSerif,
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.96, 0.70, 0.24)),
                    GameplayHudCombat,
                ));
            });
        parent.spawn((
            Text::new(""),
            TextFont {
                font: FontSource::UiMonospace,
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(Color::srgba(0.60, 0.48, 0.32, 0.92)),
            GameplayHudTick,
        ));
        parent.spawn((
            Text::new(""),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(12.0),
                weight: FontWeight::SEMIBOLD,
                ..default()
            },
            TextColor(Color::srgb(0.98, 0.84, 0.42)),
            GameplayHudObjective,
        ));
        parent.spawn((
            Text::new(""),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::srgb(0.96, 0.78, 0.38)),
            GameplayHudSettlement,
        ));
    });
}

pub fn update_gameplay_hud_combat(
    bindings: Res<KeyBindings>,
    state: Res<LivingSceneState>,
    building: Res<BuildingExplorationState>,
    terrain: Res<TerrainRebuildState>,
    mut hud_text: ParamSet<(
        Query<
            &mut Text,
            (With<GameplayHudCombat>, Without<GameplayHudCrosshair>),
        >,
        Query<
            &mut Text,
            (With<GameplayHudCrosshair>, Without<GameplayHudCombat>),
        >,
    )>,
) {
    let interact = bindings.binding(GameAction::Interact).label();
    let ride = bindings.binding(GameAction::RideCart).label();
    let skill = bindings.binding(GameAction::WeaponSkill).label();
    let building_hint = match building.prompt {
        Some(BuildingPrompt::Enter) => format!("   {interact} 进入房屋"),
        Some(BuildingPrompt::Exit) => format!("   {interact} 离开房屋"),
        None => String::new(),
    };
    let mining_feedback = terrain.last_mined.map_or_else(String::new, |target| {
        if terrain.feedback_remaining > 0.0 {
            format!("   已挖掘 ({}, {}, {})", target[0], target[1], target[2])
        } else {
            String::new()
        }
    });
    {
        let mut combat = hud_text.p0();
        let Ok(mut text) = combat.single_mut() else {
            return;
        };
        text.0 = format!(
            "{interact} {}   {skill} {}\n{ride} \u{9a91}\u{8f66}{building_hint}{mining_feedback}",
            "\u{62fe}\u{53d6}\u{6b66}\u{5668}", "\u{6b66}\u{5668}\u{6280}\u{80fd}"
        );
    }
    let mut crosshair = hud_text.p1();
    if let Ok(mut marker) = crosshair.single_mut() {
        marker.0 = if state.camera_shake > 0.0 { "✦" } else { "·" }.to_string();
    }
}

pub fn update_gameplay_hud_target(
    targeting: Res<TargetingState>,
    mut target_hud: Query<(&mut Text, &mut Visibility), With<GameplayHudTarget>>,
) {
    let Ok((mut text, mut visibility)) = target_hud.single_mut() else {
        return;
    };
    if let Some(selection) = targeting.selection {
        text.0 = format!(
            "{}  ·  {}",
            selection.inspectable.display_name, selection.inspectable.category
        );
        *visibility = Visibility::Visible;
    } else {
        text.0.clear();
        *visibility = Visibility::Hidden;
    }
}

fn spawn_resource_row(
    parent: &mut ChildSpawnerCommands,
    left: (ResourceKind, &'static str, Color),
    right: (ResourceKind, &'static str, Color),
) {
    parent
        .spawn((Node {
            width: percent(100),
            column_gap: px(6),
            ..default()
        },))
        .with_children(|row| {
            spawn_resource_cell(row, left.0, left.1, left.2);
            spawn_resource_cell(row, right.0, right.1, right.2);
        });
}

fn spawn_resource_cell(
    parent: &mut ChildSpawnerCommands,
    kind: ResourceKind,
    label: &'static str,
    accent: Color,
) {
    parent
        .spawn((
            Node {
                flex_grow: 1.0,
                min_height: px(40),
                padding: UiRect::axes(px(7), px(5)),
                column_gap: px(6),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.055, 0.085, 0.10, 0.90)),
            BorderColor::all(Color::srgba(0.16, 0.28, 0.30, 0.80)),
        ))
        .with_children(|cell| {
            cell.spawn((
                Node {
                    width: px(4),
                    height: px(24),
                    border_radius: BorderRadius::all(px(2)),
                    ..default()
                },
                BackgroundColor(accent),
            ));
            cell.spawn((Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                ..default()
            },))
                .with_children(|content| {
                    content.spawn((
                        Text::new(label),
                        TextFont {
                            font: FontSource::UiSansSerif,
                            font_size: FontSize::Px(9.0),
                            ..default()
                        },
                        TextColor(Color::srgba(0.66, 0.76, 0.75, 0.96)),
                    ));
                    content.spawn((
                        Text::new("0"),
                        TextFont {
                            font: FontSource::UiMonospace,
                            font_size: FontSize::Px(15.0),
                            weight: FontWeight::SEMIBOLD,
                            ..default()
                        },
                        TextColor(Color::srgb(0.94, 0.98, 0.91)),
                        GameplayHudResourceValue(kind),
                    ));
                });
        });
}

pub fn update_gameplay_hud(
    time: Res<Time>,
    nature: Res<OfflineNature>,
    quest: Res<OfflineQuestPhase>,
    dimension: Res<DimensionTravelState>,
    starfall: Res<StarfallProgress>,
    bindings: Res<KeyBindings>,
    mut terrain: ResMut<TerrainRebuildState>,
    // These HUD labels all mutate the same `Text` component type. Their
    // marker filters are mutually exclusive at runtime, but Bevy's static
    // query validation cannot prove that across five parameters, so keep the
    // disjoint mutable views in a ParamSet.
    mut hud_text: ParamSet<(
        Query<
            (&GameplayHudResourceValue, &mut Text),
            (
                With<GameplayHudResourceValue>,
                Without<GameplayHudTick>,
                Without<GameplayHudHints>,
                Without<GameplayHudSettlement>,
                Without<GameplayHudObjective>,
            ),
        >,
        Query<
            &mut Text,
            (
                With<GameplayHudTick>,
                Without<GameplayHudHints>,
                Without<GameplayHudSettlement>,
                Without<GameplayHudObjective>,
            ),
        >,
        Query<
            &mut Text,
            (
                With<GameplayHudHints>,
                Without<GameplayHudTick>,
                Without<GameplayHudSettlement>,
                Without<GameplayHudObjective>,
            ),
        >,
        Query<&mut Text, (With<GameplayHudSettlement>, Without<GameplayHudObjective>)>,
        Query<&mut Text, With<GameplayHudObjective>>,
    )>,
) {
    terrain.feedback_remaining = (terrain.feedback_remaining - time.delta_secs()).max(0.0);
    for (kind, mut text) in &mut hud_text.p0() {
        text.0 = nature.resources.get(kind.0).to_string();
    }
    if let Ok(mut text) = hud_text.p1().single_mut() {
        let realm = match dimension.current {
            DimensionId::Forest => "\u{68ee}\u{6797}",
            DimensionId::Starfall => "\u{661f}\u{843d}",
        };
        text.0 = format!(
            "\u{7ef4}\u{5ea6} {realm}  ·  \u{4e16}\u{754c}\u{8fdb}\u{5ea6}  {} tick",
            nature.tick
        );
    }
    if let Ok(mut text) = hud_text.p3().single_mut() {
        text.0 = settlement_hud_text(&nature, &bindings);
    }
    if let Ok(mut text) = hud_text.p4().single_mut() {
        text.0 = if dimension.current == DimensionId::Starfall {
            if starfall.reward_claimed {
                "\u{5b8c}\u{6210}\u{ff1a}\u{661f}\u{843d}\u{754c}\u{7684}\u{661f}\u{7802}\u{5df2}\u{6536}\u{96c6}\u{5b8c}\u{6bd5}".to_string()
            } else if starfall.collected >= starfall.total {
                if starfall.guardian_defeated {
                    "\u{76ee}\u{6807}\u{ff1a}\u{56de}\u{5230}\u{4e2d}\u{592e}\u{9886}\u{53d6}\u{661f}\u{843d}\u{9057}\u{7269}".to_string()
                } else {
                    "\u{76ee}\u{6807}\u{ff1a}\u{51fb}\u{8d25}\u{661f}\u{843d}\u{5b88}\u{536b}"
                        .to_string()
                }
            } else {
                format!(
                    "\u{76ee}\u{6807}\u{ff1a}\u{6536}\u{96c6}\u{661f}\u{7802} {}/{}\n\u{9760}\u{8fd1}\u{540e}\u{6309} {} \u{62fe}\u{53d6}",
                    starfall.collected,
                    starfall.total,
                    bindings.binding(GameAction::Interact).label(),
                )
            }
        } else {
            offline_quest_hud_text(&nature, *quest, &bindings)
        };
    }
    if let Ok(mut text) = hud_text.p2().single_mut() {
        let inventory = bindings.binding(GameAction::OpenInventory).label();
        let farming = bindings.binding(GameAction::OpenFarming).label();
        let settings = bindings.binding(GameAction::OpenBindings).label();
        let world = bindings.binding(GameAction::OpenWorldStatus).label();
        let god_view = bindings.binding(GameAction::ToggleGodView).label();
        let mine = bindings.binding(GameAction::Mine).label();
        let sprint = bindings.binding(GameAction::Sprint).label();
        let crouch = bindings.binding(GameAction::Crouch).label();
        text.0 = format!(
            "{sprint} 疾跑   {crouch} 下蹲   {mine} 采矿\n{inventory} 背包   {farming} 农田\n{settings} 设置   {world} 世界状态   {god_view} 上帝视角",
        );
    }
}

fn offline_quest_hud_text(
    nature: &OfflineNature,
    phase: OfflineQuestPhase,
    bindings: &KeyBindings,
) -> String {
    match phase {
        OfflineQuestPhase::Explore => format!(
            "目标：收集木材 {}/12、石头 {}/6",
            nature.resources.get(ResourceKind::Wood),
            nature.resources.get(ResourceKind::Stone),
        ),
        OfflineQuestPhase::CampReady => format!(
            "目标：按 {} 建造营地",
            bindings.binding(GameAction::BuildCamp).label(),
        ),
        OfflineQuestPhase::Establishing => {
            format!("目标：守住营地 {}/30 tick", nature.settlement.tick,)
        }
        OfflineQuestPhase::HuntGuardian => "目标：击败森林守护者".to_string(),
        OfflineQuestPhase::Victory => "完成：营地已建立，森林守护者已击败！".to_string(),
    }
}
