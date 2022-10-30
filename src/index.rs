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

use crate::system::animation::init_animation;
use crate::system::assets::init_assets;
use crate::system::camera::init_camera_system;
use crate::system::instanceInput::init_instanceInput;
use crate::system::state::init_state;
use crate::system::testSystem::init_test_system;
use crate::system::timeLine::init_timeLine_system;
use crate::system::{
    collision::init_ins_collision_dependence, factory::init_ins_factory_dependence,
};

pub fn entry(app: &mut App) {
    app.insert_resource(ImageSettings::default_nearest()) // prevents blurry sprites
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb(0.4, 0.4, 0.4)))
        .add_plugins(DefaultPlugins);

    // 依赖初始化+外部插件的系统
    init_dependence(app);

    // 相机系统
    init_camera_system(app);

    // 设置状态系统
    init_state(app);
    // 设置时间轴
    init_timeLine_system(app);
    // 资源加载系统
    init_assets(app);

    // 实例创建相关的系统
    init_ins_factory_dependence(app);
    // 碰撞相关的系统
    init_ins_collision_dependence(app);
    // 动画系统
    init_animation(app);
    // 输入系统
    init_instanceInput(app);

    // 线性系统
    init_test_system(app);
}

fn init_dependence(app: &mut App) {
    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(EguiPlugin)
        .add_plugin(WorldInspectorPlugin::new());
}
