//! Runtime key bindings and the in-game binding panel.

use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GameAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Attack,
    ToggleCamera,
    CameraTurnLeft,
    CameraTurnRight,
    OpenInventory,
    OpenWorldStatus,
    OpenBindings,
    CycleCrop,
    PlantCrop,
    HarvestCrop,
    OpenFarming,
}

impl GameAction {
    pub const ALL: [Self; 16] = [
        Self::MoveForward,
        Self::MoveBackward,
        Self::MoveLeft,
        Self::MoveRight,
        Self::Jump,
        Self::Attack,
        Self::ToggleCamera,
        Self::CameraTurnLeft,
        Self::CameraTurnRight,
        Self::OpenInventory,
        Self::OpenWorldStatus,
        Self::OpenBindings,
        Self::CycleCrop,
        Self::PlantCrop,
        Self::HarvestCrop,
        Self::OpenFarming,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::MoveForward => "向前移动",
            Self::MoveBackward => "后退",
            Self::MoveLeft => "向左移动",
            Self::MoveRight => "向右移动",
            Self::Jump => "跳跃",
            Self::Attack => "攻击",
            Self::ToggleCamera => "切换视角",
            Self::CameraTurnLeft => "向左转视角",
            Self::CameraTurnRight => "向右转视角",
            Self::OpenInventory => "打开背包",
            Self::OpenWorldStatus => "打开内容调试",
            Self::OpenBindings => "打开设置菜单",
            Self::CycleCrop => "选择作物",
            Self::PlantCrop => "种植作物",
            Self::HarvestCrop => "收获作物",
            Self::OpenFarming => "打开农田",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
    Unbound,
}

impl Binding {
    pub fn label(self) -> String {
        match self {
            Self::Key(key) => key_label(key),
            Self::Mouse(button) => match button {
                MouseButton::Left => "鼠标左键".into(),
                MouseButton::Right => "鼠标右键".into(),
                MouseButton::Middle => "鼠标中键".into(),
                MouseButton::Back => "鼠标后退键".into(),
                MouseButton::Forward => "鼠标前进键".into(),
                MouseButton::Other(index) => format!("鼠标按键 {index}"),
            },
            Self::Unbound => "未绑定".into(),
        }
    }
}

#[derive(Resource)]
pub struct KeyBindings {
    bindings: HashMap<GameAction, Binding>,
    pub menu_open: bool,
    listening: Option<GameAction>,
    capture_blocked: bool,
}

#[derive(Resource)]
pub struct UiSettings {
    pub panel_opacity: f32,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            panel_opacity: 0.96,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SettingsPage {
    Controls,
    Ui,
    Algorithm,
}

impl SettingsPage {
    const ALL: [Self; 3] = [Self::Controls, Self::Ui, Self::Algorithm];

    const fn label(self) -> &'static str {
        match self {
            Self::Controls => "操作",
            Self::Ui => "界面",
            Self::Algorithm => "算法",
        }
    }
}

#[derive(Resource)]
pub struct SettingsUiState {
    pub page: SettingsPage,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            page: SettingsPage::Controls,
        }
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(GameAction::MoveForward, Binding::Key(KeyCode::KeyW));
        bindings.insert(GameAction::MoveBackward, Binding::Key(KeyCode::KeyS));
        bindings.insert(GameAction::MoveLeft, Binding::Key(KeyCode::KeyA));
        bindings.insert(GameAction::MoveRight, Binding::Key(KeyCode::KeyD));
        bindings.insert(GameAction::Jump, Binding::Key(KeyCode::Space));
        bindings.insert(GameAction::Attack, Binding::Mouse(MouseButton::Left));
        bindings.insert(GameAction::ToggleCamera, Binding::Key(KeyCode::KeyC));
        bindings.insert(GameAction::CameraTurnLeft, Binding::Key(KeyCode::KeyQ));
        bindings.insert(GameAction::CameraTurnRight, Binding::Key(KeyCode::KeyE));
        bindings.insert(GameAction::OpenInventory, Binding::Key(KeyCode::KeyI));
        bindings.insert(GameAction::OpenWorldStatus, Binding::Key(KeyCode::F2));
        bindings.insert(GameAction::OpenBindings, Binding::Key(KeyCode::F1));
        bindings.insert(GameAction::CycleCrop, Binding::Key(KeyCode::KeyR));
        bindings.insert(GameAction::PlantCrop, Binding::Key(KeyCode::KeyF));
        bindings.insert(GameAction::HarvestCrop, Binding::Key(KeyCode::KeyG));
        bindings.insert(GameAction::OpenFarming, Binding::Key(KeyCode::KeyO));
        Self {
            bindings,
            menu_open: false,
            listening: None,
            capture_blocked: false,
        }
    }
}

