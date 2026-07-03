use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::collections::{HashMap, HashSet};

use avian3d::prelude::{Collider, RigidBody};

use crate::pretty::PlayerAnimState;
use lk2_core::constant;
use lk2_core::creature::Creature;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::resource::ResourceKind;
use lk2_core::world::{Biome, BlockType, World as GameWorld};

mod greedy_mesh;
use greedy_mesh::build_all_terrain_meshes_aabb;

mod marching_cubes;
pub mod scalar_field;
mod smooth_mesh;

#[derive(Resource, Debug, Clone)]
pub struct RenderConfig {
    pub radius: i32,
    pub max_blocks: usize,
    pub y_offset: f32,
    pub sky_color: Color,
    pub fog_color: Color,
    pub fog_start: f32,
    pub fog_end: f32,
    pub auto_orbit: bool,
    pub auto_orbit_speed: f32,
    pub auto_orbit_distance: f32,
    pub auto_walk: bool,
    pub auto_walk_interval_secs: f32,
    pub auto_keys: bool,
    pub mouse_look: bool,
    pub smooth_terrain: bool,
    pub smooth_passes: u32,
    pub ground_step_threshold: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            radius: 48,
            max_blocks: 5000,
            y_offset: 0.0,
            sky_color: Color::srgb(0.45, 0.65, 0.95),
            fog_color: Color::srgb(0.78, 0.85, 0.95),
            fog_start: 130.0,
            fog_end: 360.0,
            auto_orbit: true,

            auto_orbit_speed: 0.30,

            auto_orbit_distance: 22.0,
            auto_walk: false,
            auto_walk_interval_secs: 0.1,
            auto_keys: false,
            mouse_look: false,

            smooth_terrain: true,
            smooth_passes: 4,
            ground_step_threshold: 0.85,
        }
    }
}

#[derive(Resource)]
pub struct CameraAngles {
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for CameraAngles {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: -1.2 }
    }
}

#[derive(Resource, PartialEq, Eq, Debug, Clone, Copy)]
pub enum CameraMode {
    FirstPerson,

    ThirdPerson,
}

impl Default for CameraMode {
    fn default() -> Self {
        Self::ThirdPerson
    }
}

const TP_DISTANCE: f32 = 6.0;
const TP_HEIGHT: f32 = 4.0;
const MANUAL_MOVE_SPEED: f32 = 4.5;

#[derive(Resource)]
pub struct FreeFlyState {
    pub enabled: bool,

    pub position: Vec3,

    pub velocity: Vec3,

    pub saved_player_pos: Option<[i32; 3]>,

    pub saved_player_world_pos: Option<Vec3>,
}

impl Default for FreeFlyState {
    fn default() -> Self {
        Self {
            enabled: false,
            position: Vec3::new(48.5, 18.0, 48.5),
            velocity: Vec3::ZERO,
            saved_player_pos: None,
            saved_player_world_pos: None,
        }
    }
}

const FREEFLY_SPEED: f32 = 30.0;

const FREEFLY_BOOST: f32 = 3.0;

const MOUSE_SENS: f32 = 0.0022;
const PITCH_LIMIT: f32 = 1.483;
const YAW_QE_STEP: f32 = 22.5_f32.to_radians();

#[derive(Resource, Default)]
pub struct SpawnedBlocks {
    pub visual_entities: Vec<Entity>,
    pub collider_entities: Vec<Entity>,

    pub last_player_block: [i32; 3],
    pub last_mesh_center: Option<Vec3>,
}

#[derive(Component)]
pub struct PlayerCube;

