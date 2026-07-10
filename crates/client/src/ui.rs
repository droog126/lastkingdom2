use bevy::ecs::system::SystemParam;
use bevy::feathers::{
    theme::{ThemeBackgroundColor, ThemedText},
    tokens,
};
use bevy::prelude::*;
use bevy::text::{FontCx, LetterSpacing, RemSize};

use crate::pvp_systems::HealthHudMarker;
use crate::render::{
    AnimalIndicatorText, CameraAngles, CameraMode, FreeFlyState, NestIndicatorText, Player,
    RenderFeatureSettings,
};
use crate::{
    GameplayFeedbackToast, NetworkStatus, OnlineCommandDiagnostics, OnlineConnectionDiagnostics,
    OnlineNetworkStatus,
};
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

#[derive(SystemParam)]
pub struct HudReadParams<'w> {
    player: Res<'w, PlayerState>,
    pool: Res<'w, GlobalResourcePool>,
    eco: Res<'w, EcoCycle>,
    nations: Res<'w, NationRegistry>,
    monsters: Res<'w, MonsterEcosystem>,
    objectives: Res<'w, Objectives>,
    time: Res<'w, Time>,
    run_mode: Res<'w, ClientRunMode>,
    online_commands: Res<'w, OnlineCommandDiagnostics>,
    online_connection: Res<'w, OnlineConnectionDiagnostics>,
    online_status: Res<'w, OnlineNetworkStatus>,
    match_clock: Res<'w, MatchClock>,
    camera_mode: Res<'w, CameraMode>,
}

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

#[derive(Component)]
pub struct HudFeedbackToastText;

#[derive(Component)]
pub struct NestRadarDot {
    pub slot: usize,
}

#[derive(Component, Default, Clone)]
pub struct FeathersCameraModeText;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FeathersCameraButton {
    pub mode: CameraMode,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FeathersFreeflyButton;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameMenuState {
    pub open: bool,
}

impl Default for GameMenuState {
    fn default() -> Self {
        Self { open: false }
    }
}

#[derive(Component)]
pub struct GameMenuRoot;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderToggle {
    Atmosphere,
    Taa,
    Ssr,
    Ssao,
    VolumetricFog,
}

impl RenderToggle {
    fn label(self) -> &'static str {
        match self {
            Self::Atmosphere => "Atmosphere Sky",
            Self::Taa => "TAA",
            Self::Ssr => "SSR",
            Self::Ssao => "SSAO",
            Self::VolumetricFog => "Volumetric Fog",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Atmosphere => "procedural sky",
            Self::Taa => "temporal anti-aliasing",
            Self::Ssr => "screen reflections",
            Self::Ssao => "contact shadowing",
            Self::VolumetricFog => "raymarched fog",
        }
    }

    fn is_enabled(self, settings: &RenderFeatureSettings) -> bool {
        match self {
            Self::Atmosphere => settings.atmosphere,
            Self::Taa => settings.taa,
            Self::Ssr => settings.ssr,
            Self::Ssao => settings.ssao,
            Self::VolumetricFog => settings.volumetric_fog,
        }
    }

    fn toggle(self, settings: &mut RenderFeatureSettings) {
        match self {
            Self::Atmosphere => settings.atmosphere = !settings.atmosphere,
            Self::Taa => settings.taa = !settings.taa,
            Self::Ssr => settings.ssr = !settings.ssr,
            Self::Ssao => settings.ssao = !settings.ssao,
            Self::VolumetricFog => settings.volumetric_fog = !settings.volumetric_fog,
        }
    }
}

#[derive(Component)]
pub struct RenderToggleStatus {
    pub toggle: RenderToggle,
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

    pub fn snapshot_role(self) -> SnapshotRole {
        match self {
            Self::Offline => SnapshotRole::ClientOffline,
            Self::Online => SnapshotRole::ClientOnline,
        }
    }
}

pub fn setup_fonts(mut commands: Commands, mut font_cx: ResMut<FontCx>) {
    let _ = font_cx.set_system_ui_family("Noto Sans CJK SC");
    let _ = font_cx.set_sans_serif_family("Noto Sans CJK SC");
    let _ = font_cx.set_ui_sans_serif_family("Noto Sans CJK SC");
    let _ = font_cx.set_monospace_family("Cascadia Mono");
    let _ = font_cx.set_ui_monospace_family("Cascadia Mono");
    commands.insert_resource(RemSize(15.0));
}

