//! 体素渲染：把 World 的方块转成 PBR cube
//!
//! 策略：玩家周围 R 半径内的 solid 块 → spawn 一个 Mesh3d+MeshMaterial3d entity
//! 性能：3D scene 持 ~2000 个 entity 没问题；超过会卡

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::constant;
use crate::creature::Creature;
use crate::monster::MonsterEcosystem;
use crate::nation::NationRegistry;
use crate::resource::ResourceKind;
use crate::world::{BlockType, World as GameWorld};

/// 体素渲染配置
#[derive(Resource, Debug, Clone)]
pub struct RenderConfig {
    pub radius: i32,                  // 渲染半径（玩家 ±R）
    pub max_blocks: usize,            // 一次性最多 spawn 多少个
    pub y_offset: f32,                // 玩家脚下贴图偏移（让 y=0 在地面）
    pub sky_color: Color,
    pub fog_color: Color,
    pub fog_start: f32,
    pub fog_end: f32,
    pub auto_orbit: bool,
    pub auto_orbit_speed: f32,
    pub auto_orbit_distance: f32,
    pub auto_walk: bool,
    pub auto_walk_interval_secs: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            radius: 16,
            max_blocks: 3000,
            y_offset: 0.0,
            sky_color: Color::srgb(0.45, 0.65, 0.95), // 亮天蓝
            fog_color: Color::srgb(0.75, 0.82, 0.95),
            fog_start: 18.0,
            fog_end: 48.0,
            auto_orbit: false,            // 默认玩家控制；--auto-demo 开启（loop.ps1 用）
            auto_orbit_speed: 0.22,
            auto_orbit_distance: 15.0,
            auto_walk: false,             // 默认玩家控制；--auto-demo 开启
            auto_walk_interval_secs: 1.2,
        }
    }
}

/// 已 spawn 的方块 entity 列表（用于 despawn 重生）
#[derive(Resource, Default)]
pub struct SpawnedBlocks {
    pub entities: Vec<Entity>,
    /// 上次 spawn 时用的玩家位置（玩家移动 > 1 格才重新 spawn）
    pub last_player_block: [i32; 3],
}

/// 玩家 + 相机 marker
#[derive(Component)]
pub struct PlayerCube;

