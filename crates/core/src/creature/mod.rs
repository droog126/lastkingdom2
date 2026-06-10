//! 动物 / 被动生物：猪 / 羊 / 鸡
//!
//! 行为：闲逛（每 1.5-3s 选个随机方向走 1 格），遇到障碍就停；头顶晃一晃。
//! 纯客户端渲染（不参与 sim/economy），但提供"活物感"。

use bevy::prelude::*;
use rand::prelude::*;

use crate::player::PlayerState;
use crate::resource::ResourceKind;
use crate::world::BlockType;
use crate::world::World as GameWorld;

// ---------------------------------------------------------------------------
// 物种
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreatureKind {
    Pig,     // 粉红
    Sheep,   // 白色
    Cow,     // 棕白
    Chicken, // 黄色
}

impl CreatureKind {
    pub fn color(self) -> Color {
        match self {
            CreatureKind::Pig => Color::srgb(0.95, 0.75, 0.78),
            CreatureKind::Sheep => Color::srgb(0.96, 0.96, 0.92),
            CreatureKind::Cow => Color::srgb(0.65, 0.45, 0.30),
            CreatureKind::Chicken => Color::srgb(1.00, 0.90, 0.30),
        }
    }
    pub const fn size(self) -> Vec3 {
        match self {
            CreatureKind::Pig => Vec3::new(0.40, 0.32, 0.55),
            CreatureKind::Sheep => Vec3::new(0.42, 0.42, 0.55),
            CreatureKind::Cow => Vec3::new(0.50, 0.50, 0.65),
            CreatureKind::Chicken => Vec3::new(0.22, 0.28, 0.25),
        }
    }
    pub fn label_zh(self) -> &'static str {
        match self {
            CreatureKind::Pig => "猪",
            CreatureKind::Sheep => "羊",
            CreatureKind::Cow => "牛",
            CreatureKind::Chicken => "鸡",
        }
    }
}

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct Creature {
    pub kind: CreatureKind,
    pub block_pos: [i32; 3],
}

#[derive(Component)]
pub struct CreatureAI {
    pub wander_timer: f32,
    pub next_wander_secs: f32,
    pub bob_phase: f32,
}

// ---------------------------------------------------------------------------
// 资源：一次性 spawn 标记
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct CreatureSpawnerDone(pub bool);

// ---------------------------------------------------------------------------
// Spawn：在世界里撒 30 只动物，避开出生点
// ---------------------------------------------------------------------------

pub fn spawn_creatures(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<GameWorld>,
    mut done: ResMut<CreatureSpawnerDone>,
) {
    if done.0 {
        return;
    }
    done.0 = true;

    let s = world.size;
    let mut rng = rand::rng();
    let kinds = [
        CreatureKind::Pig,
        CreatureKind::Sheep,
        CreatureKind::Cow,
        CreatureKind::Chicken,
    ];
    let count = 30;
    let mut placed = 0;
    let mut attempts = 0;
    let spawn_cx = s / 2;
    let spawn_cz = s / 2;

    // 先 spawn 一个"起始牧场"：出生点周围 5-9 格的空地上 spawn 12 只
    // （加密，确保 auto-walk 时玩家总能看到动物）
    let mut starter_attempts = 0;
    while placed < 12 && starter_attempts < 400 {
        starter_attempts += 1;
        let x = spawn_cx + rng.random_range(-8..9);
        let z = spawn_cz + rng.random_range(-8..9);
        if try_spawn_creature(
            &mut commands,
            &mut meshes,
            &mut materials,
            &world,
            &kinds,
            x,
            z,
        ) {
            placed += 1;
        }
    }
    info!("🐄 起始牧场 spawn {} 只", placed);

    // 然后再 spawn 散落的 count 只
    let target = count;
    while placed < target && attempts < count * 20 {
        attempts += 1;
        let x = rng.random_range(2..(s - 2));
        let z = rng.random_range(2..(s - 2));
        // 离出生点 8 格以外（避免覆盖起始牧场）
        if (x - spawn_cx).abs() + (z - spawn_cz).abs() < 8 {
            continue;
        }
        if try_spawn_creature(
            &mut commands,
            &mut meshes,
            &mut materials,
            &world,
            &kinds,
            x,
            z,
        ) {
            placed += 1;
        }
    }
    info!("🐄 总共 spawn {} 只动物 (尝试 {} 次)", placed, attempts);
}