fn hud_mono_font(size: FontSize) -> TextFont {
    TextFont {
        font: FontSource::UiMonospace,
        font_size: size,
        width: FontWidth::SEMI_CONDENSED,
        ..default()
    }
}

fn hud_label_font(size: FontSize) -> TextFont {
    TextFont {
        font: FontSource::SystemUi,
        font_size: size,
        weight: FontWeight::SEMIBOLD,
        ..default()
    }
}

fn ui_text_layout(justify: Justify) -> TextLayout {
    TextLayout::new(justify, LineBreak::AnyCharacter)
}

pub fn feathers_tools_scene() -> impl SceneList {
    bsn_list![feathers_tools_panel()]
}

fn feathers_tools_panel() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            right: px(12),
            bottom: px(92),
            width: px(270),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(6),
            padding: px(8),
            border_radius: BorderRadius::all(px(4)),
        }
        ThemeBackgroundColor(tokens::PANE_BODY_BG)
        Children [
            (Text("Feathers Tools") ThemedText),
            (Text("Camera: --") ThemedText FeathersCameraModeText),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(4),
                    align_items: AlignItems::Center,
                }
                Children [
                    (
                        Button
                        FeathersCameraButton { mode: CameraMode::FirstPerson }
                        Node {
                            flex_grow: 1.0,
                            min_height: px(28),
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::left(px(4)),
                        }
                        BackgroundColor(Color::srgba(0.23, 0.25, 0.29, 0.95))
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.08))
                        Children [(Text("1P") ThemedText)]
                    ),
                    (
                        Button
                        FeathersCameraButton { mode: CameraMode::ThirdPerson }
                        Node {
                            flex_grow: 1.0,
                            min_height: px(28),
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::ZERO,
                        }
                        BackgroundColor(Color::srgba(0.23, 0.25, 0.29, 0.95))
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.08))
                        Children [(Text("3P") ThemedText)]
                    ),
                    (
                        Button
                        FeathersCameraButton { mode: CameraMode::TopDown }
                        Node {
                            flex_grow: 1.0,
                            min_height: px(28),
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::right(px(4)),
                        }
                        BackgroundColor(Color::srgba(0.23, 0.25, 0.29, 0.95))
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.08))
                        Children [(Text("Top") ThemedText)]
                    )
                ]
            ),
            (
                Button
                FeathersFreeflyButton
                Node {
                    min_height: px(28),
                    display: Display::Flex,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(4)),
                }
                BackgroundColor(Color::srgba(0.23, 0.25, 0.29, 0.95))
                BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.08))
                Children [(Text("Toggle freefly") ThemedText)]
            )
        ]
    }
}

pub fn handle_feathers_tools_buttons(
    mut interactions: Query<
        (
            &Interaction,
            Option<&FeathersCameraButton>,
            Option<&FeathersFreeflyButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut mode: ResMut<CameraMode>,
    mut freefly: ResMut<FreeFlyState>,
) {
    for (interaction, camera_button, freefly_button) in interactions.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(button) = camera_button {
            freefly.enabled = false;
            *mode = button.mode;
            info!("Feathers Tools: CameraMode -> {:?}", button.mode);
        }
        if freefly_button.is_some() {
            freefly.enabled = !freefly.enabled;
            if freefly.enabled {
                *mode = CameraMode::FirstPerson;
                info!("Feathers Tools: freefly enabled");
            } else {
                info!("Feathers Tools: freefly disabled");
            }
        }
    }
}

pub fn update_feathers_tools_status(
    mut q: Query<&mut Text, With<FeathersCameraModeText>>,
    mode: Res<CameraMode>,
    freefly: Res<FreeFlyState>,
) {
    let label = if freefly.enabled {
        "Camera: Freefly".to_string()
    } else {
        format!("Camera: {:?}", *mode)
    };
    for mut text in q.iter_mut() {
        text.0 = label.clone();
    }
}