/// 启动时 spawn 玩家周围方块
pub fn spawn_terrain_around_player(
    mut commands: Commands,
    game_world: Res<GameWorld>,
    cfg: Res<RenderConfig>,
    player: Res<PlayerState>,
    mut spawned: ResMut<SpawnedBlocks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    // 限流：同一次刷屏只 warn 一次（1 秒间隔，按真实时间）
    mut last_warn_time: Local<f32>,
) {
    // 如果上次就在这，skip（玩家没移动）
    if spawned.last_player_block == player.block_pos && !spawned.entities.is_empty() {
        return;
    }

    // 清掉上一次的
    for e in spawned.entities.drain(..) {
        commands.entity(e).despawn();
    }

    // 准备 12 种 BlockType 对应的材质（共享，减少 GPU 状态切换）
    let mut mats: HashMap<BlockType, Handle<StandardMaterial>> = HashMap::new();
    for bt in [
        BlockType::Dirt,
        BlockType::Stone,
        BlockType::Sand,
        BlockType::Snow,
        BlockType::Leaves,
        BlockType::Water,
        BlockType::Wood,
        BlockType::IronOre,
        BlockType::SunstoneOre,
        BlockType::FrostcoreOre,
        BlockType::LivingRoot,
        BlockType::BerryThicket,
    ] {
        let c = bt.debug_color_rgba();
        let emissive = if matches!(bt, BlockType::BerryThicket | BlockType::SunstoneOre) {
            Color::srgb(c[0] * 0.3, c[1] * 0.3, c[2] * 0.3)
        } else {
            Color::BLACK
        };
        // Water: 半透明 + 蓝绿色 alpha blend
        let is_water = matches!(bt, BlockType::Water);
        let material = if is_water {
            StandardMaterial {
                base_color: Color::srgba(c[0], c[1], c[2], 0.65),
                emissive: Color::srgb(0.05, 0.10, 0.18).into(),
                perceptual_roughness: 0.10,
                metallic: 0.30,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }
        } else {
            StandardMaterial {
                base_color: Color::srgba(c[0], c[1], c[2], c[3]),
                emissive: emissive.into(),
                perceptual_roughness: 0.85,
                metallic: 0.0,
                ..default()
            }
        };
        mats.insert(bt, materials.add(material));
    }

    // 共享的 cube mesh
    let cube_mesh: Handle<Mesh> = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // 收集要 spawn 的方块（按 z 排序，远的先画）
    let [px, py, pz] = player.block_pos;
    let r = cfg.radius;
    let s = game_world.size;
    let mut candidates: Vec<(i32, i32, i32, BlockType)> = Vec::new();
    for y in (py - r).max(0)..(py + r + 1).min(s) {
        for z in (pz - r).max(0)..(pz + r + 1).min(s) {
            for x in (px - r).max(0)..(px + r + 1).min(s) {
                let b = game_world.get(x, y, z);
                if b.is_renderable() {  // 包括 Water（半透明但要画）
                    candidates.push((x, y, z, b));
                }
            }
        }
    }
    // 远到近排序：先画远的（z 大的），painter's algorithm
    candidates.sort_by_key(|(_, y, z, _)| -(*y + *z));

    // 限流：warn 至少 1 秒间隔（按真实时间，不按帧——demo 跑 200fps 时 30 帧节流变 150ms 太短）
    if candidates.len() > cfg.max_blocks {
        candidates.truncate(cfg.max_blocks);
        let now = time.elapsed_secs();
        if now - *last_warn_time > 1.0 {
            warn!("体素过多 ({}+), 截断到 {}", candidates.len(), cfg.max_blocks);
            *last_warn_time = now;
        }
    }

    // Spawn
    for (x, y, z, b) in candidates {
        let mat = mats[&b].clone();
        let pos = Vec3::new(
            x as f32 + 0.5,
            y as f32 + 0.5 + cfg.y_offset,
            z as f32 + 0.5,
        );
        let e = commands
            .spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(pos),
            ))
            .id();
        spawned.entities.push(e);
    }

    spawned.last_player_block = player.block_pos;
    info!("🧱 spawn 了 {} 个方块（玩家 {:?}）", spawned.entities.len(), player.block_pos);
}

/// 天空颜色 + 雾 + 武器 spawn（spawn 时把剑挂到相机子节点上，跟着相机走）
pub fn setup_atmosphere(
    mut commands: Commands,
    cfg: Res<RenderConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Query<Entity, With<Camera3d>>,
) {
    // 雾
    // bevy 0.18: Fog 组件挂在 camera 上
    // 0.18 的 fog: use bevy::pbr::FogSettings 或者 scene::Fog
    // 这里用简化版 ClearColor
    commands.insert_resource(ClearColor(cfg.sky_color));

    // ── 武器：剑（handle 棕 + blade 银）— 大尺寸怼屏幕中央，第一眼就能看见 ──
    let handle_mesh = meshes.add(Cuboid::new(0.40, 1.20, 0.40));
    let handle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.28, 0.12),
        perceptual_roughness: 0.6,
        emissive: Color::srgb(0.10, 0.06, 0.02).into(),
        ..default()
    });
    let blade_mesh = meshes.add(Cuboid::new(0.40, 2.00, 0.12));
    let blade_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.94, 1.00),
        perceptual_roughness: 0.20,
        metallic: 0.90,
        emissive: Color::srgb(0.20, 0.22, 0.30).into(),
        ..default()
    });
    // 把剑挂到相机子节点：放在屏幕中央偏下、明显位置
    let Ok(cam_entity) = camera.single() else {
        warn!("setup_atmosphere: 找不到 Camera3d 实体，剑不 spawn");
        return;
    };
    // 把手：相机本地 (0.0, -0.8, -1.5) — 中央、下、前
    let handle = commands.spawn((
        HeldWeaponPart,
        Mesh3d(handle_mesh),
        MeshMaterial3d(handle_mat),
        Transform::from_translation(Vec3::new(0.0, -0.80, -1.50)),
    )).id();
    // 刀刃：叠在把手上方
    let blade = commands.spawn((
        HeldWeaponPart,
        Mesh3d(blade_mesh),
        MeshMaterial3d(blade_mat),
        Transform::from_translation(Vec3::new(0.0, 0.10, -1.50)),
    )).id();
    commands.entity(cam_entity).add_child(handle);
    commands.entity(cam_entity).add_child(blade);
    info!("⚔ 大剑已 spawn 作为 Camera3d 的子节点（屏幕中央）");
}

