//! 角色控制器系统
//!
//! 核心系统：
//! - `ground_detection`    — 射线检测地面
//! - `character_movement`  — WASD + 跳跃
//! - `auto_step_up`        — 自动爬 1 格
//! - `knockback_decay`     — 击退衰减

use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lk2_core::controller::components::{GroundHit, PlayerCollider, PvPController};
use lk2_core::world::World as GameWorld;

// ---------------------------------------------------------------------------
// 1. 地面检测（射线投射）
// ---------------------------------------------------------------------------

/// 地面检测：从脚底向下射一条短射线
///
/// 检测逻辑：
/// 1. 从玩家脚底（translation.y - half_height）向下射射线
/// 2. 射线长度 = step_height + 0.05（略大于自动爬高度）
/// 3. 如果命中，记录地面法线和距离
/// 4. 更新 is_grounded 状态
pub fn ground_detection(
    time: Res<Time>,
    spatial_query: SpatialQuery,
    mut controllers: Query<(&Transform, &PlayerCollider, &mut PvPController)>,
    // 体素世界碰撞体（如果有）
    _voxel_colliders: Query<Entity, (With<Collider>, Without<PvPController>)>,
) {
    let now = time.elapsed_secs();

    for (transform, collider, mut controller) in controllers.iter_mut() {
        // 脚底位置
        let foot_pos = transform.translation - Vec3::Y * collider.half_height;

        // 射线方向：向下
        let ray_dir = Dir3::NEG_Y;

        // 射线长度：略大于 step_height，确保能检测到脚下方块
        let ray_length = controller.step_height + 0.05;

        // 射线检测
        let hit = spatial_query.cast_ray(
            foot_pos,
            ray_dir,
            ray_length,
            true, // 检测固体
            &SpatialQueryFilter::default(),
        );

        if let Some(hit_result) = hit {
            // 命中地面
            controller.is_grounded = true;
            controller.last_grounded_time = now;
            controller.ground_normal = Some(hit_result.normal);

            // 记录地面碰撞信息（用于斜坡滑动）
            // hit_result.entity 可能是体素方块 entity
        } else {
            // 未命中地面
            // 给一点缓冲时间（0.1 秒），避免刚跳起就判定为"空中"
            if now - controller.last_grounded_time > 0.1 {
                controller.is_grounded = false;
                controller.ground_normal = None;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. 角色移动（WASD + 跳跃）
// ---------------------------------------------------------------------------

/// 角色移动系统
///
/// 输入处理：
/// 1. WASD → 计算移动方向（相对相机朝向）
/// 2. Space → 请求跳跃（只有 grounded 才生效）
/// 3. Shift → 疾跑（速度 +50%）
///
/// 速度应用：
/// - 地面：直接设置 velocity.x/z（瞬时响应）
/// - 空中：平滑过渡（air_control 参数）
/// - 击退硬直：输入削弱
pub fn character_movement(
    mut controllers: Query<(&mut LinearVelocity, &mut PvPController, &Transform)>,
    camera: Query<&Transform, With<Camera3d>>,
    input: Res<ButtonInput<KeyCode>>,
    _time: Res<Time>,
) {
    // 获取相机朝向
    let cam_tf = camera.single().ok();
    let (forward, right) = if let Some(tf) = cam_tf {
        let f = tf.forward();
        let f_h = Vec3::new(f.x, 0.0, f.z);
        let f_n = if f_h.length() > 0.01 {
            f_h.normalize()
        } else {
            Vec3::X
        };
        let r = Vec3::Y.cross(f_n);
        (f_n, r)
    } else {
        (Vec3::X, Vec3::Z)
    };

    // 收集输入
    let mut move_dir = Vec3::ZERO;
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp) {
        move_dir += forward;
    }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) {
        move_dir -= forward;
    }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
        move_dir -= right;
    }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
        move_dir += right;
    }

    // 归一化移动方向
    if move_dir.length() > 0.01 {
        move_dir = move_dir.normalize();
    }

    // 疾跑检测
    let sprinting = input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight);

    // 跳跃请求
    let jump_requested = input.just_pressed(KeyCode::Space);

    for (mut velocity, mut controller, _transform) in controllers.iter_mut() {
        // 更新输入状态
        controller.move_input = Vec2::new(move_dir.x, move_dir.z);
        controller.jump_requested = jump_requested;
        controller.is_sprinting = sprinting;

        // 计算目标速度
        let speed = if sprinting {
            controller.speed * 1.5 // 疾跑 +50%
        } else {
            controller.speed
        };

        // 输入削弱（击退硬直期间）
        let input_mult = controller.input_multiplier();

        // 目标水平速度
        let target_vel = move_dir * speed * input_mult;

        // 应用速度（地面 vs 空中）
        if controller.is_grounded {
            // 地面：瞬时响应（MC 手感）
            velocity.x = target_vel.x;
            velocity.z = target_vel.z;

            // 跳跃：只有 grounded 才能跳
            if jump_requested {
                velocity.y = controller.jump_impulse;
                controller.is_grounded = false; // 立刻标记为空中
            }
        } else {
            // 空中：平滑过渡（air_control）
            let lerp_factor = controller.air_control;
            velocity.x = velocity.x.lerp(target_vel.x, lerp_factor);
            velocity.z = velocity.z.lerp(target_vel.z, lerp_factor);
        }

        // 击退速度叠加（逐渐衰减）
        velocity.x += controller.knockback_velocity.x;
        velocity.z += controller.knockback_velocity.z;
    }
}

// ---------------------------------------------------------------------------
// 3. 自动爬台阶（体素友好）
// ---------------------------------------------------------------------------