pub fn update_feathers_tools_buttons(
    mode: Res<CameraMode>,
    freefly: Res<FreeFlyState>,
    mut camera_buttons: Query<
        (
            &FeathersCameraButton,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        Without<FeathersFreeflyButton>,
    >,
    mut freefly_buttons: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<FeathersFreeflyButton>,
    >,
    mut text: Query<(&mut TextColor, Option<&mut Text>)>,
) {
    for (button, interaction, mut background, mut border, children) in camera_buttons.iter_mut() {
        let active = !freefly.enabled && *mode == button.mode;
        style_tool_button(
            active,
            *interaction,
            &mut background,
            &mut border,
            children,
            &mut text,
        );
    }
    for (interaction, mut background, mut border, children) in freefly_buttons.iter_mut() {
        style_tool_button(
            freefly.enabled,
            *interaction,
            &mut background,
            &mut border,
            children,
            &mut text,
        );
    }
}

fn style_tool_button(
    active: bool,
    interaction: Interaction,
    background: &mut BackgroundColor,
    border: &mut BorderColor,
    children: &Children,
    text: &mut Query<(&mut TextColor, Option<&mut Text>)>,
) {
    let base = if active {
        Color::srgba(0.08, 0.42, 0.82, 0.98)
    } else {
        Color::srgba(0.25, 0.27, 0.31, 0.94)
    };
    background.0 = match interaction {
        Interaction::Pressed => Color::srgba(0.06, 0.32, 0.66, 1.0),
        Interaction::Hovered => base.with_alpha(1.0),
        Interaction::None => base,
    };
    *border = BorderColor::all(if active {
        Color::srgba(0.48, 0.78, 1.0, 0.62)
    } else {
        Color::srgba(1.0, 1.0, 1.0, 0.08)
    });
    for child in children.iter() {
        if let Ok((mut color, _)) = text.get_mut(child) {
            color.0 = if active {
                Color::srgb(0.96, 0.99, 1.0)
            } else {
                Color::srgb(0.86, 0.88, 0.91)
            };
        }
    }
}

pub fn setup_game_menu(mut commands: Commands) {
    let toggles = [
        RenderToggle::Atmosphere,
        RenderToggle::Taa,
        RenderToggle::Ssr,
        RenderToggle::Ssao,
        RenderToggle::VolumetricFog,
    ];

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                display: Display::None,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(16)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.04, 0.54)),
            GlobalZIndex(50),
            GameMenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(420),
                    max_width: percent(92),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    padding: UiRect::all(px(14)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.10, 0.12, 0.94)),
                BorderColor::all(Color::srgba(0.72, 0.83, 0.92, 0.22)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Game Menu"),
                    hud_label_font(FontSize::Rem(1.04)),
                    TextColor(Color::srgb(0.94, 0.97, 1.0)),
                ));
                panel.spawn((
                    Text::new("Render"),
                    hud_label_font(FontSize::Rem(0.82)),
                    TextColor(Color::srgb(0.70, 0.82, 0.90)),
                ));
                for toggle in toggles {
                    panel.spawn(render_toggle_row(toggle));
                }
                panel.spawn((
                    Text::new("M / Esc closes menu"),
                    hud_mono_font(FontSize::Rem(0.62)),
                    TextColor(Color::srgba(0.82, 0.88, 0.92, 0.72)),
                ));
            });
        });
}

fn render_toggle_row(toggle: RenderToggle) -> impl Bundle {
    (
        Button,
        RenderToggleStatus { toggle },
        Node {
            min_height: px(42),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: px(10),
            padding: UiRect::axes(px(10), px(6)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.13, 0.16, 0.18, 0.88)),
        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.10)),
        children![
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2),
                    flex_grow: 1.0,
                    ..default()
                },
                children![
                    (
                        Text::new(toggle.label()),
                        hud_label_font(FontSize::Rem(0.78)),
                        TextColor(Color::srgb(0.94, 0.96, 0.98)),
                    ),
                    (
                        Text::new(toggle.description()),
                        hud_mono_font(FontSize::Rem(0.58)),
                        TextColor(Color::srgba(0.74, 0.80, 0.84, 0.78)),
                    )
                ],
            ),
            (
                Text::new("OFF"),
                hud_mono_font(FontSize::Rem(0.72)),
                TextColor(Color::srgb(0.78, 0.82, 0.86)),
            )
        ],
    )
}