pub fn spawn_terrain_around_player(
    mut commands: Commands,
    game_world: Res<GameWorld>,
    cfg: Res<RenderConfig>,
    player: Res<PlayerState>,
    mut spawned: ResMut<SpawnedBlocks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,

    _last_warn_time: Local<f32>,

    mut last_mesh_wall: Local<f32>,
) {
    let now = time.elapsed_secs();
    let moved = spawned.last_player_block != player.block_pos;
    let moved_far = if let Some(last) = spawned.last_mesh_center {
        last.distance(Vec3::new(
            player.block_pos[0] as f32,
            player.block_pos[1] as f32,
            player.block_pos[2] as f32,
        )) > cfg.ground_step_threshold * 6.0
    } else {
        true
    };
    if !moved && !spawned.visual_entities.is_empty() {
        return;
    }
    if moved_far && now - *last_mesh_wall < 1.5 && !spawned.visual_entities.is_empty() {
        return;
    }

    for e in spawned.visual_entities.drain(..) {
        commands.entity(e).despawn();
    }
    for e in spawned.collider_entities.drain(..) {
        commands.entity(e).despawn();
    }

    let r = cfg.radius as i32;
    let py = player.block_pos[1];
    let y_min = (py - 40).max(0);
    let y_max = (py + 40).min(game_world.size as i32 - 1);
    let min = [player.block_pos[0] - r, y_min, player.block_pos[2] - r];
    let max = [player.block_pos[0] + r, y_max, player.block_pos[2] + r];

    if cfg.smooth_terrain {
        let started = time.elapsed_secs();
        let sm = smooth_mesh::build_smooth_mesh(&game_world, min, max, 0.5, cfg.smooth_passes);
        if let Some(sm) = sm {
            let total_tris = sm.collider_indices.len() / 3;

            let mat = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::srgb(0.04, 0.05, 0.03).into(),
                unlit: false,
                perceptual_roughness: 0.95,
                metallic: 0.0,
                cull_mode: Some(bevy::render::render_resource::Face::Back),
                double_sided: false,
                ..default()
            });
            let mesh_handle = meshes.add(sm.mesh.clone());
            let visual = commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(mat),
                    Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                    TerrainChunk,
                ))
                .id();
            spawned.visual_entities.push(visual);

            let collider_verts: Vec<Vec3> =
                sm.collider_trimesh.iter().map(|p| Vec3::new(p[0], p[1], p[2])).collect();
            let collider_indices: Vec<[u32; 3]> = sm
                .collider_indices
                .chunks(3)
                .filter(|c| c.len() == 3)
                .map(|c| [c[0], c[1], c[2]])
                .collect();
            let collider = Collider::trimesh(collider_verts, collider_indices);
            let collider_ent = commands
                .spawn((
                    RigidBody::Static,
                    collider,
                    Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                    TerrainChunk,
                ))
                .id();
            spawned.collider_entities.push(collider_ent);

            spawned.last_player_block = player.block_pos;
            spawned.last_mesh_center = Some(Vec3::new(
                player.block_pos[0] as f32,
                player.block_pos[1] as f32,
                player.block_pos[2] as f32,
            ));
            let mesh_secs = time.elapsed_secs() - started;
            info!(
                "🌊 smooth mesh: {} tris, passes={}, 耗时 {:.0}ms（玩家 {:?}）",
                total_tris,
                cfg.smooth_passes,
                mesh_secs * 1000.0,
                player.block_pos
            );
        } else {
            debug!("🌊 smooth mesh: 标量场全空（无 solid 在 AABB 内）");
            spawned.last_player_block = player.block_pos;
        }
        *last_mesh_wall = time.elapsed_secs();
        return;
    }

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

    let started = time.elapsed_secs();
    let block_meshes = build_all_terrain_meshes_aabb(&game_world, min, max);
    let mesh_count = block_meshes.len();
    let total_tris: usize = block_meshes.iter().map(|m| m.indices.len() / 3).sum();
    let mesh_secs = time.elapsed_secs() - started;

    for bm in block_meshes {
        let mat = mats[&bm.block_type].clone();
        let bevy_mesh = bm.to_bevy_mesh();

        let collider_opt = if matches!(bm.block_type, BlockType::Water) {
            None
        } else {
            Collider::trimesh_from_mesh(&bevy_mesh)
        };

        let mesh_handle = meshes.add(bevy_mesh);

        let visual = commands
            .spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat),
                Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                TerrainChunk,
            ))
            .id();
        spawned.visual_entities.push(visual);

        if let Some(collider) = collider_opt {
            let collider_ent = commands
                .spawn((
                    RigidBody::Static,
                    collider,
                    Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                    TerrainChunk,
                ))
                .id();
            spawned.collider_entities.push(collider_ent);
        }
    }

    spawned.last_player_block = player.block_pos;
    debug!(
        "🧱 greedy mesh: {} type(s), {} tris, 耗时 {:.0}ms（玩家 {:?}）",
        mesh_count,
        total_tris,
        mesh_secs * 1000.0,
        player.block_pos
    );
    *last_mesh_wall = time.elapsed_secs();
}

#[derive(Component)]
pub struct TerrainChunk;

pub fn setup_atmosphere(
    mut commands: Commands,
    cfg: Res<RenderConfig>,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    camera: Query<Entity, With<Camera3d>>,
) {
    use bevy::pbr::DistanceFog;
    commands.insert_resource(ClearColor(Color::srgb(0.20, 0.45, 0.78)));

    if let Ok(cam_entity) = camera.single() {
        commands.entity(cam_entity).insert(DistanceFog {
            color: cfg.fog_color,
            directional_light_color: cfg.fog_color,
            directional_light_exponent: 2.0,
            falloff: bevy::pbr::FogFalloff::Linear { start: cfg.fog_start, end: cfg.fog_end },
        });
    }
}

#[derive(Component)]
pub struct TerrainUnderlay;

pub fn setup_terrain_underlay(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let plane_mesh = meshes.add(Plane3d::default().mesh().size(100.0, 100.0));

    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.30, 0.26),
        emissive: Color::srgb(0.08, 0.07, 0.05).into(),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(plane_mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(Vec3::new(0.0, -100.0, 0.0)),
        TerrainUnderlay,
    ));
    info!("🟫 兜底盖板 100x100 plane 已 spawn（player 脚下 -5m 跟随）");
}

pub fn underlay_follow_player(
    mut q: Query<&mut Transform, With<TerrainUnderlay>>,
    player: Res<PlayerState>,
) {
    let Ok(mut tf) = q.single_mut() else {
        return;
    };
    tf.translation = Vec3::new(player.pos.x, -100.0, player.pos.z);
}

#[derive(Component)]
pub struct HeldWeaponPart;

#[derive(Resource, Default)]
pub struct SwordSwing {
    pub start_t: f32,
    pub swinging: bool,
}

const SWING_DURATION: f32 = 0.18;

pub fn held_weapon_follow(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut swing: ResMut<SwordSwing>,
    mut q: Query<&mut Transform, With<HeldWeaponPart>>,
) {
    if keys.just_pressed(KeyCode::KeyK) && !swing.swinging {
        swing.swinging = true;
        swing.start_t = time.elapsed_secs();
    }

    if !swing.swinging {
        return;
    }

    let elapsed = time.elapsed_secs() - swing.start_t;
    if elapsed >= SWING_DURATION {
        swing.swinging = false;

        for mut tf in &mut q {
            tf.rotation = Quat::from_rotation_z(15_f32.to_radians());
        }
        return;
    }
    let t = elapsed / SWING_DURATION;

    let swing_phase = (t * std::f32::consts::PI).sin();

    let total_pitch = 15_f32.to_radians() + swing_phase * 90_f32.to_radians();
    for mut tf in &mut q {
        tf.rotation = Quat::from_euler(EulerRot::XYZ, total_pitch, 0.0, 15_f32.to_radians());
    }
}