/// 工具：尝试在 (x, z) spawn 一只动物，失败返回 false
fn try_spawn_creature(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    world: &GameWorld,
    kinds: &[CreatureKind; 4],
    x: i32,
    z: i32,
) -> bool {
    let mut rng = rand::rng();
    let s = world.size;
    if x < 2 || x >= s - 2 || z < 2 || z >= s - 2 {
        return false;
    }
    // 找地表
    let mut surface_y = None;
    for y in (1..s).rev() {
        if world.get(x, y, z).is_solid() {
            surface_y = Some(y);
            break;
        }
    }
    let Some(y) = surface_y else {
        return false;
    };
    if !world.get(x, y, z).is_surface() {
        return false;
    }
    let y = y + 1; // 站在地表上一格

    let kind = kinds[rng.random_range(0..kinds.len())];

    let mesh_h = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mat_h = materials.add(StandardMaterial {
        base_color: kind.color(),
        perceptual_roughness: 0.85,
        ..default()
    });

    commands.spawn((
        Creature { kind, block_pos: [x, y, z] },
        CreatureAI {
            wander_timer: 0.0,
            next_wander_secs: rng.random_range(0.5..1.5), // 起始牧场动得快点
            bob_phase: rng.random_range(0.0..std::f32::consts::TAU),
        },
        Mesh3d(mesh_h),
        MeshMaterial3d(mat_h),
        Transform::from_translation(Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5))
            .with_scale(kind.size()),
    ));
    true
}

/// 玩家攻击：K 键一刀秒半径 1.5 格内最近的动物
/// 死亡后掉落食物到 pool，creature entity 移除
pub fn player_attack_creatures(
    keys: Res<ButtonInput<KeyCode>>,
    player: Res<PlayerState>,
    mut pool: ResMut<crate::resource::GlobalResourcePool>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Creature, &mut CreatureAI)>,
) {
    if !keys.just_pressed(KeyCode::KeyK) {
        return;
    }
    let px = player.block_pos[0] as f32 + 0.5;
    let py = player.block_pos[1] as f32 + 0.5;
    let pz = player.block_pos[2] as f32 + 0.5;
    // 找最近的
    let mut best: Option<(Entity, f32, CreatureKind)> = None;
    for (e, c, _) in q.iter() {
        let dx = (c.block_pos[0] as f32 + 0.5) - px;
        let dy = (c.block_pos[1] as f32 + 0.5) - py;
        let dz = (c.block_pos[2] as f32 + 0.5) - pz;
        let d2 = dx * dx + dy * dy + dz * dz;
        if d2 < 2.5 && (best.is_none() || d2 < best.unwrap().1) {
            best = Some((e, d2, c.kind));
        }
    }
    if let Some((e, _d, kind)) = best {
        // 掉落物：每种动物各产一种食物
        let drop = match kind {
            CreatureKind::Pig => crate::resource::ResourceKind::Food,
            CreatureKind::Sheep => crate::resource::ResourceKind::Food,
            CreatureKind::Cow => crate::resource::ResourceKind::Food,
            CreatureKind::Chicken => crate::resource::ResourceKind::Apple,
        };
        let _ = pool.try_add(drop, 3);
        info!("⚔ 你杀了一只{}（+3 {:?}）", kind.label_zh(), drop);
        commands.entity(e).despawn();
    } else {
        info!("⚔ 挥空（范围内没有动物）");
    }
}

// ---------------------------------------------------------------------------
// 每帧 update：闲逛 + 头顶晃
// ---------------------------------------------------------------------------

pub fn update_creatures(
    time: Res<Time>,
    world: Res<GameWorld>,
    mut q: Query<(&mut Creature, &mut CreatureAI, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::rng();
    for (mut creature, mut ai, mut tf) in q.iter_mut() {
        ai.wander_timer += dt;
        ai.bob_phase += dt * 3.0;
        let bob = (ai.bob_phase).sin() * 0.05;
        tf.translation.y = creature.block_pos[1] as f32 + 0.5 + bob;

        if ai.wander_timer < ai.next_wander_secs {
            continue;
        }
        ai.wander_timer = 0.0;
        ai.next_wander_secs = rng.random_range(1.5..3.0);

        // 选个方向：4 水平 + 偶尔原地转身
        let dirs: [[i32; 3]; 4] = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]];
        let d = dirs[rng.random_range(0..dirs.len())];
        let nx = creature.block_pos[0] + d[0];
        let ny = creature.block_pos[1] + d[1];
        let nz = creature.block_pos[2] + d[2];
        if !world.in_bounds(nx, ny, nz) {
            continue;
        }
        // 目标格必须空（Air），下方必须是实心
        if world.get(nx, ny, nz) != BlockType::Air {
            continue;
        }
        if !world.get(nx, ny - 1, nz).is_solid() {
            continue;
        }
        // 转向（用 y 旋转）
        let yaw = match (d[0], d[2]) {
            (1, 0) => std::f32::consts::FRAC_PI_2,
            (-1, 0) => -std::f32::consts::FRAC_PI_2,
            (0, 1) => std::f32::consts::PI,
            (0, -1) => 0.0,
            _ => 0.0,
        };
        creature.block_pos = [nx, ny, nz];
        tf.translation = Vec3::new(nx as f32 + 0.5, ny as f32 + 0.5 + bob, nz as f32 + 0.5);
        tf.rotation = Quat::from_rotation_y(yaw);
    }
}