pub fn toggle_game_menu_input(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<GameMenuState>) {
    if keys.just_pressed(KeyCode::KeyM) || keys.just_pressed(KeyCode::Escape) {
        state.open = !state.open;
    }
}

pub fn sync_game_menu_visibility(
    state: Res<GameMenuState>,
    mut roots: Query<&mut Node, With<GameMenuRoot>>,
) {
    if !state.is_changed() {
        return;
    }
    for mut node in roots.iter_mut() {
        node.display = if state.open {
            Display::Flex
        } else {
            Display::None
        };
    }
}

pub fn handle_render_toggle_buttons(
    mut interactions: Query<
        (&Interaction, &RenderToggleStatus),
        (Changed<Interaction>, With<Button>),
    >,
    mut settings: ResMut<RenderFeatureSettings>,
) {
    for (interaction, toggle) in interactions.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        toggle.toggle.toggle(&mut settings);
        info!(
            "Render menu: {} -> {}",
            toggle.toggle.label(),
            toggle.toggle.is_enabled(&settings)
        );
    }
}

pub fn update_render_toggle_buttons(
    settings: Res<RenderFeatureSettings>,
    mut buttons: Query<(
        &RenderToggleStatus,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
        &Children,
    )>,
    mut text: Query<(&mut Text, &mut TextColor)>,
) {
    if !settings.is_changed() {
        return;
    }

    for (toggle, interaction, mut background, mut border, children) in buttons.iter_mut() {
        let enabled = toggle.toggle.is_enabled(&settings);
        let base = if enabled {
            Color::srgba(0.12, 0.34, 0.24, 0.92)
        } else {
            Color::srgba(0.13, 0.16, 0.18, 0.88)
        };
        background.0 = match *interaction {
            Interaction::Pressed => Color::srgba(0.20, 0.44, 0.34, 0.96),
            Interaction::Hovered => base.with_alpha(1.0),
            Interaction::None => base,
        };
        *border = BorderColor::all(if enabled {
            Color::srgba(0.48, 0.92, 0.68, 0.46)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.10)
        });

        if let Some(status_entity) = children.iter().last()
            && let Ok((mut status, mut color)) = text.get_mut(status_entity)
        {
            status.0 = if enabled { "ON" } else { "OFF" }.to_string();
            color.0 = if enabled {
                Color::srgb(0.62, 1.0, 0.72)
            } else {
                Color::srgb(0.78, 0.82, 0.86)
            };
        }
    }
}