pub fn mouse_look_system(
    motion: Res<AccumulatedMouseMotion>,
    mut angles: ResMut<CameraAngles>,
    cfg: Res<RenderConfig>,
    freefly: Res<FreeFlyState>,
) {
    if !cfg.mouse_look && !freefly.enabled {
        return;
    }
    if motion.delta == Vec2::ZERO {
        return;
    }

    angles.yaw += motion.delta.x * MOUSE_SENS;
    angles.pitch -= motion.delta.y * MOUSE_SENS;
    angles.pitch = angles.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
}

pub fn freefly_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut freefly: ResMut<FreeFlyState>,
    mut player: ResMut<PlayerState>,
) {
    if !keys.just_pressed(KeyCode::F3) {
        return;
    }
    freefly.enabled = !freefly.enabled;
    if freefly.enabled {
        freefly.saved_player_pos = Some(player.block_pos);
        freefly.saved_player_world_pos = Some(player.pos);
        freefly.position = player.pos + Vec3::Y * 18.0;
        freefly.velocity = Vec3::ZERO;
        info!(
            "🕊 FreeFly ON — 玩家身体冻结在 {:?}，WASD 飞 / Space↑ / Shift↓ / 鼠标视角 / F3 回本体",
            player.block_pos
        );
    } else {
        if let Some(saved) = freefly.saved_player_pos.take() {
            let saved_pos = freefly.saved_player_world_pos.take().unwrap_or(Vec3::new(
                saved[0] as f32 + 0.5,
                saved[1] as f32,
                saved[2] as f32 + 0.5,
            ));
            info!(
                "🕊 FreeFly OFF — 还原玩家 {:?} -> {:?}",
                player.block_pos, saved
            );
            set_player_position(&mut player, saved_pos, saved);
        } else {
            info!("🕊 FreeFly OFF");
        }
        freefly.velocity = Vec3::ZERO;
    }
}

pub fn camera_mode_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<CameraMode>,
    freefly: Res<FreeFlyState>,
) {
    if freefly.enabled {
        return;
    }
    if !keys.just_pressed(KeyCode::KeyC) {
        return;
    }
    *mode = match *mode {
        CameraMode::FirstPerson => {
            info!("📷 CameraMode → 3rd person");
            CameraMode::ThirdPerson
        }
        CameraMode::ThirdPerson => {
            info!("📷 CameraMode → 1st person");
            CameraMode::FirstPerson
        }
    };
}

pub fn emergency_teleport(
    keys: Res<ButtonInput<KeyCode>>,
    game_world: Res<GameWorld>,
    mut player: ResMut<PlayerState>,
) {
    if !keys.just_pressed(KeyCode::F5) {
        return;
    }
    let x = lk2_core::constant::WORLD_SIZE / 2;
    let z = lk2_core::constant::WORLD_SIZE / 2;
    let Some((pos, block_pos)) = player_spawn_position_at(&game_world, x, z) else {
        warn!("🚨 F5 紧急传送失败：出生列没有可站位置");
        return;
    };
    warn!("🚨 F5 紧急传送： {:?} -> {:?}", player.block_pos, block_pos);
    set_player_position(&mut player, pos, block_pos);
}

pub fn cycle_terrain_preset(keys: Res<ButtonInput<KeyCode>>, mut game_world: ResMut<GameWorld>) {
    if !keys.just_pressed(KeyCode::F8) {
        return;
    }
    let names = lk2_core::world::terrain::presets::preset_names();
    let current = game_world.pipeline.name.clone();
    let next_idx =
        names.iter().position(|n| *n == current).map(|i| (i + 1) % names.len()).unwrap_or(0);
    let next_name = names[next_idx];
    let new_pipeline = lk2_core::world::terrain::presets::by_name(next_name);
    let new_name = new_pipeline.name.clone();
    game_world.pipeline = std::sync::Arc::new(new_pipeline);
    info!("🌍 F8 切 preset: {} -> {}", current, new_name);
}

pub fn freefly_movement(
    keys: Res<ButtonInput<KeyCode>>,
    mut freefly: ResMut<FreeFlyState>,
    angles: Res<CameraAngles>,
    time: Res<Time>,
) {
    if !freefly.enabled {
        return;
    }

    let (sy, cy) = angles.yaw.sin_cos();
    let (sp, cp) = angles.pitch.sin_cos();
    let forward = Vec3::new(sy * cp, sp, -cy * cp);

    let right = forward.cross(Vec3::Y);

    let up = Vec3::Y;

    let mut wish = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        wish += forward;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        wish -= forward;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        wish += right;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        wish -= right;
    }
    if keys.pressed(KeyCode::Space) {
        wish += up;
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        wish -= up;
    }

    let speed = if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::KeyE) {
        FREEFLY_SPEED * FREEFLY_BOOST
    } else {
        FREEFLY_SPEED
    };

    let target = if wish.length() > 0.01 {
        wish.normalize() * speed
    } else {
        Vec3::ZERO
    };
    let dt = time.delta_secs();

    let v = target;
    freefly.velocity = v;
    let pos = freefly.position + v * dt;
    freefly.position = pos;
}

