//! Compact always-visible HUD for the living forest scene.

use bevy::prelude::*;
use lk2_core::resource::ResourceKind;

use super::keybindings::{GameAction, KeyBindings};
use super::offline::OfflineNature;

#[derive(Component)]
pub(crate) struct GameplayHudTick;

#[derive(Component)]
pub(crate) struct GameplayHudResourceValue(ResourceKind);

#[derive(Component)]
pub(crate) struct GameplayHudHints;

pub fn setup_gameplay_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(20),
                right: px(20),
                width: px(350),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.04, 0.06, 0.94)),
            BorderColor::all(Color::srgba(0.34, 0.72, 0.70, 0.88)),
        ))
        .with_children(|parent| {
            parent
                .spawn((Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },))
                .with_children(|header| {
                    header.spawn((
                        Text::new("生命森林"),
                        TextFont {
                            font: FontSource::UiSansSerif,
                            font_size: FontSize::Px(20.0),
                            weight: FontWeight::BOLD,
                            ..default()
                        },
                        TextColor(Color::srgb(0.90, 0.97, 0.91)),
                    ));
                    header.spawn((
                        Text::new("离线世界"),
                        TextFont {
                            font: FontSource::UiSansSerif,
                            font_size: FontSize::Px(11.0),
                            weight: FontWeight::SEMIBOLD,
                            ..default()
                        },
                        TextColor(Color::srgb(0.46, 0.82, 0.70)),
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
                TextColor(Color::srgb(0.50, 0.82, 0.78)),
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
                        padding: UiRect::all(px(8)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(5)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.06, 0.10, 0.12, 0.92)),
                    BorderColor::all(Color::srgba(0.20, 0.38, 0.39, 0.72)),
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
                        TextColor(Color::srgb(0.47, 0.70, 0.70)),
                    ));
                    footer.spawn((
                        Text::new(""),
                        TextFont {
                            font: FontSource::UiSansSerif,
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgba(0.78, 0.86, 0.84, 0.96)),
                        GameplayHudHints,
                    ));
                });
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: FontSource::UiMonospace,
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::srgba(0.50, 0.62, 0.64, 0.9)),
                GameplayHudTick,
            ));
        });
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
                min_height: px(46),
                padding: UiRect::axes(px(8), px(6)),
                column_gap: px(8),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.055, 0.085, 0.10, 0.96)),
            BorderColor::all(Color::srgba(0.16, 0.28, 0.30, 0.86)),
        ))
        .with_children(|cell| {
            cell.spawn((
                Node {
                    width: px(4),
                    height: px(28),
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
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(Color::srgba(0.66, 0.76, 0.75, 0.96)),
                    ));
                    content.spawn((
                        Text::new("0"),
                        TextFont {
                            font: FontSource::UiMonospace,
                            font_size: FontSize::Px(16.0),
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
    nature: Res<OfflineNature>,
    bindings: Res<KeyBindings>,
    mut values: Query<
        (&GameplayHudResourceValue, &mut Text),
        (
            With<GameplayHudResourceValue>,
            Without<GameplayHudTick>,
            Without<GameplayHudHints>,
        ),
    >,
    mut tick: Query<&mut Text, (With<GameplayHudTick>, Without<GameplayHudHints>)>,
    mut hints: Query<&mut Text, (With<GameplayHudHints>, Without<GameplayHudTick>)>,
) {
    for (kind, mut text) in &mut values {
        text.0 = nature.resources.get(kind.0).to_string();
    }
    if let Ok(mut text) = tick.single_mut() {
        text.0 = format!("世界进度  ·  第 {} tick", nature.tick);
    }
    if let Ok(mut text) = hints.single_mut() {
        let inventory = bindings.binding(GameAction::OpenInventory).label();
        let farming = bindings.binding(GameAction::OpenFarming).label();
        let settings = bindings.binding(GameAction::OpenBindings).label();
        let world = bindings.binding(GameAction::OpenWorldStatus).label();
        text.0 = format!("{inventory} 背包   {farming} 农田\n{settings} 设置   {world} 世界状态",);
    }
}