pub fn setup_hud(mut commands: Commands) {
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
            hud_mono_font(FontSize::Rem(0.68)),
            ui_text_layout(Justify::Left),
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.96)),
            LetterSpacing::Px(0.6),
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
        hud_label_font(FontSize::Rem(0.68)),
        ui_text_layout(Justify::Left),
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
        hud_label_font(FontSize::Vh(1.65)),
        TextColor(Color::srgba(0.95, 0.95, 0.95, 1.0)),
        ui_text_layout(Justify::Center),
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
            hud_label_font(FontSize::Rem(0.76)),
            ui_text_layout(Justify::Center),
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
            hud_label_font(FontSize::Rem(0.68)),
            ui_text_layout(Justify::Center),
            TextColor(Color::srgba(1.0, 0.6, 0.4, 0.6)),
            TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
            NestIndicatorText,
        )],
    ));

    commands.spawn((
        Text::new("HP 100/100"),
        TextFont {
            font: FontSource::SystemUi,
            font_size: FontSize::Rem(1.05),
            weight: FontWeight::BOLD,
            width: FontWidth::SEMI_CONDENSED,
            ..default()
        },
        ui_text_layout(Justify::Left),
        TextColor(Color::srgb(1.0, 0.4, 0.4)),
        LetterSpacing::Px(0.4),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(12), right: px(12), ..default() },
        HudHpText,
        HealthHudMarker,
    ));
    commands.spawn((
        Text::new("STA 100/100"),
        hud_mono_font(FontSize::Rem(0.78)),
        ui_text_layout(Justify::Left),
        TextColor(Color::srgb(0.4, 0.8, 1.0)),
        LetterSpacing::Px(0.3),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(38), right: px(12), ..default() },
        HudStaText,
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(82),
            right: px(12),
            width: px(86),
            height: px(86),
            border_radius: BorderRadius::all(px(43)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.06, 0.08, 0.46)),
        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.22)),
        children![
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(41),
                    top: px(41),
                    width: px(4),
                    height: px(4),
                    border_radius: BorderRadius::all(px(2)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.90)),
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(40),
                    top: px(8),
                    width: px(6),
                    height: px(6),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 0.85, 0.25, 0.45)),
            ),
            radar_dot(0),
            radar_dot(1),
            radar_dot(2),
        ],
    ));
    commands.spawn((
        Text::new("Phase: --"),
        TextFont {
            font: FontSource::Family("Noto Sans CJK SC".into()),
            font_size: FontSize::Rem(0.68),
            weight: FontWeight::SEMIBOLD,
            ..default()
        },
        ui_text_layout(Justify::Left),
        TextColor(Color::srgb(0.9, 0.9, 0.5)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(60), right: px(12), ..default() },
        HudPhaseText,
    ));
    commands.spawn((
        Text::new("I/O=Light/Heavy  L=Thrust\nU=Block  Y=Parry"),
        hud_mono_font(FontSize::Rem(0.66)),
        ui_text_layout(Justify::Left),
        TextColor(Color::srgba(0.85, 0.85, 0.85, 0.85)),
        LetterSpacing::Px(0.2),
        TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, bottom: px(56), right: px(12), ..default() },
    ));
    commands.spawn((
        Text::new("Objective: -"),
        hud_label_font(FontSize::Rem(0.76)),
        ui_text_layout(Justify::Left),
        TextColor(Color::srgb(0.85, 0.95, 1.0)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node { position_type: PositionType::Absolute, top: px(106), left: px(12), ..default() },
        HudObjectiveText,
    ));
    commands.spawn((
        Text::new(""),
        TextFont {
            font: FontSource::SystemUi,
            font_size: FontSize::Vh(4.2),
            weight: FontWeight::EXTRA_BOLD,
            style: FontStyle::Italic,
            width: FontWidth::SEMI_EXPANDED,
            ..default()
        },
        ui_text_layout(Justify::Center),
        TextColor(Color::srgba(1.0, 0.95, 0.4, 0.95)),
        LetterSpacing::Px(1.0),
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

    commands.spawn((
        Text::new(""),
        hud_label_font(FontSize::Rem(0.84)),
        ui_text_layout(Justify::Center),
        TextColor(Color::srgba(0.86, 0.96, 1.0, 0.0)),
        TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
        Node {
            position_type: PositionType::Absolute,
            top: px(112),
            left: Val::Percent(0.0),
            right: Val::Percent(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        HudFeedbackToastText,
    ));
}

fn radar_dot(slot: usize) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(39),
            top: px(39),
            width: px(8),
            height: px(8),
            display: Display::None,
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 0.35, 0.25, 0.90)),
        NestRadarDot { slot },
    )
}

pub fn update_nest_radar(
    player: Res<PlayerState>,
    angles: Res<CameraAngles>,
    monsters: Res<MonsterEcosystem>,
    mut q: Query<(&NestRadarDot, &mut Node, &mut BackgroundColor)>,
) {
    let player_x = player.pos.x;
    let player_z = player.pos.z;
    let (sy, cy) = angles.yaw.sin_cos();
    let forward = Vec2::new(sy, -cy);
    let right = Vec2::new(cy, sy);

    let mut nests: Vec<(f32, Vec2, Color)> = Vec::new();
    for kingdom in monsters.kingdoms.values() {
        if kingdom.destroyed {
            continue;
        }
        for nest in kingdom.nests.values() {
            if nest.individuals.is_empty() {
                continue;
            }
            let v = Vec2::new(
                nest.center[0] as f32 + 0.5 - player_x,
                nest.center[2] as f32 + 0.5 - player_z,
            );
            let color = match nest.biome {
                lk2_core::world::Biome::Desert => Color::srgba(1.0, 0.78, 0.22, 0.92),
                lk2_core::world::Biome::Jungle => Color::srgba(0.25, 0.95, 0.40, 0.92),
                lk2_core::world::Biome::Tundra => Color::srgba(0.45, 0.82, 1.0, 0.92),
            };
            nests.push((v.length_squared(), v, color));
        }
    }
    nests.sort_by(|a, b| a.0.total_cmp(&b.0));

    for (dot, mut node, mut bg) in q.iter_mut() {
        let Some((_, v, color)) = nests.get(dot.slot).copied() else {
            node.display = Display::None;
            continue;
        };
        let local = Vec2::new(v.dot(right), v.dot(forward));
        let p = radar_project(local, 80.0, 34.0);
        node.display = Display::Flex;
        node.left = px(43.0 + p.x - 4.0);
        node.top = px(43.0 - p.y - 4.0);
        bg.0 = color;
    }
}

