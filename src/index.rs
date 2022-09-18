#![allow(
    dead_code,
    unused_variables,
    unused_mut,
    unused_imports,
    non_snake_case,
    unused_assignments,
    non_camel_case_types
)]
use bevy::app::App;
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, render::texture::ImageSettings};
use bevy_inspector_egui::{
    bevy_egui::{egui, EguiContext, EguiPlugin},
    WorldInspectorPlugin,
};

use crate::system::{
    collision::init_ins_collision_dependence, factory::init_ins_factory_dependence,
};

pub fn entry(app: &mut App) {
    app.insert_resource(ImageSettings::default_nearest()) // prevents blurry sprites
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb(0.4, 0.4, 0.4)))
        .add_plugins(DefaultPlugins);
    // 依赖初始化的系统
    init_dependence(app);
    // 实例创建相关的系统
    init_ins_factory_dependence(app);
    // 碰撞相关的系统
    init_ins_collision_dependence(app);

    app.add_system(ui_example);
}

fn init_dependence(app: &mut App) {
    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(EguiPlugin)
        .add_plugin(WorldInspectorPlugin::new());
    app.add_startup_system(init);
}

fn init(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}

fn ui_example(mut egui_context: ResMut<EguiContext>) {
    egui::Window::new("Hello").show(egui_context.ctx_mut(), |ui| {
        ui.label("test");
    });
}
