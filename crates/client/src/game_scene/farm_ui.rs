//! Farming interaction, plot presentation, and its configuration panel.

use bevy::prelude::*;
use lk2_core::farming::{CropKind, FarmingError};

use super::inventory::InventoryUiState;
use super::keybindings::{GameAction, KeyBindings, UiSettings};
use super::offline::OfflineNature;
use super::state::{FARM_PLOT_POSITIONS, FarmVisualMaterials, crop_material_index};
use super::util::PLAYER_PHYSICS_CENTER_HEIGHT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FarmCommand {
    Plant,
    Harvest,
}

#[derive(Resource)]
pub struct FarmingUiState {
    pub open: bool,
    pub selected: CropKind,
    pending: Option<FarmCommand>,
    pub status: String,
}

impl Default for FarmingUiState {
    fn default() -> Self {
        Self {
            open: false,
            selected: CropKind::Wheat,
            pending: None,
            status: "靠近农田后按 F 种植，按 G 收获".into(),
        }
    }
}

#[derive(Component)]
pub struct FarmingUiRoot;

#[derive(Component)]
pub struct FarmingBodyText;

#[derive(Component)]
pub struct FarmingStatusText;

#[derive(Component)]
pub(crate) struct FarmCropButton(CropKind);

#[derive(Component)]
pub(crate) struct FarmActionButton(FarmCommand);

#[derive(Component)]
pub(crate) struct FarmCloseButton;

pub fn setup_farming_ui(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(24),
                bottom: px(24),
                width: px(390),
                max_height: percent(70),
                padding: UiRect::all(px(16)),
                flex_direction: FlexDirection::Column,
                row_gap: px(9),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.05, 0.075, 0.96)),
            BorderColor::all(Color::srgba(0.76, 0.58, 0.28, 0.9)),
            Visibility::Hidden,
            FarmingUiRoot,
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text::new("农田管理"),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(21.0),
                weight: FontWeight::BOLD,
                ..default()
            },
            TextColor(Color::srgb(0.96, 0.90, 0.72)),
        ));
        parent
            .spawn((Node {
                width: percent(100),
                column_gap: px(6),
                ..default()
            },))
            .with_children(|buttons| {
                spawn_crop_button(buttons, CropKind::Wheat, "小麦");
                spawn_crop_button(buttons, CropKind::Carrot, "胡萝卜");
                spawn_crop_button(buttons, CropKind::Potato, "土豆");
            });
        parent.spawn((
            Text::new(""),
            TextFont {
                font: FontSource::UiMonospace,
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::srgb(0.92, 0.96, 0.88)),
            FarmingBodyText,
        ));
        parent.spawn((
            Text::new(""),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::srgba(0.82, 0.86, 0.80, 0.96)),
            FarmingStatusText,
        ));
        parent
            .spawn((Node {
                width: percent(100),
                column_gap: px(6),
                ..default()
            },))
            .with_children(|buttons| {
                spawn_action_button(buttons, FarmCommand::Plant, "种植");
                spawn_action_button(buttons, FarmCommand::Harvest, "收获");
                buttons
                    .spawn((
                        Button,
                        Node {
                            flex_grow: 1.0,
                            min_height: px(30),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border_radius: BorderRadius::all(px(5)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.10, 0.19, 0.22, 1.0)),
                        FarmCloseButton,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("关闭"),
                            TextFont {
                                font: FontSource::UiSansSerif,
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
            });
    });
}