pub fn setup_cursor_grab(
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    cfg: Res<RenderConfig>,
) {
    if !cfg.mouse_look {
        return;
    }
    if let Ok(mut cursor) = cursors.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

pub fn toggle_cursor_grab_on_esc(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    cfg: Res<RenderConfig>,
) {
    if !cfg.mouse_look {
        return;
    }
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    let Ok(mut cursor) = cursors.single_mut() else {
        return;
    };

    let is_locked = matches!(cursor.grab_mode, CursorGrabMode::Locked);
    if is_locked {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
        info!("🖱 ESC：释放光标（按 ESC 再抓回）");
    } else {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;

        info!("🖱 ESC：抓回光标");
    }
}

#[derive(Component)]
pub struct AnimalIndicatorText;

pub fn update_animal_indicator(
    mut q_text: Query<&mut Text, With<AnimalIndicatorText>>,
    player: Res<PlayerState>,
    camera: Query<&Transform, With<Camera3d>>,
    creatures: Query<&Creature>,
) {
    let Ok(mut text) = q_text.single_mut() else {
        return;
    };
    let px = player.block_pos[0] as f32 + 0.5;
    let pz = player.block_pos[2] as f32 + 0.5;

    let mut best: Option<(&Creature, f32)> = None;
    for c in creatures.iter() {
        let dx = c.block_pos[0] as f32 + 0.5 - px;
        let dz = c.block_pos[2] as f32 + 0.5 - pz;
        let d2 = dx * dx + dz * dz;
        if d2 < 900.0 && (best.is_none() || d2 < best.unwrap().1) {
            best = Some((c, d2));
        }
    }

    let Some((c, d2)) = best else {
        text.0 = "[no animal within 30m]".to_string();
        return;
    };
    let dist = d2.sqrt();
    let animal_v = Vec2::new(
        c.block_pos[0] as f32 + 0.5 - px,
        c.block_pos[2] as f32 + 0.5 - pz,
    );
    if animal_v.length() < 0.01 {
        text.0 = format!("* {} underfoot", c.kind.label_zh());
        return;
    }

    let arrow = if let Ok(tf) = camera.single() {
        let f = tf.forward();
        let cam = Vec2::new(f.x, f.z);
        let cam_n = if cam.length() > 0.01 {
            cam.normalize()
        } else {
            Vec2::new(1.0, 0.0)
        };
        let dot = animal_v.dot(cam_n);
        let cross = animal_v.x * cam_n.y - animal_v.y * cam_n.x;

        let angle = cross.atan2(dot);
        let oct = ((-angle).to_degrees() / 45.0).round() as i32;
        match oct.rem_euclid(8) {
            0 => "↑",
            1 => "↗",
            2 => "→",
            3 => "↘",
            4 => "↓",
            5 => "↙",
            6 => "←",
            7 => "↖",
            _ => "·",
        }
    } else {
        "·"
    };

    let label = match c.kind {
        lk2_core::creature::CreatureKind::Pig => "Pig",
        lk2_core::creature::CreatureKind::Sheep => "Sheep",
        lk2_core::creature::CreatureKind::Cow => "Cow",
        lk2_core::creature::CreatureKind::Chicken => "Chicken",
    };
    text.0 = format!("{}  {}  {:.1}m", arrow, label, dist);
}

const NEST_MARKER_SIZE: (f32, f32, f32) = (0.4, 4.0, 0.4);

#[derive(Component)]
pub struct NestMarker {
    pub nest_id: u32,
    pub kingdom_id: u32,

    pub offset_x: f32,
    pub offset_z: f32,
}

#[derive(Resource, Default)]
pub struct NestMarkerCount(pub u32);

pub fn spawn_nest_markers(
    mut commands: Commands,
    monsters: Res<MonsterEcosystem>,
    player: Res<PlayerState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut count: ResMut<NestMarkerCount>,
) {
    let px = player.block_pos[0] as f32 + 0.5;
    let pz = player.block_pos[2] as f32 + 0.5;
    let mut spawned = 0_u32;

    for (kid, kingdom) in monsters.kingdoms.iter() {
        if kingdom.destroyed {
            continue;
        }
        for (nid, nest) in kingdom.nests.iter() {
            let (base_r, base_g, base_b) = match nest.biome {
                Biome::Desert => (1.0_f32, 0.85_f32, 0.5_f32),
                Biome::Jungle => (0.4_f32, 0.7_f32, 0.3_f32),
                Biome::Tundra => (0.7_f32, 0.85_f32, 1.0_f32),
            };
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(base_r, base_g, base_b),
                emissive: Color::srgb(1.0, 0.25, 0.15).into(),
                perceptual_roughness: 0.7,
                metallic: 0.0,
                ..default()
            });

            let mesh = meshes.add(Cuboid::new(
                NEST_MARKER_SIZE.0,
                NEST_MARKER_SIZE.1,
                NEST_MARKER_SIZE.2,
            ));

            let nx = nest.center[0] as f32 + 0.5;
            let nz = nest.center[2] as f32 + 0.5;
            let ny = nest.center[1] as f32 + 5.0;
            let offset_x = nx - px;
            let offset_z = nz - pz;

            commands.spawn((
                NestMarker { nest_id: *nid, kingdom_id: *kid, offset_x, offset_z },
                Mesh3d(mesh),
                MeshMaterial3d(mat),
                Transform::from_translation(Vec3::new(px + offset_x, ny, pz + offset_z)),
            ));
            spawned += 1;
        }
    }

    count.0 = spawned;
    info!(
        "🏳 MonsterNest 3D 旗杆已 spawn {} 个（biome 配色 + RedStrong emissive）",
        spawned
    );
}

pub fn update_nest_marker_positions(
    mut q: Query<(&NestMarker, &mut Transform)>,
    player: Res<PlayerState>,
) {
    let px = player.pos.x;
    let pz = player.pos.z;
    for (m, mut tf) in q.iter_mut() {
        tf.translation.x = px + m.offset_x;
        tf.translation.z = pz + m.offset_z;
    }
}

#[derive(Component)]
pub struct NestIndicatorText;