fn radar_project(local: Vec2, max_world_dist: f32, radius_px: f32) -> Vec2 {
    if local.length_squared() < 0.0001 {
        return Vec2::ZERO;
    }
    let scaled = local * (radius_px / max_world_dist);
    if scaled.length() > radius_px {
        scaled.normalize() * radius_px
    } else {
        scaled
    }
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
    hud: HudReadParams,
    mut completed_events: MessageReader<lk2_core::objectives::ObjectiveCompleted>,
    hud_state_q: Query<&GameplayHudState>,
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
    if *hud.camera_mode == CameraMode::TopDown {
        if let Ok(mut text) = q_hud.p0().single_mut() {
            text.0.clear();
        }
        if let Ok(mut text) = q_hud.p1().single_mut() {
            text.0.clear();
        }
        if let Ok(mut text) = q_hud.p2().single_mut() {
            text.0.clear();
        }
        if let Ok(mut text) = q_hud.p3().single_mut() {
            text.0.clear();
        }
        if let Ok(mut text) = q_hud.p4().single_mut() {
            text.0.clear();
        }
        if let Ok((mut text, mut flash)) = q_hud.p5().single_mut() {
            text.0.clear();
            flash.text.clear();
        }
        if let Ok(mut text) = q_hud.p6().single_mut() {
            text.0.clear();
        }
        return;
    }

    let fps = (1.0 / hud.time.delta_secs().max(0.001)).round() as i32;
    let hud_state = hud_state_q.iter().next();
    let wood = hud_state.map(|s| s.pool_wood).unwrap_or(hud.pool.get(ResourceKind::Wood));
    let food = hud_state.map(|s| s.pool_food).unwrap_or(hud.pool.get(ResourceKind::Food));
    let apple = hud_state.map(|s| s.pool_apple).unwrap_or(hud.pool.get(ResourceKind::Apple));
    let soul = hud_state.map(|s| s.pool_soul).unwrap_or(hud.pool.get(ResourceKind::Soul));
    let flags = hud_state.map(|s| s.flag_count).unwrap_or(hud.nations.flag_count);
    let monster_count =
        hud_state.map(|s| s.monster_count).unwrap_or(hud.monsters.current_individuals);
    let network_line = format_network_hud_line(
        *hud.run_mode,
        OnlineNetworkStatus { status: hud.online_status.status },
        hud.time.elapsed_secs(),
        &hud.online_connection,
        &hud.online_commands,
    );

    if let Ok(mut text) = q_hud.p0().single_mut() {
        let phase_remaining = hud.match_clock.phase_remaining_secs();
        let m = (phase_remaining / 60.0).floor() as i32;
        let s = (phase_remaining - m as f32 * 60.0).floor() as i32;
        let phase_line = format!(
            "Phase: {}  T-{:02}:{:02}",
            hud.match_clock.phase.label_zh(),
            m,
            s
        );
        **text = format_main_hud(
            hud.run_mode.label(),
            fps,
            hud.player.pos,
            hud.player.block_pos,
            &phase_line,
            network_line.as_deref(),
            wood,
            food,
            apple,
            soul,
            flags,
            8,
            monster_count,
            hud.eco.rabbit_count(),
            hud.eco.berry_count(),
            hud.eco.total_fruit(),
            hud.eco.co2,
            hud.eco.fruit_eaten,
            hud.eco.fruit_grown,
        );
    }

    let (goal_text, status) = if let Some(obj) = hud.objectives.current() {
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
            "ECO LOOP: clouds {} rain {:.1} -> plants {}(+{}) -> rabbits {}(+{}) -> wildlife {}(+{})",
            hud.eco.clouds.len(),
            hud.eco.rainfall,
            hud.eco.plant_count(),
            hud.eco.plants_grown,
            hud.eco.rabbit_count(),
            hud.eco.rabbits_born,
            hud.eco.wildlife_count(),
            hud.eco.wildlife_born
        );
    }

    let now_secs = hud.time.elapsed_secs();
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

