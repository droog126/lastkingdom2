use crate::utils::num::MyQueue;
use bevy::{prelude::*, render::camera::RenderTarget};

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
        .add_system(step);
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
    // mut query2:Query<(&InsInput, &GlobalTransform), With<PlayerTag>>,
    mut diffQueue: Local<DiffQueue>,
) {
    // let mut dir = None;
    // let mut playerPosition = None;

    // for (insInput, playerTransform) in &mut query1) {
    //     dir = Some(insInput.dir.clone());
    //     playerPosition = Some(playerTransform.translation());
    // }

    // 捕获鼠标在Camera的坐标
    for (camera, mut camera_transform) in &mut query1 {
        if let RenderTarget::Window(winId) = camera.target {
            let wnd = wnds.get(winId).unwrap();

            // check if the cursor is inside the window and get its position
            if let Some(screen_pos) = wnd.cursor_position() {
                let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
                let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

                // matrix for undoing the projection and camera transform
                let ndc_to_world =
                    camera_transform.compute_matrix() * camera.projection_matrix().inverse();

                let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

                let world_pos: Vec2 = world_pos.truncate();
                cursorPosition.x = world_pos.x;
                cursorPosition.y = world_pos.y;

                // //diff一下鼠标和玩家的方向
                // if let Some(pos) = playerPosition {
                //     cursorDiff.0 = (Vec3::new(world_pos.x, world_pos.y, 0.0) - pos).normalize();
                // }
            }

            // // 任务:debug控制相机
            // if debugStatus.camera_debug && dir != None {
            //     let unwrapDir = dir.unwrap();
            //     camera_transform.translation.x += unwrapDir.x;
            //     camera_transform.translation.y += unwrapDir.y;
            // }

            // // 任务:跟随玩家
            // if !debugStatus.camera_debug && playerPosition != None {
            //     let mut unwrapPlayerPosition = playerPosition.unwrap();
            //     unwrapPlayerPosition.z = camera_transform.translation.z;

            //     let diff = camera_transform.translation - unwrapPlayerPosition;
            //     let diffLen = diff.length();
            //     //1/4秒回到目标身上
            //     let factor = time.delta_seconds() * 4.0;
            //     diffQueue.0.add(diffLen);
            //     let iSIncreased = diffQueue.0.iSIncreased();

            //     if diffLen <= 20.0 {
            //     } else {
            //         camera_transform.translation -= diff * factor;
            //     }
            // }
        }
    }
}