pub fn update_nest_indicator(
    mut q_text: Query<&mut Text, With<NestIndicatorText>>,
    player: Res<PlayerState>,
    camera: Query<&Transform, With<Camera3d>>,
    monsters: Res<MonsterEcosystem>,
) {
    let Ok(mut text) = q_text.single_mut() else {
        return;
    };
    let px = player.block_pos[0] as f32 + 0.5;
    let pz = player.block_pos[2] as f32 + 0.5;

    let mut best: Option<([i32; 3], u32, f32)> = None;
    for (_kid, k) in monsters.kingdoms.iter() {
        if k.destroyed {
            continue;
        }
        for (_nid, n) in k.nests.iter() {
            let dx = n.center[0] as f32 + 0.5 - px;
            let dz = n.center[2] as f32 + 0.5 - pz;
            let d2 = dx * dx + dz * dz;
            if d2 < 900.0 && (best.is_none() || d2 < best.unwrap().2) {
                best = Some((n.center, n.individuals.len() as u32, d2));
            }
        }
    }

    let Some((center, count, d2)) = best else {
        text.0 = "[no nest within 30m]".to_string();
        return;
    };
    let dist = d2.sqrt();
    let nest_v = Vec2::new(center[0] as f32 + 0.5 - px, center[2] as f32 + 0.5 - pz);
    if nest_v.length() < 0.01 {
        text.0 = format!("* Nest underfoot / {} mobs", count);
        return;
    }

    let arrow = if let Ok(tf) = camera.single() {
        let f = tf.forward();
        let cam = Vec2::new(f.x, f.z);
        let cam_n = if cam.length() > 0.01 {
            cam.normalize()
        } else {
            Vec2::new(1.0, 0.0)
        };
        let dot = nest_v.dot(cam_n);
        let cross = nest_v.x * cam_n.y - nest_v.y * cam_n.x;

        let angle = cross.atan2(dot);
        let oct = ((-angle).to_degrees() / 45.0).round() as i32;
        match oct.rem_euclid(8) {
            0 => "↑",
            1 => "↗",
            2 => "→",
            3 => "↘",
            4 => "↓",
            5 => "↙",
            6 => "←",
            7 => "↖",
            _ => "·",
        }
    } else {
        "·"
    };

    text.0 = format!("{} Nest {:.0}m / {} mobs", arrow, dist, count);
}

#[derive(Resource, Default)]
pub struct LastMoveDirection(pub Vec3);

pub fn player_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut player: ResMut<PlayerState>,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<lk2_core::resource::GlobalResourcePool>,
    mut angles: ResMut<CameraAngles>,
    mut nations: ResMut<NationRegistry>,
    mut monsters: ResMut<MonsterEcosystem>,
    camera: Query<&Transform, With<Camera3d>>,
    time: Res<Time>,
    freefly: Res<FreeFlyState>,
    cfg: Res<RenderConfig>,
) {
    let freefly_active = freefly.enabled;

    let cam_tf = camera.single().ok();
    let (forward, right) = if let Some(tf) = cam_tf {
        let f = tf.forward();
        let f_h = Vec3::new(f.x, 0.0, f.z);
        let f_n = if f_h.length() > 0.01 {
            f_h.normalize()
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };

        let r = f_n.cross(Vec3::Y);
        (f_n, r)
    } else {
        (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0))
    };

    let mut d = Vec3::ZERO;
    if !freefly_active {
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            d += forward;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            d -= forward;
        }
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            d -= right;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            d += right;
        }
        if keys.just_pressed(KeyCode::Space) {
            d += Vec3::Y;
        }
        if keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight) {
            d -= Vec3::Y;
        }
    }

    if !freefly_active {
        if keys.just_pressed(KeyCode::KeyQ) {
            angles.yaw -= YAW_QE_STEP;
        } else if keys.just_pressed(KeyCode::KeyE) {
            angles.yaw += YAW_QE_STEP;
        }
    }

    if d.length() > 0.01 {
        if d.y.abs() > 0.01 && d.x.abs() < 0.01 && d.z.abs() < 0.01 {
            try_player_move(
                &mut player,
                &mut game_world,
                [0, d.y.signum() as i32, 0],
                cfg.ground_step_threshold,
            );
        } else {
            let sprint = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
            let speed = if sprint {
                MANUAL_MOVE_SPEED * 1.5
            } else {
                MANUAL_MOVE_SPEED
            };
            try_player_move_continuous(
                &mut player,
                &game_world,
                Vec3::new(d.x, 0.0, d.z),
                speed * time.delta_secs(),
                cfg.ground_step_threshold,
            );
        }
    }

    if keys.just_pressed(KeyCode::KeyG) {
        let (x, y, z) = (
            player.block_pos[0],
            player.block_pos[1] - 1,
            player.block_pos[2],
        );
        let b = game_world.get(x, y, z);
        if b.is_solid() {
            if let Some((res, _)) = b.yields() {
                game_world.set(x, y, z, BlockType::Air);
                let _ = pool.try_add(res, 1);
                *player.inventory.entry(res).or_insert(0) += 1;
                player.blocks_gathered += 1;
                info!(
                    "⛏ 你挖了 {:?} (库存 {:?})",
                    res,
                    player.inventory.get(&res).copied().unwrap_or(0)
                );
            } else {
                info!("这个方块挖不出东西");
            }
        } else {
            info!("脚下没方块");
        }
    }

    if keys.just_pressed(KeyCode::KeyF) {
        if player.nation_id.is_some() {
            info!("🚩 你已经是一个国家的王了，不能再立旗");
        } else {
            let tick_now = time.elapsed_secs() as u64;
            let cost = nations.next_flag_cost();
            match nations.found(
                &mut pool,
                0u32,
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

    if keys.just_pressed(KeyCode::KeyJ) {
        let p = player.block_pos;
        let mut best: Option<(f32, u32, u32, u32)> = None;
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
                info!(
                    "⚔ 你击杀了怪物 (距离 {:.1} 格) kid={} nid={} iid={}",
                    dist, kid, nid, iid
                );
            }
        } else {
            info!("⚔ 2 格内没有怪物");
        }
    }
}