impl KeyBindings {
    pub fn binding(&self, action: GameAction) -> Binding {
        self.bindings
            .get(&action)
            .copied()
            .unwrap_or(Binding::Unbound)
    }

    pub fn pressed(
        &self,
        action: GameAction,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        match self.binding(action) {
            Binding::Key(key) => keys.pressed(key),
            Binding::Mouse(button) => mouse.pressed(button),
            Binding::Unbound => false,
        }
    }

    pub fn just_pressed(
        &self,
        action: GameAction,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        match self.binding(action) {
            Binding::Key(key) => keys.just_pressed(key),
            Binding::Mouse(button) => mouse.just_pressed(button),
            Binding::Unbound => false,
        }
    }

    fn set_binding(&mut self, action: GameAction, binding: Binding) {
        if binding != Binding::Unbound {
            for (other_action, other_binding) in &mut self.bindings {
                if *other_action != action && *other_binding == binding {
                    *other_binding = Binding::Unbound;
                }
            }
        }
        self.bindings.insert(action, binding);
        self.listening = None;
    }

    fn reset(&mut self) {
        *self = Self::default();
        self.menu_open = true;
    }
}

#[derive(Component)]
pub struct KeybindingUiRoot;

#[derive(Component)]
pub struct BindingButton(GameAction);

#[derive(Component)]
pub struct BindingValueText(GameAction);

#[derive(Component)]
pub struct BindingStatusText;

#[derive(Component)]
pub struct ResetBindingsButton;

#[derive(Component)]
pub struct CloseBindingsButton;

#[derive(Component)]
pub(crate) struct SettingsTabButton(pub(crate) SettingsPage);

#[derive(Component)]
pub(crate) struct SettingsSection(pub(crate) SettingsPage);

#[derive(Clone, Copy)]
pub(crate) enum UiSettingControl {
    ScaleDown,
    ScaleUp,
    OpacityDown,
    OpacityUp,
}

#[derive(Component)]
pub(crate) struct UiSettingButton(pub(crate) UiSettingControl);

#[derive(Component)]
pub(crate) struct UiScaleValueText;

#[derive(Component)]
pub(crate) struct UiOpacityValueText;

