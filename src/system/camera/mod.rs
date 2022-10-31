use crate::{constant::FPS, utils::num::MyQueue};
use bevy::{prelude::*, render::camera::RenderTarget, time::FixedTimestep};
use bevy_inspector_egui::{bevy_egui::EguiContext, egui};

use super::{animation::AnimationMachine, inViewPort::CurEntity, instanceInput::InstanceInput};

#[derive(Component)]
pub struct MainCameraTag;

#[derive(Debug)]
pub struct CursorPosition {
    pub x: f32,
    pub y: f32,
}

pub struct CursorDiff(pub Vec3);

pub struct DiffQueue(MyQueue);
impl FromWorld for DiffQueue {
    fn from_world(world: &mut World) -> Self {
        DiffQueue(MyQueue::new(3))
    }
}

pub fn init_camera_system(app: &mut App) {
    app.add_startup_system(index)
        .insert_resource(CursorPosition { x: 0.0, y: 0.0 })
        .insert_resource(CursorDiff(Vec3::new(0.0, 0.0, 0.0)))
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step((1.0 / FPS).into()))
                .with_system(step),
        );
    #[cfg(debug_assertions)]
    {
        app.add_system(debug);
    }
}

pub fn index(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.transform.scale.x = 0.5;
    camera.transform.scale.y = 0.5;
    commands.spawn_bundle(camera).insert(MainCameraTag);
}

fn step(
    wnds: Res<Windows>,
    time: Res<Time>,
    mut cursorPosition: ResMut<CursorPosition>,
    mut cursorDiff: ResMut<CursorDiff>,
    mut query1: Query<(&Camera, &mut Transform), With<MainCameraTag>>,
    mut query2: Query<&GlobalTransform, (With<InstanceInput>, Without<MainCameraTag>)>,
    mut diffQueue: Local<DiffQueue>,
    curEntity: Res<CurEntity>,
) {
    // let mut dir = None;
    let mut pos = None;
    if let Some(entity) = curEntity.0 {
        if let Ok(instanceTransform) = query2.get(entity) {
            pos = Some(Vec2::new(
                instanceTransform.translation().x,
                instanceTransform.translation().y,
            ));
        }
    }

    // 捕获鼠标在Camera的坐标
    for (camera, mut camera_transform) in &mut query1 {
        if let RenderTarget::Window(winId) = camera.target {
            let wnd = wnds.get(winId).unwrap();

            if let Some(screen_pos) = wnd.cursor_position() {
                let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
                let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
                let ndc_to_world =
                    camera_transform.compute_matrix() * camera.projection_matrix().inverse();

                let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

                let world_pos: Vec2 = world_pos.truncate();
                cursorPosition.x = world_pos.x;
                cursorPosition.y = world_pos.y;

                if let Some(_pos) = pos {
                    cursorDiff.0 =
                        (Vec3::new(world_pos.x, world_pos.y, 0.0) - _pos.extend(0.0)).normalize();
                }
            }

            if let Some(_pos) = pos {
                let diff = camera_transform.translation - _pos.extend(0.0);
                let diffLen = diff.length();
                let factor = time.delta_seconds() * 4.0;
                if diffLen >= 20.0 {
                    camera_transform.translation.x -= diff.x * factor;
                    camera_transform.translation.y -= diff.y * factor;
                }
            }
        }
    }
}

pub fn debug(mut egui_context: ResMut<EguiContext>, curPos: Res<CursorPosition>) {
    egui::Window::new("curPos").show(egui_context.ctx_mut(), |ui| {
        ui.set_width(100.0);
        ui.horizontal(|ui| {
            ui.label(format!("x:{:.1} y:{:.1}", curPos.x, curPos.y));
        });
    });
}