/// 自动爬台阶系统
///
/// 体素地形友好：前方有可跨越台阶时自动抬升（不需要按空格）
///
/// 检测逻辑：
/// 1. 检测前方是否有碰撞（shapecast）
/// 2. 如果碰撞高度 < step_height，给一个向上的速度
/// 3. 爬上去后恢复水平速度
pub fn auto_step_up(
    spatial_query: SpatialQuery,
    mut controllers: Query<(
        &mut LinearVelocity,
        &mut PvPController,
        &Transform,
        &PlayerCollider,
    )>,
    _voxel_colliders: Query<Entity, (With<Collider>, Without<PvPController>)>,
) {
    for (mut velocity, controller, transform, collider) in controllers.iter_mut() {
        // 只有在移动且 grounded 时才尝试爬台阶
        if !controller.is_grounded || controller.move_input.length() < 0.01 {
            continue;
        }

        // 移动方向（水平）
        let move_dir = Vec3::new(controller.move_input.x, 0.0, controller.move_input.y);
        if move_dir.length() < 0.01 {
            continue;
        }
        let move_dir = move_dir.normalize();

        // 前方检测：从脚底位置向前 shapecast
        let foot_pos = transform.translation - Vec3::Y * collider.half_height;
        let check_dist = collider.radius + 0.1; // 略大于碰撞体半径

        // 射线检测前方
        let forward_hit = spatial_query.cast_ray(
            foot_pos,
            Dir3::new(move_dir).unwrap_or(Dir3::X),
            check_dist,
            true,
            &SpatialQueryFilter::default(),
        );

        if let Some(hit) = forward_hit {
            // 前方有障碍
            // 检测障碍顶部高度
            let hit_point = foot_pos + move_dir * hit.distance;

            // 检测上方是否有空间（可以爬上去）
            // 从 hit_point + Vec3::Y * step_height 向上射射线
            let step_check_pos = hit_point + Vec3::Y * controller.step_height;
            let above_hit = spatial_query.cast_ray(
                step_check_pos,
                Dir3::NEG_Y,
                controller.step_height + 0.1,
                true,
                &SpatialQueryFilter::default(),
            );

            if let Some(above) = above_hit {
                // 上方有空间，可以爬
                let step_height_actual = above.distance;

                // 如果台阶高度 <= step_height，自动爬
                if step_height_actual <= controller.step_height {
                    // 给一个向上的速度（自动跳）
                    velocity.y = controller.step_speed;

                    // 爬上去后，水平速度保持（不会被台阶卡住）
                    // 这一步在下一帧的 ground_detection 会重新判定 grounded
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. 击退衰减
// ---------------------------------------------------------------------------

/// 击退衰减系统
///
/// 击退速度和硬直时间逐渐衰减
pub fn knockback_decay(time: Res<Time>, mut controllers: Query<&mut PvPController>) {
    let dt = time.delta_secs();

    for mut controller in controllers.iter_mut() {
        // 击退速度衰减（每秒衰减 50%）
        controller.knockback_velocity *= 0.5_f32.powf(dt);

        // 硬直时间衰减
        controller.knockback_stun = (controller.knockback_stun - dt).max(0.0);

        // 如果击退速度很小，清零
        if controller.knockback_velocity.length() < 0.01 {
            controller.knockback_velocity = Vec3::ZERO;
        }
    }
}

// ---------------------------------------------------------------------------
// 5. 输入收集（替代 leafwing-input-manager）
// ---------------------------------------------------------------------------

/// 输入收集系统
///
/// 如果使用 leafwing-input-manager，这个系统可以替代为 ActionState
/// 这里用 Bevy 原生 ButtonInput 作为示例
pub fn collect_input(keys: Res<ButtonInput<KeyCode>>, mut controllers: Query<&mut PvPController>) {
    for mut controller in controllers.iter_mut() {
        // WASD
        let mut input = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            input.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            input.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            input.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            input.x += 1.0;
        }

        // 归一化
        if input.length() > 1.0 {
            input = input.normalize();
        }

        controller.move_input = input;
        controller.jump_requested = keys.just_pressed(KeyCode::Space);
        controller.is_sprinting =
            keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    }
}

// ---------------------------------------------------------------------------
// 6. 体素世界碰撞体生成（可选）
// ---------------------------------------------------------------------------

/// 为体素方块生成碰撞体
///
/// 这个系统在需要时为体素世界中的方块生成 avian3d 碰撞体
/// 注意：体素世界通常有大量方块，不建议为每个方块生成碰撞体
/// 更好的做法：只生成玩家附近的方块碰撞体，或使用自定义碰撞检测
pub fn spawn_voxel_colliders(
    _commands: Commands,
    _game_world: Res<GameWorld>,
    _player: Query<&Transform, With<PvPController>>,
    _existing_colliders: Query<&Transform, (With<Collider>, Without<PvPController>)>,
) {
    // 简化：不自动生成碰撞体
    // 体素碰撞检测应该直接用 World::get() + 自定义 shapecast
    // 这里留一个接口，方便后续扩展
}

// ---------------------------------------------------------------------------
// ControllerPlugin — 把 4 个角色控制系统打包到一个 plugin
// ---------------------------------------------------------------------------
//
// 原 umbrella 的 `src/controller/mod.rs::ControllerPlugin` 把这 4 个系统 add
// 到 FixedUpdate。本 crate 是 client, 把 plugin 复制过来, 行为等价。

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                ground_detection,
                character_movement,
                auto_step_up,
                knockback_decay,
            )
                .chain(),
        );
    }
}