const PLAYER_BODY_CLEARANCE_BLOCKS: i32 = 2;
const MAX_SMOOTH_DROP: f32 = 6.0;

fn player_body_clear(world: &GameWorld, x: i32, foot_y: i32, z: i32) -> bool {
    if foot_y < 0 || foot_y + PLAYER_BODY_CLEARANCE_BLOCKS > world.size {
        return false;
    }
    for y in foot_y..(foot_y + PLAYER_BODY_CLEARANCE_BLOCKS) {
        if world.get(x, y, z).is_solid() {
            return false;
        }
    }
    true
}

fn standable_foot_y(
    world: &GameWorld,
    x: i32,
    z: i32,
    near_y: f32,
    max_step_up: f32,
) -> Option<i32> {
    let min_y = ((near_y - MAX_SMOOTH_DROP).floor() as i32).max(1);
    let max_y = ((near_y + max_step_up).ceil() as i32).min(world.size - 2);
    (min_y..=max_y)
        .filter(|foot_y| {
            world.get(x, *foot_y - 1, z).is_solid() && player_body_clear(world, x, *foot_y, z)
        })
        .min_by(|a, b| {
            let da = (*a as f32 - near_y).abs();
            let db = (*b as f32 - near_y).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn standable_foot_y_any_height(world: &GameWorld, x: i32, z: i32) -> Option<i32> {
    (1..(world.size - 2)).rev().find(|foot_y| {
        world.get(x, *foot_y - 1, z).is_solid() && player_body_clear(world, x, *foot_y, z)
    })
}

pub fn player_stand_position_at(
    world: &GameWorld,
    x: i32,
    z: i32,
    near_y: f32,
    max_step_up: f32,
) -> Option<(Vec3, [i32; 3])> {
    let foot_y = standable_foot_y(world, x, z, near_y, max_step_up)?;
    Some((
        Vec3::new(x as f32 + 0.5, foot_y as f32, z as f32 + 0.5),
        [x, foot_y, z],
    ))
}

pub fn player_spawn_position_at(world: &GameWorld, x: i32, z: i32) -> Option<(Vec3, [i32; 3])> {
    let foot_y = standable_foot_y_any_height(world, x, z)?;

    let sky_y = foot_y + 2;
    Some((
        Vec3::new(x as f32 + 0.5, sky_y as f32, z as f32 + 0.5),
        [x, sky_y, z],
    ))
}

fn set_player_position(player: &mut PlayerState, pos: Vec3, block_pos: [i32; 3]) {
    player.pos = pos;
    player.block_pos = block_pos;
}

fn try_player_move(
    player: &mut PlayerState,
    game_world: &mut GameWorld,
    d: [i32; 3],
    threshold: f32,
) -> bool {
    if d[1] != 0 && d[0] == 0 && d[2] == 0 {
        let next_y = player.block_pos[1] + d[1];
        if !game_world.in_bounds(player.block_pos[0], next_y, player.block_pos[2]) {
            return false;
        }
        if !player_body_clear(game_world, player.block_pos[0], next_y, player.block_pos[2]) {
            return false;
        }
        let pos = Vec3::new(player.pos.x, next_y as f32, player.pos.z);
        set_player_position(
            player,
            pos,
            [player.block_pos[0], next_y, player.block_pos[2]],
        );
        return true;
    }

    let next_x = player.block_pos[0] + d[0];
    let next_z = player.block_pos[2] + d[2];
    let Some((pos, block_pos)) =
        player_stand_position_at(game_world, next_x, next_z, player.pos.y, threshold)
    else {
        return false;
    };

    if pos.y - player.pos.y > threshold {
        return false;
    }

    set_player_position(player, pos, block_pos);
    true
}

fn try_player_move_continuous(
    player: &mut PlayerState,
    game_world: &GameWorld,
    dir: Vec3,
    distance: f32,
    threshold: f32,
) -> bool {
    let horizontal = Vec3::new(dir.x, 0.0, dir.z);
    if horizontal.length_squared() < 0.0001 || distance <= 0.0 {
        return false;
    }

    let next = player.pos + horizontal.normalize() * distance;
    let next_x = next.x.floor() as i32;
    let next_z = next.z.floor() as i32;
    let Some((stand_pos, block_pos)) =
        player_stand_position_at(game_world, next_x, next_z, player.pos.y, threshold)
    else {
        return false;
    };

    if stand_pos.y - player.pos.y > threshold {
        return false;
    }

    set_player_position(player, Vec3::new(next.x, stand_pos.y, next.z), block_pos);
    true
}

#[derive(Component)]
pub struct Player;

pub fn auto_demo(
    time: Res<Time>,
    mut player: ResMut<PlayerState>,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<lk2_core::resource::GlobalResourcePool>,
    mut nations: ResMut<lk2_core::nation::NationRegistry>,
    cfg: Res<RenderConfig>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut last: ResMut<LastMoveDirection>,
    mut walk_timer: Local<f32>,
    mut walk_step: Local<u32>,
    mut auto_frame: Local<u32>,

    mut walk_target: Local<Option<[i32; 3]>>,
    mut player_tf_q: Query<&mut Transform, With<Player>>,
    creatures: Query<&lk2_core::creature::Creature>,
    monsters: Res<lk2_core::monster::MonsterEcosystem>,
) {
    if cfg.auto_keys {
        *auto_frame += 1;

        if *auto_frame == 60 {
            keys.press(KeyCode::KeyF);
        }
        if *auto_frame == 62 {
            keys.release(KeyCode::KeyF);
        }

        if *auto_frame == 240 {
            keys.press(KeyCode::KeyJ);
        }
        if *auto_frame == 242 {
            keys.release(KeyCode::KeyJ);
        }
        if *auto_frame == 300 || *auto_frame == 520 {
            keys.press(KeyCode::KeyI);
        }
        if *auto_frame == 302 || *auto_frame == 522 {
            keys.release(KeyCode::KeyI);
        }
        if *auto_frame == 360 {
            keys.press(KeyCode::KeyK);
        }
        if *auto_frame == 362 {
            keys.release(KeyCode::KeyK);
        }

        if *auto_frame == 480 {
            keys.press(KeyCode::KeyF);
        }
        if *auto_frame == 482 {
            keys.release(KeyCode::KeyF);
        }
    }

    if !cfg.auto_walk {
        return;
    }

    if keys.pressed(KeyCode::KeyW)
        || keys.pressed(KeyCode::KeyA)
        || keys.pressed(KeyCode::KeyS)
        || keys.pressed(KeyCode::KeyD)
        || keys.pressed(KeyCode::Space)
        || keys.pressed(KeyCode::ShiftLeft)
    {
        return;
    }
    *walk_timer += time.delta_secs();

    let walk_interval = if cfg.auto_keys {
        0.1
    } else {
        cfg.auto_walk_interval_secs
    };
    if *walk_timer < walk_interval {
        return;
    }
    *walk_timer = 0.0;
    *walk_step += 1;

    let need_refresh = match *walk_target {
        None => true,
        Some(t) => {
            let dx = (t[0] - player.block_pos[0]) as f32;
            let dz = (t[2] - player.block_pos[2]) as f32;
            dx * dx + dz * dz < 1.5 * 1.5
        }
    };
    if need_refresh {
        const MIN_WALK_SPREAD: f32 = 25.0;
        let mut best: Option<(f32, [i32; 3])> = None;
        let p = player.block_pos;
        let mut consider = |pos: [i32; 3], weight: f32| {
            let dx = (pos[0] - p[0]) as f32;
            let dz = (pos[2] - p[2]) as f32;
            let d2 = dx * dx + dz * dz * weight;

            if d2 < MIN_WALK_SPREAD {
                return;
            }
            match best {
                None => best = Some((d2, pos)),
                Some((bd, _)) if d2 < bd => best = Some((d2, pos)),
                _ => {}
            }
        };
        for c in creatures.iter() {
            consider(c.block_pos, 1.0);
        }
        for k in monsters.kingdoms.values() {
            if k.destroyed {
                continue;
            }
            for n in k.nests.values() {
                if n.dormant {
                    continue;
                }
                consider(n.center, 1.2);
            }
        }
        *walk_target = best.map(|(_, pos)| pos);
    }

    let all_dirs: [[i32; 3]; 9] = [
        [1, 0, 0],
        [-1, 0, 0],
        [0, 0, 1],
        [0, 0, -1],
        [1, 0, 1],
        [1, 0, -1],
        [-1, 0, 1],
        [-1, 0, -1],
        [0, 1, 0],
    ];

    let near_y = player.block_pos[1] as f32;
    let good_dirs: Vec<[i32; 3]> = all_dirs
        .iter()
        .filter(|d| {
            let nx1 = player.block_pos[0] + d[0];
            let nz1 = player.block_pos[2] + d[2];
            let nx2 = nx1 + d[0];
            let nz2 = nz1 + d[2];
            player_stand_position_at(&game_world, nx1, nz1, near_y, cfg.ground_step_threshold)
                .and_then(|(pos, _)| {
                    player_stand_position_at(
                        &game_world,
                        nx2,
                        nz2,
                        pos.y,
                        cfg.ground_step_threshold,
                    )
                })
                .is_some()
        })
        .copied()
        .collect();

    let d: [i32; 3] = if good_dirs.is_empty() {
        match *walk_target {
            Some(t) => {
                let dx = (t[0] - player.block_pos[0]).signum();
                let dz = (t[2] - player.block_pos[2]).signum();
                [dx, 0, dz]
            }
            None => [1, 0, 0],
        }
    } else {
        match *walk_target {
            Some(t) => {
                let dx = (t[0] - player.block_pos[0]).signum();
                let dz = (t[2] - player.block_pos[2]).signum();
                good_dirs
                    .iter()
                    .copied()
                    .find(|gd| gd[0] == dx && gd[2] == dz)
                    .unwrap_or_else(|| good_dirs[(*walk_step as usize) % good_dirs.len()])
            }
            None => good_dirs[(*walk_step as usize) % good_dirs.len()],
        }
    };

    let before = player.block_pos;

    let moved = try_player_move(&mut player, &mut game_world, d, cfg.ground_step_threshold);
    if !moved {
        let nx = player.block_pos[0] + d[0];
        let nz = player.block_pos[2] + d[2];
        if game_world.in_bounds(nx, 1, nz) {
            player.block_pos = [nx, player.block_pos[1], nz];
            player.pos = Vec3::new(
                nx as f32 + 0.5,
                player.block_pos[1] as f32 + 0.85,
                nz as f32 + 0.5,
            );
        }
    }
    if moved || (player.block_pos != before) {
        let dx = (player.block_pos[0] - before[0]) as f32;
        let dz = (player.block_pos[2] - before[2]) as f32;
        if dx != 0.0 || dz != 0.0 {
            let v = Vec3::new(dx, 0.0, dz);
            last.0 = v.normalize();
        }
        if let Ok(mut tf) = player_tf_q.single_mut() {
            tf.translation = player.pos;
        }
    }

    const AI_WALK_KING_ID_BASE: u32 = 100;
    if let Some(target) = *walk_target {
        let dx = (player.block_pos[0] - target[0]) as f32;
        let dz = (player.block_pos[2] - target[2]) as f32;
        if dx * dx + dz * dz <= 1.5 * 1.5 {
            let cost = nations.next_flag_cost();

            if cfg.auto_keys {
                let have = pool.get(lk2_core::resource::ResourceKind::Soul);
                if have < cost {
                    let _ = pool.try_add(lk2_core::resource::ResourceKind::Soul, cost - have);
                }
            }
            let ai_king_id = AI_WALK_KING_ID_BASE + nations.flag_count;
            match nations.found(
                &mut pool,
                ai_king_id,
                format!("AIWalk#{}@{:?}", ai_king_id, player.block_pos),
                player.block_pos,
                *auto_frame as u64,
            ) {
                Ok(id) => {
                    tracing::info!(
                        "[auto-demo walk] ✓ 到达 walk_target, 在 ({},{},{}) 真建新国 id={} (king={}, cost={}, total={})",
                        player.block_pos[0],
                        player.block_pos[1],
                        player.block_pos[2],
                        id.0,
                        ai_king_id,
                        cost,
                        nations.flag_count
                    );
                }
                Err(e) => {
                    tracing::warn!("[auto-demo walk] 到达 walk_target 但建新国失败: {}", e);
                }
            }
        }
    }

    let cur = game_world.get(
        player.block_pos[0],
        player.block_pos[1] - 1,
        player.block_pos[2],
    );
    if let Some((res, _)) = cur.yields() {
        if cur.is_solid() {
            game_world.set(
                player.block_pos[0],
                player.block_pos[1] - 1,
                player.block_pos[2],
                BlockType::Air,
            );
            *player.inventory.entry(res).or_insert(0) += 1;
            player.blocks_gathered += 1;
        }
    }
}

pub fn first_person_camera(
    mut q: Query<&mut Transform, With<Camera3d>>,
    time: Res<Time>,
    player: Res<PlayerState>,
    angles: Res<CameraAngles>,
    cfg: Res<RenderConfig>,
    last: Res<LastMoveDirection>,
    creatures: Query<&Creature>,
    world: Res<GameWorld>,
    freefly: Res<FreeFlyState>,
    mode: Res<CameraMode>,
    mut orbit_angle: Local<f32>,
    anim_state: Res<PlayerAnimState>,
) {
    let Ok(mut tf) = q.single_mut() else {
        return;
    };

    if freefly.enabled {
        let (sy, cy) = angles.yaw.sin_cos();
        let (sp, cp) = angles.pitch.sin_cos();
        let dir = Vec3::new(sy * cp, sp, -cy * cp);
        let look_target = freefly.position + dir * 5.0;
        tf.translation = freefly.position;
        tf.look_at(look_target, Vec3::Y);
        return;
    }

    if cfg.auto_orbit && !cfg.mouse_look {
        *orbit_angle += time.delta_secs() * cfg.auto_orbit_speed;
        let a = *orbit_angle;
        let target = player.pos - Vec3::Y * 1.25;
        let cam_pos = target
            + Vec3::new(
                a.cos() * cfg.auto_orbit_distance,
                14.0,
                a.sin() * cfg.auto_orbit_distance,
            );
        tf.translation = cam_pos;
        tf.look_at(target, Vec3::Y);
        return;
    }

    if *mode == CameraMode::ThirdPerson {
        let (sy, cy) = angles.yaw.sin_cos();

        let back = Vec3::new(-sy, 0.0, cy);
        let target = player.pos + Vec3::Y * 1.4;
        let cam_pos = target + back * TP_DISTANCE + Vec3::new(0.0, TP_HEIGHT, 0.0);
        tf.translation = cam_pos;
        tf.look_at(target, Vec3::Y);
        return;
    }

    let eye_base = player.pos + Vec3::Y * 1.7;

    let phase = anim_state.step_phase;
    let speed = anim_state.smoothed_speed;
    let bob_y = phase.sin().abs() * 0.06 * speed + (phase * 0.18).sin() * 0.012;
    let bob_x = (phase * 0.5).sin() * 0.04 * speed;
    let eye = eye_base + Vec3::new(bob_x, bob_y, 0.0);

    let dir = if cfg.mouse_look {
        let (sy, cy) = angles.yaw.sin_cos();
        let (sp, cp) = angles.pitch.sin_cos();
        Vec3::new(sy * cp, sp, -cy * cp)
    } else {
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
            let tx = c[0] as f32 + 0.5;
            let tz = c[2] as f32 + 0.5;
            let mut clear = true;
            for i in 1..=5 {
                let t = i as f32 / 5.0;
                let x = eye.x + (tx - eye.x) * t;
                let z = eye.z + (tz - eye.z) * t;
                let bx = x.floor() as i32;
                let bz = z.floor() as i32;
                let by = eye.y.floor() as i32;
                if world.in_bounds(bx, by, bz) && world.get(bx, by, bz).is_solid() {
                    clear = false;
                    break;
                }
            }
            if clear {
                best = Some(*c);
                break;
            }
        }
        if let Some(c) = best {
            let v = Vec3::new(
                (c[0] as f32 + 0.5) - eye.x,
                0.0,
                (c[2] as f32 + 0.5) - eye.z,
            );
            if v.length() > 0.01 {
                v.normalize()
            } else {
                Vec3::new(1.0, 0.0, 0.0)
            }
        } else if last.0.length() < 0.01 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            last.0.normalize()
        }
    };
    let look_target = eye + dir * 5.0 - Vec3::new(0.0, 1.0, 0.0);
    tf.translation = eye;
    tf.look_at(look_target, Vec3::Y);
}