/// 武器 marker：被 held_weapon_follow 系统认领
#[derive(Component)]
pub struct HeldWeaponPart;

/// 把剑贴在相机右前方，跟随相机 transform
/// 现在剑是相机的子节点，bevy 自动处理 transform 跟随；这个系统留作占位 / 以后做挥剑动画
pub fn held_weapon_follow() {
    // 剑现在是 Camera3d 的子节点，bevy 自动按相机本地坐标渲染
    // 后续如果要加挥剑动画：query 剑的 Transform + 计时器 + 修改 local Y rotation
}

/// 玩家最后移动的方向（用于第一人称相机看向方向）
#[derive(Resource, Default)]
pub struct LastMoveDirection(pub Vec3);

/// 玩家键盘输入：WASD 移动（相对相机方向）/ Space 跳 / Shift 下降 / Q E 转向 / G 采集 / K 杀动物 / F 造国 / J 杀怪 / Esc 退出
pub fn player_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut player: ResMut<PlayerState>,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<crate::resource::GlobalResourcePool>,
    mut last: ResMut<LastMoveDirection>,
    mut nations: ResMut<NationRegistry>,
    mut monsters: ResMut<MonsterEcosystem>,
    camera: Query<&Transform, With<Camera3d>>,
    time: Res<Time>,
) {
    // 读相机当前朝向 → 算 forward / right（水平）
    let cam_tf = camera.single().ok();
    let (forward, right) = if let Some(tf) = cam_tf {
        // bevy 0.18: Transform::forward() 返回 local -Z 在 world 中的方向（相机看哪里）
        let f = tf.forward();
        let f_h = Vec3::new(f.x, 0.0, f.z);
        let f_n = if f_h.length() > 0.01 { f_h.normalize() } else { Vec3::new(1.0, 0.0, 0.0) };
        // right = Y.cross(forward) — +Y 上方系，forward=(1,0,0) → right=(0,0,-1)
        let r = Vec3::Y.cross(f_n);
        (f_n, r)
    } else {
        (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0))
    };

    // 移动：W = +forward, S = -forward, A = -right, D = +right；相对相机方向
    let mut d = Vec3::ZERO;
    if keys.just_pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::ArrowUp)    { d += forward; }
    if keys.just_pressed(KeyCode::KeyS) || keys.just_pressed(KeyCode::ArrowDown)  { d -= forward; }
    if keys.just_pressed(KeyCode::KeyA) || keys.just_pressed(KeyCode::ArrowLeft)  { d -= right; }
    if keys.just_pressed(KeyCode::KeyD) || keys.just_pressed(KeyCode::ArrowRight) { d += right; }
    if keys.just_pressed(KeyCode::Space)    { d += Vec3::Y; }
    if keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight) { d -= Vec3::Y; }

    // 转向：Q 左转 22.5°，E 右转 22.5°（修玩家朝向 + 写入 LastMoveDirection 让相机跟）
    if (keys.just_pressed(KeyCode::KeyQ) || keys.just_pressed(KeyCode::KeyE))
        && cam_tf.is_some()
    {
        let sign: f32 = if keys.just_pressed(KeyCode::KeyQ) { 1.0 } else { -1.0 };
        let yaw = sign * 22.5_f32.to_radians();
        let cos = yaw.cos();
        let sin = yaw.sin();
        // 在 XZ 平面绕 Y 轴旋转 forward 向量
        let old_f = last.0;
        let base = if old_f.length() > 0.01 { old_f } else { forward };
        let new_f = Vec3::new(
            base.x * cos + base.z * sin,
            0.0,
            -base.x * sin + base.z * cos,
        );
        last.0 = new_f.normalize();
    }

    // 把 d 量化成 [i32; 3] 1-格移动（选主轴）
    if d.length() > 0.01 {
        let mut di: [i32; 3] = [0, 0, 0];
        let ad = d.abs();
        if ad.x >= ad.y && ad.x >= ad.z {
            di[0] = d.x.signum() as i32;
        } else if ad.z >= ad.y {
            di[2] = d.z.signum() as i32;
        } else {
            di[1] = d.y.signum() as i32;
        }
        try_player_move(&mut player, &mut game_world, di);
        // 玩家输入不再改 LastMoveDirection（让相机自动转动物 / Q E 改朝向）
    }

    // 采集：G 键 = 挖当前脚下方块
    if keys.just_pressed(KeyCode::KeyG) {
        let (x, y, z) = (player.block_pos[0], player.block_pos[1], player.block_pos[2]);
        let b = game_world.get(x, y, z);
        if b.is_solid() {
            if let Some((res, _)) = b.yields() {
                game_world.set(x, y, z, BlockType::Air);
                let _ = pool.try_add(res, 1);
                *player.inventory.entry(res).or_insert(0) += 1;
                player.blocks_gathered += 1;
                info!("⛏ 你挖了 {:?} (库存 {:?})", res, player.inventory.get(&res).copied().unwrap_or(0));
            } else {
                info!("这个方块挖不出东西");
            }
        } else {
            info!("脚下没方块");
        }
    }

    // 杀动物：K 键（creature 系统自己处理）
    // （不在这写 — 由 creature::player_attack_creatures 系统响应）

    // 造国：F 键 = 在当前坐标立旗
    if keys.just_pressed(KeyCode::KeyF) {
        if player.nation_id.is_some() {
            info!("🚩 你已经是一个国家的王了，不能再立旗");
        } else {
            let tick_now = time.elapsed_secs() as u64;
            let cost = nations.next_flag_cost();
            match nations.found(
                &mut pool,
                0u32, // single-player：玩家固定 id=0
                format!("玩家之国@{:?}", player.block_pos),
                player.block_pos,
                tick_now,
            ) {
                Ok(id) => {
                    player.nation_id = Some(id);
                    player.nations_founded += 1;
                    info!("🚩 你建立了国家 {:?}（消耗 {} 灵魂）", id, cost);
                }
                Err(e) => {
                    info!("🚩 造国失败：{:?}", e);
                }
            }
        }
    }

    // 杀怪：J 键 = 攻击 2 格内最近怪物个体
    if keys.just_pressed(KeyCode::KeyJ) {
        let p = player.block_pos;
        let mut best: Option<(f32, u32, u32, u32)> = None; // (dist, kid, nid, iid)
        for (kid, k) in monsters.kingdoms.iter() {
            for (nid, n) in k.nests.iter() {
                for (iid, ind) in n.individuals.iter() {
                    let dx = (ind.position[0] - p[0]) as f32;
                    let dy = (ind.position[1] - p[1]) as f32;
                    let dz = (ind.position[2] - p[2]) as f32;
                    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                    if dist <= 2.0 {
                        match best {
                            Some((bd, _, _, _)) if dist >= bd => {}
                            _ => best = Some((dist, *kid, *nid, *iid)),
                        }
                    }
                }
            }
        }
        if let Some((dist, kid, nid, iid)) = best {
            if monsters.kill_individual(kid, nid, iid, &mut pool) {
                player.monsters_killed += 1;
                info!("⚔ 你击杀了怪物 (距离 {:.1} 格) kid={} nid={} iid={}", dist, kid, nid, iid);
            }
        } else {
            info!("⚔ 2 格内没有怪物");
        }
    }
}