fn spawn_crop_button(parent: &mut ChildSpawnerCommands, kind: CropKind, label: &'static str) {
    parent
        .spawn((
            Button,
            Node {
                flex_grow: 1.0,
                min_height: px(30),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.19, 0.22, 1.0)),
            FarmCropButton(kind),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font: FontSource::UiSansSerif,
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_action_button(
    parent: &mut ChildSpawnerCommands,
    command: FarmCommand,
    label: &'static str,
) {
    parent
        .spawn((
            Button,
            Node {
                flex_grow: 1.0,
                min_height: px(30),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.19, 0.22, 1.0)),
            FarmActionButton(command),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font: FontSource::UiSansSerif,
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub fn toggle_farming_ui(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    inventory: Res<InventoryUiState>,
    mut state: ResMut<FarmingUiState>,
) {
    if bindings.menu_open || inventory.open {
        return;
    }
    if bindings.just_pressed(GameAction::OpenFarming, &keys, &mouse) {
        state.open = !state.open;
    }
}

pub(crate) fn handle_farming_buttons(
    mut state: ResMut<FarmingUiState>,
    buttons: Query<
        (
            &Interaction,
            Option<&FarmCropButton>,
            Option<&FarmActionButton>,
            Option<&FarmCloseButton>,
        ),
        Changed<Interaction>,
    >,
) {
    for (interaction, crop, action, close) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(crop) = crop {
            state.selected = crop.0;
            state.status = format!("已选择{}", crop_name(crop.0));
        } else if let Some(action) = action {
            state.pending = Some(action.0);
        } else if close.is_some() {
            state.open = false;
            state.pending = None;
        }
    }
}

pub fn handle_farming_actions(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    inventory: Res<InventoryUiState>,
    mut nature: ResMut<OfflineNature>,
    mut state: ResMut<FarmingUiState>,
    players: Query<&Transform, With<super::state::PlayerActor>>,
) {
    if bindings.menu_open || inventory.open {
        return;
    }
    let mut command = state.pending.take();
    if !state.open && bindings.just_pressed(GameAction::CycleCrop, &keys, &mouse) {
        let index = CropKind::ALL
            .iter()
            .position(|kind| *kind == state.selected)
            .unwrap_or(0);
        state.selected = CropKind::ALL[(index + 1) % CropKind::ALL.len()];
        state.status = format!("已选择{}", crop_name(state.selected));
    }
    if !state.open {
        if bindings.just_pressed(GameAction::PlantCrop, &keys, &mouse) {
            command = Some(FarmCommand::Plant);
        } else if bindings.just_pressed(GameAction::HarvestCrop, &keys, &mouse) {
            command = Some(FarmCommand::Harvest);
        }
    }
    let Some(command) = command else {
        return;
    };
    let Ok(player) = players.single() else {
        state.status = "找不到玩家位置".into();
        return;
    };
    let player_position = player.translation - Vec3::Y * PLAYER_PHYSICS_CENTER_HEIGHT;
    let plot_id = match command {
        FarmCommand::Plant => nearest_plot_id(player_position, &nature, false),
        FarmCommand::Harvest => nearest_plot_id(player_position, &nature, true),
    };
    let Some(plot_id) = plot_id else {
        state.status = "请靠近一块可用农田".into();
        return;
    };
    match command {
        FarmCommand::Plant => {
            let result = {
                let OfflineNature {
                    farming, resources, ..
                } = &mut *nature;
                farming.plant(plot_id, state.selected, resources)
            };
            match result {
                Ok(()) => {
                    state.status = format!(
                        "已在第 {} 块农田种下{}",
                        plot_id + 1,
                        crop_name(state.selected)
                    );
                }
                Err(error) => state.status = farming_error_text(error),
            }
        }
        FarmCommand::Harvest => {
            let result = {
                let OfflineNature {
                    farming, resources, ..
                } = &mut *nature;
                farming.harvest(plot_id, resources)
            };
            match result {
                Ok((resource, amount)) => {
                    state.status = format!(
                        "已收获第 {} 块农田：{} x{}",
                        plot_id + 1,
                        resource.label_zh(),
                        amount
                    );
                }
                Err(error) => state.status = farming_error_text(error),
            }
        }
    }
}

pub(crate) fn reconcile_farm_visuals(
    nature: Res<OfflineNature>,
    materials: Res<FarmVisualMaterials>,
    mut crops: Query<(
        &super::state::FarmCropVisual,
        &mut Transform,
        &mut Visibility,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    for (visual, mut transform, mut visibility, mut material) in &mut crops {
        let Some(plot) = nature
            .farming
            .plots
            .iter()
            .find(|plot| plot.id == visual.id)
        else {
            continue;
        };
        let Some(crop) = plot.crop else {
            *visibility = Visibility::Hidden;
            transform.scale = Vec3::ZERO;
            continue;
        };
        let growth = (crop.age_ticks as f32 / crop.kind.growth_ticks() as f32).clamp(0.0, 1.0);
        let scale = 0.35 + growth * 0.75;
        *visibility = Visibility::Visible;
        transform.translation =
            FARM_PLOT_POSITIONS[visual.id as usize] + Vec3::Y * (0.58 + scale * 0.35);
        transform.scale = Vec3::splat(scale);
        material.0 = materials.crop_materials[crop_material_index(crop.kind)].clone();
    }
}

pub(crate) fn update_farming_ui(
    nature: Res<OfflineNature>,
    state: Res<FarmingUiState>,
    ui_settings: Res<UiSettings>,
    mut root: Query<
        (&mut Visibility, &mut BackgroundColor),
        (With<FarmingUiRoot>, Without<Button>),
    >,
    mut body: Query<&mut Text, (With<FarmingBodyText>, Without<FarmingStatusText>)>,
    mut status: Query<&mut Text, (With<FarmingStatusText>, Without<FarmingBodyText>)>,
    mut crop_buttons: Query<(&FarmCropButton, &Interaction, &mut BackgroundColor), With<Button>>,
    mut action_buttons: Query<
        (&Interaction, &mut BackgroundColor),
        (With<Button>, Without<FarmCropButton>),
    >,
) {
    if let Ok((mut visibility, mut background)) = root.single_mut() {
        *visibility = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        background.0 = Color::srgba(0.035, 0.05, 0.075, ui_settings.panel_opacity);
    }
    if let Ok(mut text) = body.single_mut() {
        let mut value = format!(
            "当前作物：{}\n种子库存：{}\n\n",
            crop_name(state.selected),
            nature.resources.get(state.selected.seed())
        );
        for plot in &nature.farming.plots {
            match plot.crop {
                Some(crop) => value.push_str(&format!(
                    "农田 {}：{} {}/{} {}\n",
                    plot.id + 1,
                    crop_name(crop.kind),
                    crop.age_ticks,
                    crop.kind.growth_ticks(),
                    if crop.is_mature() {
                        "已成熟"
                    } else {
                        "生长中"
                    }
                )),
                None => value.push_str(&format!("农田 {}：空闲\n", plot.id + 1)),
            }
        }
        text.0 = value;
    }
    if let Ok(mut text) = status.single_mut() {
        text.0 = format!("{}   O 打开/关闭", state.status);
    }
    for (button, interaction, mut background) in &mut crop_buttons {
        background.0 = button_background(*interaction, button.0 == state.selected);
    }
    for (interaction, mut background) in &mut action_buttons {
        background.0 = button_background(*interaction, false);
    }
}

fn nearest_plot_id(player_position: Vec3, nature: &OfflineNature, occupied: bool) -> Option<u32> {
    FARM_PLOT_POSITIONS
        .into_iter()
        .enumerate()
        .filter_map(|(id, position)| {
            let plot = nature
                .farming
                .plots
                .iter()
                .find(|plot| plot.id == id as u32)?;
            if plot.crop.is_some() != occupied || position.distance(player_position) > 4.8 {
                return None;
            }
            Some((id as u32, position.distance_squared(player_position)))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(id, _)| id)
}

fn crop_name(kind: CropKind) -> &'static str {
    match kind {
        CropKind::Wheat => "小麦",
        CropKind::Carrot => "胡萝卜",
        CropKind::Potato => "土豆",
    }
}

fn farming_error_text(error: FarmingError) -> String {
    match error {
        FarmingError::MissingSeed(resource) => format!("种子不足：{}", resource.label_zh()),
        FarmingError::PlotOccupied(id) => format!("农田 {} 已经有作物", id + 1),
        FarmingError::NotMature(id) => format!("农田 {} 还没有成熟", id + 1),
        other => format!("农田操作失败：{other:?}"),
    }
}

fn button_background(interaction: Interaction, selected: bool) -> Color {
    if selected {
        Color::srgba(0.42, 0.34, 0.16, 1.0)
    } else {
        match interaction {
            Interaction::Pressed => Color::srgba(0.22, 0.46, 0.47, 1.0),
            Interaction::Hovered => Color::srgba(0.16, 0.34, 0.36, 1.0),
            Interaction::None => Color::srgba(0.10, 0.19, 0.22, 1.0),
        }
    }
}