pub fn update_feedback_toast_text(
    time: Res<Time>,
    toast: Res<GameplayFeedbackToast>,
    mut q: Query<(&mut Text, &mut TextColor), With<HudFeedbackToastText>>,
) {
    let Ok((mut text, mut color)) = q.single_mut() else {
        return;
    };
    let Some(summary) = toast.visible_summary(time.elapsed_secs()) else {
        text.0.clear();
        color.0 = color.0.with_alpha(0.0);
        return;
    };

    text.0 = if toast.ok {
        summary.to_string()
    } else {
        format!("Blocked: {summary}")
    };
    color.0 = if toast.ok {
        Color::srgba(0.72, 1.0, 0.78, 0.92)
    } else {
        Color::srgba(1.0, 0.62, 0.48, 0.92)
    };
}

pub fn format_main_hud(
    run_mode_label: &str,
    fps: i32,
    player_pos: Vec3,
    player_block_pos: [i32; 3],
    phase_line: &str,
    network_line: Option<&str>,
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
    let network_line =
        network_line.map_or_else(String::new, |line| format!("\n         > NET {line}"));
    format!(
        "> WANGUO ORIGINS v0.4 | {run_mode_label} | {fps} fps | {phase_line}\n\
         > POS x {x:.1} y {y:.1} z {z:.1} | block {bx},{by},{bz}\n\
         > RES wood {wood} food {food} apple {apple} soul {soul}\n\
         > WORLD flags {flags}/{flag_cap} monsters {monsters}\n\
         > ECO rabbits {rabbits}/5 berries {berry_bushes}/10 fruit {fruit} CO2 {co2:.1} eat/grow {fruit_eaten}/{fruit_grown}{network_line}",
        x = player_pos.x,
        y = player_pos.y,
        z = player_pos.z,
        bx = player_block_pos[0],
        by = player_block_pos[1],
        bz = player_block_pos[2],
    )
}

pub fn format_network_hud_line(
    run_mode: ClientRunMode,
    status: OnlineNetworkStatus,
    now_secs: f32,
    connection: &OnlineConnectionDiagnostics,
    commands: &OnlineCommandDiagnostics,
) -> Option<String> {
    if run_mode != ClientRunMode::Online {
        return None;
    }

    let snapshot_age = connection
        .snapshot_age_secs(now_secs)
        .map_or_else(|| "--".to_string(), |age| format!("{age:.1}s"));
    let pos_age = connection
        .server_pos_age_secs(now_secs)
        .map_or_else(|| "--".to_string(), |age| format!("{age:.1}s"));
    let pong_age = connection
        .pong_age_secs(now_secs)
        .map_or_else(|| "--".to_string(), |age| format!("{age:.1}s"));
    let ping =
        connection.ping_ms.map_or_else(|| "--".to_string(), |ping_ms| format!("{ping_ms:.0}ms"));
    let server =
        connection.server_addr.map_or_else(|| "server ?".to_string(), |addr| addr.to_string());
    let command_path = if commands.gameplay_udp_active {
        "udp"
    } else if commands.sender_entities > 0 {
        "msg"
    } else {
        "idle"
    };
    let disconnect_reason = connection
        .disconnect_reason
        .as_deref()
        .filter(|_| status.status == NetworkStatus::Disconnected)
        .map(|reason| {
            let reason = reason.chars().take(80).collect::<String>();
            format!(" reason {reason}")
        })
        .unwrap_or_default();

    Some(format!(
        "{} {server} id {:04} ping {ping} pong {pong_age} snap {snapshot_age} pos {pos_age} tick {} drift {:.2} corr {:.2}{disconnect_reason} | {command_path} sent {} dir {},{}",
        status.status.label(),
        connection.client_id % 10_000,
        connection.last_server_tick,
        connection.last_server_drift,
        connection.last_server_correction,
        commands.move_world_sent,
        commands.last_dx_milli,
        commands.last_dz_milli,
    ))
}