/// 玩家移动：3D cardinal 方向。如果目标块是实心，向上找空位（最多 6 格）。
fn try_player_move(player: &mut PlayerState, game_world: &mut GameWorld, d: [i32; 3]) {
    let mut new_pos = [
        player.block_pos[0] + d[0],
        player.block_pos[1] + d[1],
        player.block_pos[2] + d[2],
    ];
    if !game_world.in_bounds(new_pos[0], new_pos[1], new_pos[2]) {
        return;
    }
    if new_pos[1] < 0 || new_pos[1] >= game_world.size as i32 {
        return;
    }
    let b = game_world.get(new_pos[0], new_pos[1], new_pos[2]);
    if b.is_solid() {
        // 向上找空位（最多 6 格）
        let mut landed = false;
        for up in 1..=6 {
            let try_pos = [new_pos[0], new_pos[1] + up, new_pos[2]];
            if game_world.in_bounds(try_pos[0], try_pos[1], try_pos[2])
                && !game_world.get(try_pos[0], try_pos[1], try_pos[2]).is_solid()
            {
                new_pos = try_pos;
                landed = true;
                break;
            }
        }
        if !landed {
            return; // 被挡死
        }
    }
    player.block_pos = new_pos;
    player.pos = Vec3::new(
        new_pos[0] as f32 + 0.5,
        new_pos[1] as f32 + 0.5,
        new_pos[2] as f32 + 0.5,
    );
}

