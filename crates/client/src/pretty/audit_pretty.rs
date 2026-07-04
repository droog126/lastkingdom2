use bevy::prelude::*;
use lk2_core::player::PlayerState;
use lk2_core::world::World as GameWorld;

use crate::render::scalar_field::effective_ground_height;

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
    ("mountain_snow", "procedural/pretty/mountain_snow.glb"),
    ("volcano", "procedural/pretty/volcano.glb"),
    ("desert_dune", "procedural/pretty/desert_dune.glb"),
    ("lake", "procedural/pretty/lake.glb"),
    ("swamp", "procedural/pretty/swamp.glb"),
    ("cliff", "procedural/pretty/cliff.glb"),
    ("cave_entrance", "procedural/pretty/cave_entrance.glb"),
    ("beach", "procedural/pretty/beach.glb"),
    ("house_small", "procedural/pretty/house_small.glb"),
    ("watchtower", "procedural/pretty/watchtower.glb"),
    ("windmill", "procedural/pretty/windmill.glb"),
    ("bridge_stone", "procedural/pretty/bridge_stone.glb"),
    ("well", "procedural/pretty/well.glb"),
    ("barn", "procedural/pretty/barn.glb"),
    ("fence", "procedural/pretty/fence.glb"),
    ("shrine", "procedural/pretty/shrine.glb"),
    ("lighthouse", "procedural/pretty/lighthouse.glb"),
];

const KENNEY_RING_PATHS: &[(&str, &str, f32)] = &[
    (
        "kenney_bunny",
        "kenney/curated/animals/kenney_bunny.glb",
        1.0,
    ),
    ("kenney_deer", "kenney/curated/animals/kenney_deer.glb", 1.0),
    ("kenney_cow", "kenney/curated/animals/kenney_cow.glb", 1.0),
    (
        "kenney_villager_male_a",
        "kenney/curated/characters/kenney_villager_male_a.glb",
        1.0,
    ),
    (
        "kenney_villager_female_a",
        "kenney/curated/characters/kenney_villager_female_a.glb",
        1.0,
    ),
    (
        "kenney_campfire_pit",
        "kenney/curated/survival_props/kenney_campfire_pit.glb",
        1.0,
    ),
    (
        "kenney_tent",
        "kenney/curated/survival_props/kenney_tent.glb",
        1.0,
    ),
    (
        "kenney_workbench",
        "kenney/curated/survival_props/kenney_workbench.glb",
        1.0,
    ),
    (
        "kenney_row_boat_small",
        "kenney/curated/coastal_and_pirate/kenney_row_boat_small.glb",
        1.0,
    ),
    (
        "kenney_pirate_flag",
        "kenney/curated/coastal_and_pirate/kenney_pirate_flag.glb",
        1.0,
    ),
    (
        "kenney_coin_gold",
        "kenney/curated/terrain_and_pickups/kenney_coin_gold.glb",
        1.0,
    ),
    (
        "kenney_heart",
        "kenney/curated/terrain_and_pickups/kenney_heart.glb",
        1.0,
    ),
];

#[derive(Component)]
pub struct AuditPrettyMarker;

#[derive(Component)]
pub struct AuditRingOffset {
    pub rel: Vec3,
}

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
    let terrain_names = [
        "mountain_snow",
        "volcano",
        "desert_dune",
        "lake",
        "swamp",
        "cliff",
        "cave_entrance",
        "beach",
    ];
    let buildings_names = [
        "house_small",
        "watchtower",
        "windmill",
        "bridge_stone",
        "well",
        "barn",
        "fence",
        "shrine",
        "lighthouse",
    ];

    info!(
        "[audit-pretty-models] spawning audit ring ({} procedural, {} kenney assets)",
        RING_PATHS.len(),
        KENNEY_RING_PATHS.len()
    );

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
    spawn_ring(
        commands,
        &asset_server,
        &terrain_names,
        player_pos,
        ground_y,
        9.0,
        0.0,
    );
    spawn_ring(
        commands,
        &asset_server,
        &buildings_names,
        player_pos,
        ground_y,
        15.0,
        0.0,
    );
    spawn_kenney_ring(commands, &asset_server, player_pos, ground_y);

    spawn_single(
        commands,
        &asset_server,
        "cloud_puff",
        player_pos + Vec3::new(-30.0, 6.0, 0.0),
        player_pos,
        ground_y,
    );
    spawn_single(
        commands,
        &asset_server,
        "cloud_puff",
        player_pos + Vec3::new(30.0, 8.0, 0.0),
        player_pos,
        ground_y,
    );
}

fn spawn_kenney_ring(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    player_pos: Vec3,
    ground_y: f32,
) {
    let radius = 20.0;
    let n = KENNEY_RING_PATHS.len();
    for (i, (name, path, scale)) in KENNEY_RING_PATHS.iter().enumerate() {
        let angle = (i as f32) / (n as f32) * std::f32::consts::TAU;
        let x = player_pos.x + angle.cos() * radius;
        let z = player_pos.z + angle.sin() * radius;
        let world_y = ground_y - 0.5;
        let rel = Vec3::new(x - player_pos.x, world_y - ground_y, z - player_pos.z);
        let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", path));
        commands.spawn((
            SceneRoot(scene),
            Transform::from_translation(Vec3::new(x, world_y, z)).with_scale(Vec3::splat(*scale)),
            AuditPrettyMarker,
            AuditRingOffset { rel },
        ));
        info!(
            "[audit-pretty-models]   kenney [{}] {} at ({:.1}, {:.1})",
            i, name, x, z
        );
    }
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
        let world_y = ground_y + y_offset - 0.5;
        let rel = Vec3::new(x - player_pos.x, world_y - ground_y, z - player_pos.z);
        let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", path));
        commands.spawn((
            SceneRoot(scene),
            Transform::from_translation(Vec3::new(x, world_y, z)),
            AuditPrettyMarker,
            AuditRingOffset { rel },
        ));
        info!(
            "[audit-pretty-models]   [{}] {} at ({:.1}, {:.1})",
            i, name, x, z
        );
    }
}

fn spawn_single(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    name: &str,
    pos: Vec3,
    origin_player: Vec3,
    origin_ground: f32,
) {
    let path = match RING_PATHS.iter().find(|(n, _)| *n == name) {
        Some((_, p)) => *p,
        None => {
            warn!("[audit-pretty-models] missing in RING_PATHS: {}", name);
            return;
        }
    };
    let rel = Vec3::new(
        pos.x - origin_player.x,
        pos.y - origin_ground - 0.5,
        pos.z - origin_player.z,
    );
    let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", path));
    commands.spawn((
        SceneRoot(scene),
        Transform::from_translation(pos),
        AuditPrettyMarker,
        AuditRingOffset { rel },
    ));
    info!("[audit-pretty-models]   single {} at {:?}", name, pos);
}

pub fn follow_audit_ring(
    player: Res<PlayerState>,
    game_world: Res<GameWorld>,
    mut q: Query<(&mut Transform, &AuditRingOffset)>,
) {
    for (mut t, off) in &mut q {
        let ix = t.translation.x as i32;
        let iz = t.translation.z as i32;
        let ground_top = effective_ground_height(&game_world, ix, iz);
        t.translation.x = player.pos.x + off.rel.x;
        t.translation.z = player.pos.z + off.rel.z;
        t.translation.y = ground_top + off.rel.y;
    }
}
