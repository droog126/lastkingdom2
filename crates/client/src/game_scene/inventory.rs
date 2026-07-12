//! Inventory and crafting overview for the offline playable scene.

use bevy::prelude::*;
use lk2_core::content::{ContentRegistry, resource_content_id};
use lk2_core::resource::ResourceKind;

use super::keybindings::{GameAction, KeyBindings};
use super::offline::OfflineNature;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InventoryTab {
    Inventory,
    Crafting,
}

#[derive(Resource)]
pub struct InventoryUiState {
    pub open: bool,
    pub tab: InventoryTab,
}

impl Default for InventoryUiState {
    fn default() -> Self {
        Self {
            open: false,
            tab: InventoryTab::Inventory,
        }
    }
}

#[derive(Component)]
pub struct InventoryUiRoot;

#[derive(Component)]
pub(crate) struct InventoryTabButton(pub(crate) InventoryTab);

#[derive(Component)]
pub(crate) struct InventoryBodyText;

#[derive(Component)]
pub(crate) struct InventoryStatusText;

#[derive(Component)]
pub(crate) struct CloseInventoryButton;

pub fn setup_inventory_ui(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(24),
                top: px(24),
                width: px(500),
                max_height: percent(88),
                padding: UiRect::all(px(18)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.05, 0.075, 0.96)),
            BorderColor::all(Color::srgba(0.34, 0.72, 0.76, 0.8)),
            Visibility::Hidden,
            InventoryUiRoot,
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text::new("背包与制作"),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(22.0),
                weight: FontWeight::BOLD,
                ..default()
            },
            TextColor(Color::srgb(0.86, 0.96, 0.95)),
        ));
        parent
            .spawn((Node {
                width: percent(100),
                column_gap: px(6),
                ..default()
            },))
            .with_children(|tabs| {
                spawn_tab(tabs, InventoryTab::Inventory, "背包");
                spawn_tab(tabs, InventoryTab::Crafting, "制作");
            });
        parent
            .spawn((Node {
                max_height: px(560),
                overflow: Overflow::scroll_y(),
                ..default()
            },))
            .with_children(|body| {
                body.spawn((
                    Text::new(""),
                    TextFont {
                        font: FontSource::UiMonospace,
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.90, 0.98, 0.96)),
                    InventoryBodyText,
                ));
            });
        parent.spawn((
            Text::new("按 I 打开或关闭此面板"),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::srgba(0.70, 0.78, 0.80, 0.95)),
            InventoryStatusText,
        ));
        parent
            .spawn((
                Button,
                Node {
                    min_width: px(80),
                    min_height: px(30),
                    padding: UiRect::axes(px(10), px(5)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px(5)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.16, 0.26, 0.28, 1.0)),
                CloseInventoryButton,
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
}

fn spawn_tab(parent: &mut ChildSpawnerCommands, tab: InventoryTab, label: &'static str) {
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
            InventoryTabButton(tab),
        ))
        .with_children(|tab_button| {
            tab_button.spawn((
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

pub fn toggle_inventory_ui(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    mut state: ResMut<InventoryUiState>,
) {
    if bindings.menu_open {
        return;
    }
    if bindings.just_pressed(GameAction::OpenInventory, &keys, &mouse) {
        state.open = !state.open;
    }
}

pub fn handle_inventory_buttons(
    mut state: ResMut<InventoryUiState>,
    buttons: Query<
        (
            &Interaction,
            Option<&InventoryTabButton>,
            Option<&CloseInventoryButton>,
        ),
        Changed<Interaction>,
    >,
) {
    for (interaction, tab, close) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if close.is_some() {
            state.open = false;
        } else if let Some(tab) = tab {
            state.tab = tab.0;
        }
    }
}

pub fn update_inventory_ui(
    nature: Res<OfflineNature>,
    catalog: Res<ContentRegistry>,
    state: Res<InventoryUiState>,
    mut root: Query<&mut Visibility, With<InventoryUiRoot>>,
    mut body: Query<&mut Text, (With<InventoryBodyText>, Without<InventoryStatusText>)>,
    mut status: Query<&mut Text, (With<InventoryStatusText>, Without<InventoryBodyText>)>,
    mut tabs: Query<(&InventoryTabButton, &mut BackgroundColor)>,
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor),
        (With<Button>, Without<InventoryTabButton>),
    >,
) {
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok(mut text) = body.single_mut() {
        text.0 = match state.tab {
            InventoryTab::Inventory => inventory_text(&nature, &catalog),
            InventoryTab::Crafting => crafting_text(&nature, &catalog),
        };
    }
    if let Ok(mut text) = status.single_mut() {
        text.0 = match state.tab {
            InventoryTab::Inventory => "已收集的资源".into(),
            InventoryTab::Crafting => format!(
                "已登记 {} 个配方；是否可制作取决于当前资源",
                catalog.recipes().len()
            ),
        };
    }
    for (tab, mut background) in &mut tabs {
        background.0 = if tab.0 == state.tab {
            Color::srgba(0.22, 0.46, 0.47, 1.0)
        } else {
            Color::srgba(0.10, 0.19, 0.22, 1.0)
        };
    }
    for (interaction, mut background) in &mut buttons {
        background.0 = match interaction {
            Interaction::Pressed => Color::srgba(0.22, 0.46, 0.47, 1.0),
            Interaction::Hovered => Color::srgba(0.16, 0.34, 0.36, 1.0),
            Interaction::None => Color::srgba(0.10, 0.19, 0.22, 1.0),
        };
    }
}

fn inventory_text(nature: &OfflineNature, catalog: &ContentRegistry) -> String {
    let mut text =
        String::from("资源                         拥有\n-------------------------------\n");
    let mut count = 0;
    for kind in ResourceKind::ALL {
        let amount = nature.resources.get(*kind);
        if amount > 0 {
            let name = catalog
                .get(resource_content_id(*kind))
                .map(|definition| definition.display_name.as_str())
                .unwrap_or("未注册资源");
            text.push_str(&format!("{name:<24} {amount:>6}\n"));
            count += 1;
        }
    }
    if count == 0 {
        text.push_str("还没有收集到资源。\n");
    }
    text
}

fn crafting_text(nature: &OfflineNature, catalog: &ContentRegistry) -> String {
    let mut text =
        String::from("配方                         状态\n-------------------------------\n");
    for recipe in catalog.recipes() {
        let output_name = catalog
            .get(recipe.output)
            .map(|definition| definition.display_name.as_str())
            .unwrap_or("未注册产物");
        let ready = recipe.is_ready(&nature.resources);
        text.push_str(&format!(
            "{output_name}  [{}]\n",
            if ready { "可制作" } else { "材料不足" }
        ));
        for ingredient in &recipe.ingredients {
            let current = nature.resources.get(ingredient.resource);
            let ingredient_name = catalog
                .get(resource_content_id(ingredient.resource))
                .map(|definition| definition.display_name.as_str())
                .unwrap_or("未注册资源");
            text.push_str(&format!(
                "  {ingredient_name:<21} {current:>4}/{:<4}\n",
                ingredient.amount
            ));
        }
        text.push('\n');
    }
    if catalog.recipes().is_empty() {
        text.push_str("暂无已登记的配方。");
    }
    text
}
