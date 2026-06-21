//! [dev-only] pretty/ 资产可视化审计.
//!
//! 仅当 crate feature `audit-pretty-models` 开启时编译. 把所有 23 个
//! assets/procedural/pretty/*.glb 摆成两圈, 用来在不动原 pretty/mod.rs
//! 代码的前提下, 验证 Blender 资产在 Bevy 里能正常渲染 + 看大致外观.
//!
//! 跑法: cargo run -p lk2-client --features audit-pretty-models -- --offline

use bevy::prelude::*;
use lk2_core::world::World as GameWorld;

/// 一圈 23 个 .glb — 顺序与 tools/build_all_models.py 输出一致.
const RING_PATHS: &[(&str, &str)] = &[
    ("player_avatar", "procedural/pretty/player_avatar.glb"),
    ("monster_snake", "procedural/pretty/monster_snake.glb"),
    (
        "monster_frost_elf",
        "procedural/pretty/monster_frost_elf.glb",
    ),
    (
        "monster_sand_wurm",
        "procedural/pretty/monster_sand_wurm.glb",
    ),
    ("monster_treant", "procedural/pretty/monster_treant.glb"),
    (
        "monster_aether_wraith",
        "procedural/pretty/monster_aether_wraith.glb",
    ),
    ("cloud_puff", "procedural/pretty/cloud_puff.glb"),
    ("tree", "procedural/pretty/tree.glb"),
    ("rock_dark", "procedural/pretty/rock_dark.glb"),
    ("rock_mid", "procedural/pretty/rock_mid.glb"),
    ("rock_moss", "procedural/pretty/rock_moss.glb"),
    ("flower_0_pink", "procedural/pretty/flower_0.glb"),
    ("flower_1_yellow", "procedural/pretty/flower_1.glb"),
    ("flower_2_purple", "procedural/pretty/flower_2.glb"),
    ("flower_3_orange", "procedural/pretty/flower_3.glb"),
    ("flower_4_red", "procedural/pretty/flower_4.glb"),
    ("hill", "procedural/pretty/hill.glb"),
    ("poi_pillar_red", "procedural/pretty/poi_pillar_red.glb"),
    ("poi_pillar_cyan", "procedural/pretty/poi_pillar_cyan.glb"),
    ("poi_pillar_pink", "procedural/pretty/poi_pillar_pink.glb"),
    ("poi_pillar_gold", "procedural/pretty/poi_pillar_gold.glb"),
    (
        "ground_disc_outer",
        "procedural/pretty/ground_disc_outer.glb",
    ),
    (
        "ground_disc_inner",
        "procedural/pretty/ground_disc_inner.glb",
    ),
];

#[derive(Component)]
pub struct AuditPrettyMarker;

/// 在 player 周围排成 2 圈: 内圈 r=8m (玩家 avatar / 怪物), 外圈 r=16m
/// (环境资产). 高度按"脚下"基准 — 大部分模型 y=0 是脚, 直接放 ground_y.
pub fn spawn_audit_ring(
    commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    player_pos: Vec3,
    ground_y: f32,
) {
    let inner_names = [
        "player_avatar",
        "monster_snake",
        "monster_frost_elf",
        "monster_sand_wurm",
        "monster_treant",
        "monster_aether_wraith",
        "cloud_puff",
        "ground_disc_inner",
    ];
    let outer_names = [
        "tree",
        "rock_dark",
        "rock_mid",
        "rock_moss",
        "flower_0_pink",
        "flower_1_yellow",
        "flower_2_purple",
        "flower_3_orange",
        "flower_4_red",
        "hill",
        "poi_pillar_red",
        "poi_pillar_cyan",
        "poi_pillar_pink",
        "poi_pillar_gold",
        "ground_disc_outer",
    ];

    info!(
        "[audit-pretty-models] spawning audit ring ({} assets)",
        RING_PATHS.len()
    );

    // 内圈 r=5m y=+0m (脚底贴地, 玩家 avatar 锚点 = 脚底)
    // 外圈 r=12m y=+0m (环境资产贴地, 大部分锚点 = 脚底)
    // 云朵单独抬到 y=+6m 高空, 跟玩家错开
    spawn_ring(
        commands,
        &asset_server,
        &inner_names,
        player_pos,
        ground_y,
        5.0,
        0.0,
    );
    spawn_ring(
        commands,
        &asset_server,
        &outer_names,
        player_pos,
        ground_y,
        12.0,
        0.0,
    );
    // 云朵 (边缘, 远离玩家, 不要挡视线)
    spawn_single(
        commands,
        &asset_server,
        "cloud_puff",
        player_pos + Vec3::new(-30.0, 6.0, 0.0),
    );
    spawn_single(
        commands,
        &asset_server,
        "cloud_puff",
        player_pos + Vec3::new(30.0, 8.0, 0.0),
    );
}

fn spawn_ring(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    names: &[&str],
    player_pos: Vec3,
    ground_y: f32,
    radius: f32,
    y_offset: f32,
) {
    let n = names.len();
    for (i, name) in names.iter().enumerate() {
        let path = match RING_PATHS.iter().find(|(n, _)| n == name) {
            Some((_, p)) => *p,
            None => {
                warn!("[audit-pretty-models] missing in RING_PATHS: {}", name);
                continue;
            }
        };
        let angle = (i as f32) / (n as f32) * std::f32::consts::TAU;
        let x = player_pos.x + angle.cos() * radius;
        let z = player_pos.z + angle.sin() * radius;
        let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", path));
        commands.spawn((
            SceneRoot(scene),
            Transform::from_translation(Vec3::new(x, ground_y + y_offset, z)),
            AuditPrettyMarker,
        ));
        info!(
            "[audit-pretty-models]   [{}] {} at ({:.1}, {:.1})",
            i, name, x, z
        );
    }
}

fn spawn_single(commands: &mut Commands, asset_server: &Res<AssetServer>, name: &str, pos: Vec3) {
    let path = match RING_PATHS.iter().find(|(n, _)| *n == name) {
        Some((_, p)) => *p,
        None => {
            warn!("[audit-pretty-models] missing in RING_PATHS: {}", name);
            return;
        }
    };
    let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", path));
    commands.spawn((
        SceneRoot(scene),
        Transform::from_translation(pos),
        AuditPrettyMarker,
    ));
    info!("[audit-pretty-models]   single {} at {:?}", name, pos);
}