// ---------------------------------------------------------------------------
// 相机：auto_orbit 时绕玩家慢转；玩家控制时停在固定俯瞰角跟随玩家
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct PlayerState {
    pub pos: Vec3,
    pub block_pos: [i32; 3],
    pub inventory: std::collections::HashMap<ResourceKind, i64>,
    pub nation_id: Option<crate::nation::NationId>,
    pub monsters_killed: u32,
    pub blocks_gathered: u32,
    pub nations_founded: u32,
}

/// 自动 demo 模式：每 N 秒随机移动玩家，让相机跟着转
/// Demo 模式：玩家可以"飞" — 如果被固体挡住，自动向上找空位
pub fn auto_demo(
    time: Res<Time>,
    mut player: ResMut<PlayerState>,
    mut game_world: ResMut<GameWorld>,
    cfg: Res<RenderConfig>,
    keys: Res<ButtonInput<KeyCode>>,
    mut last: ResMut<LastMoveDirection>,
    mut walk_timer: Local<f32>,
    mut walk_step: Local<u32>,
) {
    if !cfg.auto_walk {
        return;
    }
    // 玩家按了任何移动键 → 让位给真实输入
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::KeyA)
        || keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::KeyD)
        || keys.pressed(KeyCode::Space) || keys.pressed(KeyCode::ShiftLeft)
    {
        return;
    }
    *walk_timer += time.delta_secs();
    if *walk_timer < cfg.auto_walk_interval_secs {
        return;
    }
    *walk_timer = 0.0;
    *walk_step += 1;

    // 8 水平方向 + 偶尔 Y 方向，但只挑前方 2 格都空的方向（避免第一视角撞墙）
    let all_dirs: [[i32; 3]; 9] = [
        [1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1],
        [1, 0, 1], [1, 0, -1], [-1, 0, 1], [-1, 0, -1],
        [0, 1, 0],
    ];
    // 过滤：前方 2 格都空（避免被挡住后第一视角贴着墙看）
    let good_dirs: Vec<[i32; 3]> = all_dirs.iter().filter(|d| {
        let np1 = [player.block_pos[0] + d[0], player.block_pos[1] + d[1], player.block_pos[2] + d[2]];
        let np2 = [np1[0] + d[0], np1[1] + d[1], np1[2] + d[2]];
        if !game_world.in_bounds(np1[0], np1[1], np1[2]) { return false; }
        if !game_world.in_bounds(np2[0], np2[1], np2[2]) { return false; }
        if game_world.get(np1[0], np1[1], np1[2]).is_solid() { return false; }
        if game_world.get(np2[0], np2[1], np2[2]).is_solid() { return false; }
        true
    }).copied().collect();
    if good_dirs.is_empty() { return; }
    let d = good_dirs[(*walk_step as usize) % good_dirs.len()];

    // demo 模式：被挡住时向上飞过（最高 +6 格）。新地形很高，不强行压回平地
    let mut new_pos = [
        player.block_pos[0] + d[0],
        player.block_pos[1] + d[1],
        player.block_pos[2] + d[2],
    ];
    if !game_world.in_bounds(new_pos[0], new_pos[1], new_pos[2]) {
        return;
    }
    let b = game_world.get(new_pos[0], new_pos[1], new_pos[2]);
    if b.is_solid() {
        // 向上找空位（最多 6 格）
        for up in 1..=6 {
            let try_pos = [new_pos[0], new_pos[1] + up, new_pos[2]];
            if game_world.in_bounds(try_pos[0], try_pos[1], try_pos[2])
                && !game_world.get(try_pos[0], try_pos[1], try_pos[2]).is_solid()
            {
                new_pos = try_pos;
                break;
            }
        }
    }
    if !game_world.get(new_pos[0], new_pos[1], new_pos[2]).is_solid() {
        let before = player.block_pos;
        player.block_pos = new_pos;
        player.pos = Vec3::new(
            new_pos[0] as f32 + 0.5,
            new_pos[1] as f32 + 0.5,
            new_pos[2] as f32 + 0.5,
        );
        // 记下最后水平移动方向
        let dx = (player.block_pos[0] - before[0]) as f32;
        let dz = (player.block_pos[2] - before[2]) as f32;
        if dx != 0.0 || dz != 0.0 {
            let v = Vec3::new(dx, 0.0, dz);
            last.0 = v.normalize();
        }
    }
    // 周期性地采集当前块（如果可采集）
    let cur = game_world.get(player.block_pos[0], player.block_pos[1], player.block_pos[2]);
    if let Some((res, _)) = cur.yields() {
        if cur.is_solid() {
            game_world.set(player.block_pos[0], player.block_pos[1], player.block_pos[2], BlockType::Air);
            *player.inventory.entry(res).or_insert(0) += 1;
            player.blocks_gathered += 1;
        }
    }
}