pub fn setup_keybinding_ui(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(24),
                top: px(24),
                width: px(390),
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
            KeybindingUiRoot,
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text::new("设置"),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(22.0),
                weight: FontWeight::BOLD,
                ..default()
            },
            TextColor(Color::srgb(0.86, 0.96, 0.95)),
        ));
        parent.spawn((
            Text::new("点击要修改的按键，然后按下键盘或鼠标按钮"),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::srgba(0.70, 0.78, 0.80, 0.95)),
        ));

        parent
            .spawn((Node {
                width: percent(100),
                column_gap: px(6),
                ..default()
            },))
            .with_children(|tabs| {
                for page in SettingsPage::ALL {
                    spawn_settings_tab(tabs, page);
                }
            });

        for action in GameAction::ALL {
            parent
                .spawn((
                    Node {
                        width: percent(100),
                        min_height: px(34),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: px(12),
                        ..default()
                    },
                    SettingsSection(SettingsPage::Controls),
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new(action.label()),
                        TextFont {
                            font: FontSource::UiSansSerif,
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
                        Button,
                        Node {
                            min_width: px(128),
                            min_height: px(30),
                            padding: UiRect::axes(px(10), px(5)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(5)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.10, 0.19, 0.22, 1.0)),
                        BorderColor::all(Color::srgba(0.30, 0.60, 0.62, 0.85)),
                        BindingButton(action),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new(""),
                            TextFont {
                                font: FontSource::UiMonospace,
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.90, 0.98, 0.96)),
                            BindingValueText(action),
                        ));
                    });
                });
        }

        parent.spawn((
            Text::new("界面显示"),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(14.0),
                weight: FontWeight::BOLD,
                ..default()
            },
            TextColor(Color::srgb(0.55, 0.84, 0.82)),
            SettingsSection(SettingsPage::Ui),
        ));
        spawn_ui_setting_row(
            parent,
            "界面缩放",
            UiSettingControl::ScaleDown,
            UiSettingControl::ScaleUp,
            UiScaleValueText,
            SettingsPage::Ui,
        );
        spawn_ui_setting_row(
            parent,
            "面板不透明度",
            UiSettingControl::OpacityDown,
            UiSettingControl::OpacityUp,
            UiOpacityValueText,
            SettingsPage::Ui,
        );

        parent
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(42),
                    align_items: AlignItems::Center,
                    ..default()
                },
                SettingsSection(SettingsPage::Algorithm),
            ))
            .with_children(|section| {
                section.spawn((
                    Text::new("暂无算法设置。"),
                    TextFont {
                        font: FontSource::UiSansSerif,
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::srgba(0.70, 0.78, 0.80, 0.95)),
                ));
            });

        parent.spawn((
            Text::new(""),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.78, 0.35)),
            BindingStatusText,
        ));

        parent
            .spawn((Node {
                width: percent(100),
                column_gap: px(8),
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },))
            .with_children(|row| {
                row.spawn((
                    Button,
                    Node {
                        min_width: px(112),
                        min_height: px(30),
                        padding: UiRect::axes(px(10), px(5)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(px(5)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.16, 0.26, 0.28, 1.0)),
                    ResetBindingsButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("恢复默认"),
                        TextFont {
                            font: FontSource::UiSansSerif,
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
                row.spawn((
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
                    CloseBindingsButton,
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

fn spawn_settings_tab(parent: &mut ChildSpawnerCommands, page: SettingsPage) {
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
            SettingsTabButton(page),
        ))
        .with_children(|tab| {
            tab.spawn((
                Text::new(page.label()),
                TextFont {
                    font: FontSource::UiSansSerif,
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_ui_setting_row<C: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    decrease: UiSettingControl,
    increase: UiSettingControl,
    value_marker: C,
    page: SettingsPage,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(30),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(8),
                ..default()
            },
            SettingsSection(page),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: FontSource::UiSansSerif,
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            row.spawn((Node {
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },))
                .with_children(|controls| {
                    spawn_stepper_button(controls, "-", decrease);
                    controls.spawn((
                        Text::new(""),
                        TextFont {
                            font: FontSource::UiMonospace,
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.90, 0.98, 0.96)),
                        value_marker,
                    ));
                    spawn_stepper_button(controls, "+", increase);
                });
        });
}

fn spawn_stepper_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: UiSettingControl,
) {
    parent
        .spawn((
            Button,
            Node {
                width: px(28),
                height: px(26),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.19, 0.22, 1.0)),
            UiSettingButton(control),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font: FontSource::UiMonospace,
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub fn toggle_keybinding_ui(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut bindings: ResMut<KeyBindings>,
) {
    if bindings.just_pressed(GameAction::OpenBindings, &keys, &mouse) {
        bindings.menu_open = !bindings.menu_open;
        if !bindings.menu_open {
            bindings.listening = None;
        }
    }
}

pub fn handle_keybinding_buttons(
    mut bindings: ResMut<KeyBindings>,
    mut ui_settings: ResMut<UiSettings>,
    mut ui_scale: ResMut<UiScale>,
    mut ui_state: ResMut<SettingsUiState>,
    buttons: Query<
        (
            &Interaction,
            Option<&BindingButton>,
            Option<&ResetBindingsButton>,
            Option<&CloseBindingsButton>,
            Option<&UiSettingButton>,
            Option<&SettingsTabButton>,
        ),
        Changed<Interaction>,
    >,
) {
    for (interaction, binding_button, reset_button, close_button, ui_setting_button, tab_button) in
        &buttons
    {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if close_button.is_some() {
            bindings.menu_open = false;
            bindings.listening = None;
        } else if reset_button.is_some() {
            bindings.reset();
            ui_settings.panel_opacity = UiSettings::default().panel_opacity;
            ui_scale.0 = 1.0;
        } else if let Some(tab_button) = tab_button {
            ui_state.page = tab_button.0;
        } else if let Some(ui_setting_button) = ui_setting_button {
            match ui_setting_button.0 {
                UiSettingControl::ScaleDown => ui_scale.0 = (ui_scale.0 - 0.1).max(0.8),
                UiSettingControl::ScaleUp => ui_scale.0 = (ui_scale.0 + 0.1).min(1.4),
                UiSettingControl::OpacityDown => {
                    ui_settings.panel_opacity = (ui_settings.panel_opacity - 0.1).max(0.4)
                }
                UiSettingControl::OpacityUp => {
                    ui_settings.panel_opacity = (ui_settings.panel_opacity + 0.1).min(1.0)
                }
            }
        } else if let Some(binding_button) = binding_button {
            bindings.listening = Some(binding_button.0);
            bindings.capture_blocked = true;
        }
    }
}

pub fn capture_keybinding_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut bindings: ResMut<KeyBindings>,
) {
    if bindings.capture_blocked {
        bindings.capture_blocked = false;
        return;
    }
    let Some(action) = bindings.listening else {
        return;
    };
    if let Some(key) = keys.get_just_pressed().next().copied() {
        bindings.set_binding(action, Binding::Key(key));
    } else if let Some(button) = mouse.get_just_pressed().next().copied() {
        bindings.set_binding(action, Binding::Mouse(button));
    }
}

pub fn update_keybinding_ui(
    bindings: Res<KeyBindings>,
    ui_settings: Res<UiSettings>,
    ui_scale: Res<UiScale>,
    ui_state: Res<SettingsUiState>,
    mut root: Query<&mut Visibility, With<KeybindingUiRoot>>,
    mut sections: Query<(&SettingsSection, &mut Node)>,
    mut values: Query<
        (&BindingValueText, &mut Text),
        (
            Without<BindingStatusText>,
            Without<UiScaleValueText>,
            Without<UiOpacityValueText>,
        ),
    >,
    mut status: Query<
        &mut Text,
        (
            With<BindingStatusText>,
            Without<BindingValueText>,
            Without<UiScaleValueText>,
            Without<UiOpacityValueText>,
        ),
    >,
    mut scale_value: Query<
        &mut Text,
        (
            With<UiScaleValueText>,
            Without<BindingValueText>,
            Without<BindingStatusText>,
            Without<UiOpacityValueText>,
        ),
    >,
    mut opacity_value: Query<
        &mut Text,
        (
            With<UiOpacityValueText>,
            Without<BindingValueText>,
            Without<BindingStatusText>,
            Without<UiScaleValueText>,
        ),
    >,
    mut backgrounds: ParamSet<(
        Query<&mut BackgroundColor, (With<KeybindingUiRoot>, Without<Button>)>,
        Query<(&Interaction, &mut BackgroundColor), (With<Button>, Without<SettingsTabButton>)>,
        Query<(&SettingsTabButton, &mut BackgroundColor)>,
    )>,
) {
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = if bindings.menu_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok(mut background) = backgrounds.p0().single_mut() {
        background.0 = Color::srgba(0.035, 0.05, 0.075, ui_settings.panel_opacity);
    }
    for (section, mut node) in &mut sections {
        node.display = if section.0 == ui_state.page {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (value, mut text) in &mut values {
        text.0 = if bindings.listening == Some(value.0) {
            "Press a key...".into()
        } else {
            bindings.binding(value.0).label()
        };
    }
    if let Ok(mut text) = status.single_mut() {
        text.0 = bindings.listening.map_or_else(
            || "F1 打开或关闭此面板".into(),
            |action| format!("正在设置「{}」：请按下键盘或鼠标按钮", action.label()),
        );
    }
    if let Ok(mut text) = scale_value.single_mut() {
        text.0 = format!("{:.1}x", ui_scale.0);
    }
    if let Ok(mut text) = opacity_value.single_mut() {
        text.0 = format!("{:.1}", ui_settings.panel_opacity);
    }
    for (interaction, mut background) in &mut backgrounds.p1() {
        background.0 = match interaction {
            Interaction::Pressed => Color::srgba(0.22, 0.46, 0.47, 1.0),
            Interaction::Hovered => Color::srgba(0.16, 0.34, 0.36, 1.0),
            Interaction::None => Color::srgba(0.10, 0.19, 0.22, 1.0),
        };
    }
    for (tab, mut background) in &mut backgrounds.p2() {
        background.0 = if tab.0 == ui_state.page {
            Color::srgba(0.22, 0.46, 0.47, 1.0)
        } else {
            Color::srgba(0.10, 0.19, 0.22, 1.0)
        };
    }
}

fn key_label(key: KeyCode) -> String {
    match key {
        KeyCode::Space => "空格".into(),
        KeyCode::Escape => "Esc".into(),
        KeyCode::Enter => "回车".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::ShiftLeft => "左 Shift".into(),
        KeyCode::ShiftRight => "右 Shift".into(),
        KeyCode::ControlLeft => "左 Ctrl".into(),
        KeyCode::ControlRight => "右 Ctrl".into(),
        KeyCode::ArrowUp => "上方向键".into(),
        KeyCode::ArrowDown => "下方向键".into(),
        KeyCode::ArrowLeft => "左方向键".into(),
        KeyCode::ArrowRight => "右方向键".into(),
        other => format!("{other:?}").trim_start_matches("Key").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_cover_core_actions() {
        let bindings = KeyBindings::default();
        assert_eq!(
            bindings.binding(GameAction::Jump),
            Binding::Key(KeyCode::Space)
        );
        assert_eq!(
            bindings.binding(GameAction::Attack),
            Binding::Mouse(MouseButton::Left)
        );
        assert_eq!(
            bindings.binding(GameAction::ToggleCamera),
            Binding::Key(KeyCode::KeyC)
        );
        assert_eq!(
            bindings.binding(GameAction::OpenWorldStatus),
            Binding::Key(KeyCode::F2)
        );
    }

    #[test]
    fn rebinding_removes_duplicate_binding() {
        let mut bindings = KeyBindings::default();
        bindings.set_binding(GameAction::Jump, Binding::Key(KeyCode::KeyC));
        assert_eq!(
            bindings.binding(GameAction::Jump),
            Binding::Key(KeyCode::KeyC)
        );
        assert_eq!(bindings.binding(GameAction::ToggleCamera), Binding::Unbound);
    }
}