#[cfg(test)]
mod tests {
    use super::{ClientRunMode, format_main_hud, format_network_hud_line, radar_project};
    use crate::{
        NetworkStatus, OnlineCommandDiagnostics, OnlineConnectionDiagnostics, OnlineNetworkStatus,
    };
    use bevy::prelude::{Vec2, Vec3};

    #[test]
    fn format_main_hud_includes_eco_cycle() {
        let s = format_main_hud(
            "OFFLINE",
            60,
            Vec3::new(48.5, 16.0, 48.5),
            [48, 16, 48],
            "Phase: x",
            None,
            1,
            2,
            3,
            4,
            1,
            8,
            60,
            5,
            10,
            7,
            0.8,
            11,
            12,
        );
        assert_eq!(s.matches('\n').count(), 4);
        assert!(s.contains("POS x 48.5 y 16.0 z 48.5 | block 48,16,48"));
        assert!(s.contains("rabbits 5/5"));
        assert!(s.contains("berries 10/10"));
        assert!(s.contains("CO2 0.8"));
        assert!(s.contains("eat/grow 11/12"));
        assert!(!s.contains("> NET"));
    }

    #[test]
    fn format_main_hud_can_include_online_status() {
        let s = format_main_hud(
            "ONLINE",
            60,
            Vec3::new(1.0, 2.0, 3.0),
            [1, 2, 3],
            "Phase: x",
            Some("live 127.0.0.1:5000"),
            1,
            2,
            3,
            4,
            1,
            8,
            60,
            5,
            10,
            7,
            0.8,
            11,
            12,
        );
        assert_eq!(s.matches('\n').count(), 5);
        assert!(s.contains("> NET live 127.0.0.1:5000"));
    }

    #[test]
    fn format_network_hud_line_tracks_online_freshness() {
        let connection = OnlineConnectionDiagnostics {
            client_entity: None,
            server_addr: Some("127.0.0.1:5000".parse().unwrap()),
            client_id: 12_345,
            transport_state: crate::TransportConnectionState::Connected,
            disconnect_reason: None,
            snapshot_count: 3,
            server_pos_updates: 9,
            ping_sequence: 4,
            ping_ms: Some(31.5),
            last_snapshot_secs: Some(9.5),
            last_server_pos_secs: Some(9.75),
            last_pong_secs: Some(9.9),
            last_server_tick: 42,
            last_server_drift: 0.25,
            last_server_correction: 0.0,
        };
        let commands = OnlineCommandDiagnostics {
            sender_entities: 1,
            move_world_sent: 7,
            last_dx_milli: 1000,
            last_dz_milli: -250,
            gameplay_udp_active: true,
        };
        let status = OnlineNetworkStatus { status: NetworkStatus::Connected };

        let line = format_network_hud_line(
            ClientRunMode::Online,
            status.clone(),
            10.0,
            &connection,
            &commands,
        )
        .unwrap();
        assert!(line.contains("live 127.0.0.1:5000 id 2345 ping 32ms"));
        assert!(line.contains("pong 0.1s snap 0.5s pos 0.2s"));
        assert!(line.contains("udp sent 7 dir 1000,-250"));
        assert!(
            format_network_hud_line(ClientRunMode::Offline, status, 10.0, &connection, &commands,)
                .is_none()
        );

        let disconnected = OnlineConnectionDiagnostics {
            disconnect_reason: Some("Link failed: timeout".into()),
            ..connection
        };
        let line = format_network_hud_line(
            ClientRunMode::Online,
            OnlineNetworkStatus { status: NetworkStatus::Disconnected },
            10.0,
            &disconnected,
            &commands,
        )
        .unwrap();
        assert!(line.contains("reason Link failed: timeout"));
    }

    #[test]
    fn radar_project_clamps_to_circle() {
        let near = radar_project(Vec2::new(20.0, 0.0), 80.0, 34.0);
        assert!((near.x - 8.5).abs() < 0.01);
        assert!(near.y.abs() < 0.01);

        let far = radar_project(Vec2::new(1000.0, 1000.0), 80.0, 34.0);
        assert!(far.length() <= 34.01);
    }
}
