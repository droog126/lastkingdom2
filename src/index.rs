#[allow(
    dead_code,
    unused_variables,
    unused_mut,
    unused_imports,
    non_snake_case
)]
use bevy::app::App;
use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin, prelude::*,
    render::texture::ImageSettings,
};
use bevy_inspector_egui::{
    bevy_egui::{egui, EguiContext, EguiPlugin},
    WorldInspectorPlugin,
};
pub fn entry(app: &mut App) {
    app.insert_resource(ImageSettings::default_nearest()) // prevents blurry sprites
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb(0.4, 0.4, 0.4)))
        .add_plugins(DefaultPlugins)
        .add_startup_system(Init);
    init_dependence(app);
    app.add_system(UiExample);
}

fn init_dependence(app: &mut App) {
    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(EguiPlugin)
        .add_plugin(WorldInspectorPlugin::new());
}

fn Init(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}

fn UiExample(mut egui_context: ResMut<EguiContext>) {
    egui::Window::new("Hello").show(egui_context.ctx_mut(), |ui| {
        ui.label("world");
    });
}