/// 第一人称相机：把相机放到玩家头部位置，朝玩家最近一次移动的方向看（略向下倾）
/// 强制第一帧对准最近的动物（保证玩家起手就能看见动物）
pub fn first_person_camera(
    mut q: Query<&mut Transform, With<Camera3d>>,
    player: Res<PlayerState>,
    last: Res<LastMoveDirection>,
    creatures: Query<&Creature>,
    world: Res<GameWorld>,
    mut debug_count: Local<u32>,
) {
    let Ok(mut tf) = q.single_mut() else { return; };
    let eye = Vec3::new(
        player.block_pos[0] as f32 + 0.5,
        player.block_pos[1] as f32 + 0.5 + 1.3,
        player.block_pos[2] as f32 + 0.5,
    );
    // 找最近的、未被墙挡住的动物，30 格内
    // 简化：只挑前 3 个最近的，逐一检查视线（中间 5 格都空就行）
    let mut candidates: Vec<(f32, [i32; 3])> = Vec::new();
    for c in creatures.iter() {
        let dx = (c.block_pos[0] as f32 + 0.5) - eye.x;
        let dz = (c.block_pos[2] as f32 + 0.5) - eye.z;
        let d2 = dx * dx + dz * dz;
        if d2 < 900.0 {
            candidates.push((d2, c.block_pos));
        }
    }
    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut best: Option<[i32; 3]> = None;
    for (_d2, c) in candidates.iter().take(5) {
        // 检查视线：眼睛到目标，沿途 5 格每格必须空
        let tx = c[0] as f32 + 0.5;
        let tz = c[2] as f32 + 0.5;
        let steps = 5;
        let mut clear = true;
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let x = eye.x + (tx - eye.x) * t;
            let z = eye.z + (tz - eye.z) * t;
            let bx = x.floor() as i32;
            let bz = z.floor() as i32;
            if world.in_bounds(bx, player.block_pos[1], bz)
                && world.get(bx, player.block_pos[1], bz).is_solid()
            {
                clear = false;
                break;
            }
        }
        if clear {
            best = Some(*c);
            break;
        }
    }
    if *debug_count < 3 {
        *debug_count += 1;
        info!(
            "🎥 第一人称: 玩家={:?} 候选={} 选中={:?}",
            player.block_pos,
            candidates.len(),
            best
        );
    }
    let dir = if let Some(c) = best {
        let v = Vec3::new((c[0] as f32 + 0.5) - eye.x, 0.0, (c[2] as f32 + 0.5) - eye.z);
        if v.length() > 0.01 { v.normalize() } else { Vec3::new(1.0, 0.0, 0.0) }
    } else if last.0.length() < 0.01 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        last.0.normalize()
    };
    let look_target = eye + dir * 5.0 - Vec3::new(0.0, 1.0, 0.0);
    tf.translation = eye;
    tf.look_at(look_target, Vec3::Y);
}
