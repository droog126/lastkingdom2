//! Bevy resources and components shared by the living forest scene.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use avian3d::prelude::PhysicsGizmos;
use bevy::prelude::*;
use lk2_core::farming::CropKind;
use lk2_core::legendary::LegendaryWeapon;
use lk2_core::pvp::CREATURE_HIT_INVULNERABILITY_TICKS;
use lk2_core::resource::ResourceKind;
use lk2_core::world::terrain::{self, LandformModule, TerrainPipeline};

use super::creature_ai::AiMood;
use super::stylized_material::StylizedTerrainMaterial;

pub const PROCEDURAL_TERRAIN_CENTER: i32 = 48;
pub const PROCEDURAL_TERRAIN_RADIUS: i32 = 64;
pub const PROCEDURAL_COLLISION_GRID_STEP: i32 = 1;
pub const MINE_SURFACE_DEFORMATION_RADIUS: f32 = 1.8;
// Keep replaced meshes alive long enough for the render world to finish the
// current frame, but do not retain a minute's worth of terrain/water meshes
// after every edit. The old value allowed repeated edits to exhaust VRAM.
pub const TERRAIN_MESH_RETIRE_FRAMES: u16 = 8;
pub const PROCEDURAL_TERRAIN_WATER_CLEARANCE: f32 = 0.08;
const PROCEDURAL_TERRAIN_REFERENCE_SURFACE: f32 = 20.0;
const PROCEDURAL_TERRAIN_HEIGHT_SCALE: f32 = 0.14;
// The authored terrain profile is expressed in voxel-height units, while
// the presentation world compresses those units to 0.14m. Keep the visible
// seabed deep enough for the player capsule to leave the surface and dive.
const PROCEDURAL_WATER_FLOOR_DEPTH_BONUS: f32 = 1.15;
pub const PROCEDURAL_TERRAIN_SEA_LEVEL: f32 = 12.0;
// Gameplay, content placement, and collision retain the established height
// query cadence. The presentation mesh owns its finer smooth render grid.
const PROCEDURAL_TERRAIN_GRID_STEP: i32 = 2;

#[derive(Resource, Clone)]
pub struct ProceduralTerrainSurface {
    pub pipeline: Arc<TerrainPipeline>,
    pub landform: LandformModule,
    pub deformations: Vec<TerrainDeformation>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainDeformation {
    pub center: Vec2,
    pub radius: f32,
    pub depth: f32,
}

pub struct RetiredTerrainMesh {
    pub handle: Handle<Mesh>,
    pub frames_remaining: u16,
}

#[derive(Resource, Default)]
pub struct TerrainRebuildState {
    pub requested: bool,
    pub last_mined: Option<[i32; 3]>,
    pub feedback_remaining: f32,
    pub retired_meshes: Vec<RetiredTerrainMesh>,
}

#[derive(Resource, Default)]
pub struct TerrainUndergroundState {
    pub requested: bool,
    pub chunk: Option<lk2_core::world::TerrainChunkCoord>,
    pub target: Option<[i32; 3]>,
}

#[derive(Component)]
pub struct CaveEntrance;

#[derive(Component)]
pub struct CaveExit;

#[derive(Component)]
pub struct CaveRealmVisual;

#[derive(Component)]
pub struct CaveRealmCollider;

#[derive(Component, Clone, Copy)]
pub struct CaveOre {
    pub resource: ResourceKind,
    pub amount: i64,
}

#[derive(Resource, Default)]
pub struct CaveTravelState {
    pub active: bool,
    pub return_position: Option<Vec3>,
    pub return_rotation: Option<Quat>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DimensionId {
    #[default]
    Forest,
    Starfall,
}

#[derive(Resource, Debug)]
pub struct DimensionTravelState {
    pub current: DimensionId,
    pub return_position: Option<Vec3>,
    pub return_rotation: Option<Quat>,
}

impl Default for DimensionTravelState {
    fn default() -> Self {
        Self {
            current: DimensionId::Forest,
            return_position: None,
            return_rotation: None,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct DimensionPortal {
    pub destination: DimensionId,
}

/// Visuals authored for the forest are hidden while the starfall dimension is
/// active. Runtime ecology assets also receive this marker through
/// `spawn_asset`, so the client does not need to invent a second ecology.
#[derive(Component)]
pub struct ForestRealmVisual;

#[derive(Component)]
pub struct StarfallRealmVisual;

#[derive(Component)]
pub struct ForestRealmCollider;

#[derive(Component)]
pub struct StarfallRealmCollider;

#[derive(Resource, Debug, Clone, Copy)]
pub struct StarfallProgress {
    pub collected: u32,
    pub total: u32,
    pub guardian_defeated: bool,
    pub reward_claimed: bool,
}

impl Default for StarfallProgress {
    fn default() -> Self {
        Self {
            collected: 0,
            total: 8,
            guardian_defeated: false,
            reward_claimed: false,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct StarfallShard {
    pub base: Vec3,
    pub phase: f32,
}

#[derive(Component)]
pub struct StarfallRelic;

#[derive(Component, Clone, Copy)]
pub struct StarfallGuardian {
    pub base: Vec3,
    pub phase: f32,
}

#[derive(Component, Default)]
pub struct StarfallGuardianAttack {
    pub cooldown_remaining: f32,
}

#[derive(Resource, Default)]
pub struct CollisionDebugState {
    pub enabled: bool,
}

pub fn configure_collision_debug_gizmos(store: &mut GizmoConfigStore, enabled: bool) {
    let (config, physics) = store.config_mut::<PhysicsGizmos>();
    config.enabled = enabled;
    *physics = PhysicsGizmos::colliders(Color::srgb(1.0, 0.16, 0.08));
}

impl ProceduralTerrainSurface {
    pub fn default_world() -> Self {
        Self {
            pipeline: Arc::new(terrain::presets::default_preset()),
            landform: LandformModule::default(),
            deformations: Vec::new(),
        }
    }

    pub fn render_height(&self, surface: f32) -> f32 {
        (surface - PROCEDURAL_TERRAIN_REFERENCE_SURFACE) * PROCEDURAL_TERRAIN_HEIGHT_SCALE
    }

    pub fn water_surface_height(&self) -> f32 {
        self.render_height(PROCEDURAL_TERRAIN_SEA_LEVEL)
    }

    pub fn ground_height(&self, position: Vec3) -> f32 {
        let step = PROCEDURAL_TERRAIN_GRID_STEP as f32;
        let local_x0 = (position.x / step).floor() * step;
        let local_z0 = (position.z / step).floor() * step;
        let tx = ((position.x - local_x0) / step).clamp(0.0, 1.0);
        let tz = ((position.z - local_z0) / step).clamp(0.0, 1.0);
        let x0 = PROCEDURAL_TERRAIN_CENTER + local_x0 as i32;
        let z0 = PROCEDURAL_TERRAIN_CENTER + local_z0 as i32;
        let x1 = x0 + PROCEDURAL_TERRAIN_GRID_STEP;
        let z1 = z0 + PROCEDURAL_TERRAIN_GRID_STEP;
        let h00 = self.vertex_ground_height(x0, z0);
        let h10 = self.vertex_ground_height(x1, z0);
        let h11 = self.vertex_ground_height(x1, z1);
        let h01 = self.vertex_ground_height(x0, z1);

        // Keep gameplay and collision interpolation deterministic on the
        // established grid; presentation uses a separate smooth render grid.
        if tz <= tx {
            h00 + tx * (h10 - h00) + tz * (h11 - h10)
        } else {
            h00 + tz * (h01 - h00) + tx * (h11 - h01)
        }
    }

    /// Sample the exact piecewise plane used by the 1x1 collision mesh.
    ///
    /// `ground_height` intentionally uses the coarser gameplay grid. Player
    /// contacts and visual slopes must use this companion query or they can
    /// resolve against a different plane on the same hillside.
    pub fn collision_ground_height(&self, position: Vec3) -> f32 {
        let x0 = position.x.floor() as i32;
        let z0 = position.z.floor() as i32;
        let tx = position.x - x0 as f32;
        let tz = position.z - z0 as f32;
        let world_x = PROCEDURAL_TERRAIN_CENTER + x0;
        let world_z = PROCEDURAL_TERRAIN_CENTER + z0;
        let h00 = self.terrain_floor_height_at_world(world_x, world_z);
        let h10 = self.terrain_floor_height_at_world(world_x + 1, world_z);
        let h11 = self.terrain_floor_height_at_world(world_x + 1, world_z + 1);
        let h01 = self.terrain_floor_height_at_world(world_x, world_z + 1);

        if tz <= tx {
            h00 + tx * (h10 - h00) + tz * (h11 - h10)
        } else {
            h00 + tz * (h01 - h00) + tx * (h11 - h01)
        }
    }

    /// Return the normal of the exact collision triangle under `position`.
    pub fn collision_surface_normal(&self, position: Vec3) -> Vec3 {
        let x0 = position.x.floor() as i32;
        let z0 = position.z.floor() as i32;
        let tx = position.x - x0 as f32;
        let tz = position.z - z0 as f32;
        let world_x = PROCEDURAL_TERRAIN_CENTER + x0;
        let world_z = PROCEDURAL_TERRAIN_CENTER + z0;
        let h00 = self.terrain_floor_height_at_world(world_x, world_z);
        let h10 = self.terrain_floor_height_at_world(world_x + 1, world_z);
        let h11 = self.terrain_floor_height_at_world(world_x + 1, world_z + 1);
        let h01 = self.terrain_floor_height_at_world(world_x, world_z + 1);
        let normal = if tz <= tx {
            Vec3::new(-(h10 - h00), 1.0, -(h11 - h10))
        } else {
            Vec3::new(-(h11 - h01), 1.0, -(h01 - h00))
        };
        normal.normalize_or_zero()
    }

    pub fn ground_height_at_world(&self, x: i32, z: i32) -> f32 {
        let base_surface = self.base_surface_at_world(x, z);
        let surface = if self.is_water_at(x, z, base_surface) {
            PROCEDURAL_TERRAIN_SEA_LEVEL
        } else {
            self.deformed_surface_at_world(x as f32, z as f32, base_surface)
        };
        self.render_height(surface)
    }

    /// Returns the actual terrain floor, including submerged terrain.
    ///
    /// `ground_height_at_world` intentionally returns the walkable surface
    /// (the sea surface for water cells). Swimming and the physics mesh need
    /// the separate floor query so water is a volume rather than a solid lid.
    pub fn terrain_floor_height_at_world(&self, x: i32, z: i32) -> f32 {
        let base_surface = self.base_surface_at_world(x, z);
        let floor =
            self.render_height(self.deformed_surface_at_world(x as f32, z as f32, base_surface));
        if self.is_water_at(x, z, base_surface) {
            floor - PROCEDURAL_WATER_FLOOR_DEPTH_BONUS
        } else {
            floor
        }
    }

    pub fn terrain_floor_height(&self, position: Vec3) -> f32 {
        let step = PROCEDURAL_TERRAIN_GRID_STEP as f32;
        let local_x0 = (position.x / step).floor() * step;
        let local_z0 = (position.z / step).floor() * step;
        let tx = ((position.x - local_x0) / step).clamp(0.0, 1.0);
        let tz = ((position.z - local_z0) / step).clamp(0.0, 1.0);
        let x0 = PROCEDURAL_TERRAIN_CENTER + local_x0 as i32;
        let z0 = PROCEDURAL_TERRAIN_CENTER + local_z0 as i32;
        let x1 = x0 + PROCEDURAL_TERRAIN_GRID_STEP;
        let z1 = z0 + PROCEDURAL_TERRAIN_GRID_STEP;
        let h00 = self.terrain_floor_height_at_world(x0, z0);
        let h10 = self.terrain_floor_height_at_world(x1, z0);
        let h11 = self.terrain_floor_height_at_world(x1, z1);
        let h01 = self.terrain_floor_height_at_world(x0, z1);

        if tz <= tx {
            h00 + tx * (h10 - h00) + tz * (h11 - h10)
        } else {
            h00 + tz * (h01 - h00) + tx * (h11 - h01)
        }
    }

    pub fn water_floor_height(&self, position: Vec3) -> f32 {
        self.terrain_floor_height(position)
    }

    pub fn water_depth_at(&self, position: Vec3) -> Option<f32> {
        let world_x = PROCEDURAL_TERRAIN_CENTER + position.x.floor() as i32;
        let world_z = PROCEDURAL_TERRAIN_CENTER + position.z.floor() as i32;
        let surface = self.base_surface_at_world(world_x, world_z);
        self.is_water_at(world_x, world_z, surface)
            .then(|| (self.water_surface_height() - self.water_floor_height(position)).max(0.0))
    }

    pub fn dig_at(&mut self, local_position: Vec3, radius: f32, depth: f32) {
        self.deformations.push(TerrainDeformation {
            center: Vec2::new(
                PROCEDURAL_TERRAIN_CENTER as f32 + local_position.x,
                PROCEDURAL_TERRAIN_CENTER as f32 + local_position.z,
            ),
            radius: radius.max(0.1),
            depth: depth.max(0.0),
        });
    }

    /// Add the presentation edit for the exact terrain column changed by the
    /// authoritative voxel operation.  Mining resolves an integer world
    /// column, while the camera hit point is fractional and can sit in a
    /// neighboring column after rounding.  Keeping this conversion here
    /// makes the surface, cavity, and underground projections share one
    /// coordinate contract.
    pub fn dig_at_world_column(&mut self, world_x: i32, world_z: i32, radius: f32, depth: f32) {
        self.dig_at(
            Vec3::new(
                world_x as f32 - PROCEDURAL_TERRAIN_CENTER as f32,
                0.0,
                world_z as f32 - PROCEDURAL_TERRAIN_CENTER as f32,
            ),
            radius,
            depth,
        );
    }

    /// Project an authoritative exposed surface height onto the smooth
    /// presentation field. Deep edits that do not change the exposed surface
    /// intentionally do not create a second surface crater; their underground
    /// mesh is still rebuilt from the voxel world.
    pub fn dig_at_world_column_to_surface(
        &mut self,
        world_x: i32,
        world_z: i32,
        target_surface: f32,
        radius: f32,
    ) -> bool {
        let base_surface = self.base_surface_at_world(world_x, world_z);
        if self.is_water_at(world_x, world_z, base_surface) || !target_surface.is_finite() {
            return false;
        }
        let current_surface =
            self.deformed_surface_at_world(world_x as f32, world_z as f32, base_surface);
        let depth = current_surface - target_surface;
        if !depth.is_finite() || depth <= f32::EPSILON {
            return false;
        }
        self.dig_at_world_column(world_x, world_z, radius, depth);
        true
    }

    fn base_surface_at_world(&self, x: i32, z: i32) -> f32 {
        self.pipeline
            .surface_f32(x, z)
            .unwrap_or(PROCEDURAL_TERRAIN_SEA_LEVEL + 1.0)
    }

    fn deformed_surface_at_world(&self, x: f32, z: f32, base_surface: f32) -> f32 {
        self.deformations
            .iter()
            .fold(base_surface, |surface, edit| {
                let distance = Vec2::new(x, z).distance(edit.center);
                if distance >= edit.radius {
                    return surface;
                }
                let t = 1.0 - distance / edit.radius;
                let smooth = t * t * (3.0 - 2.0 * t);
                (surface - edit.depth * smooth).max(0.1)
            })
    }

    fn vertex_ground_height(&self, x: i32, z: i32) -> f32 {
        self.ground_height_at_world(x, z)
    }

    pub fn is_water_at(&self, x: i32, z: i32, surface: f32) -> bool {
        let _ = (x, z);
        surface < PROCEDURAL_TERRAIN_SEA_LEVEL
    }

    /// Returns whether a local scene-space position is over water.
    ///
    /// Ecology coordinates are intentionally kept in local scene space while
    /// the terrain pipeline samples world-space coordinates around
    /// `PROCEDURAL_TERRAIN_CENTER`. Keeping this conversion here prevents each
    /// presentation consumer from inventing a slightly different water test.
    pub fn is_water_local(&self, position: Vec3) -> bool {
        let world_x = PROCEDURAL_TERRAIN_CENTER + position.x.round() as i32;
        let world_z = PROCEDURAL_TERRAIN_CENTER + position.z.round() as i32;
        let surface = self.base_surface_at_world(world_x, world_z);
        self.is_water_at(world_x, world_z, surface)
    }

    /// Returns whether a point is safe for an above-water visual. The discrete
    /// cell test alone is insufficient at shorelines because the presentation
    /// heightfield interpolates between submerged and walkable samples.
    pub fn is_renderable_land_at(&self, position: Vec3) -> bool {
        !self.is_water_local(position)
            && self.ground_height(position)
                > self.water_surface_height() + PROCEDURAL_TERRAIN_WATER_CLEARANCE
    }

    /// Finds the closest land cell for a local scene-space position.
    ///
    /// This is a presentation projection only: authoritative ecology keeps
    /// its original x/z coordinates, while visible assets are never placed on
    /// the water plane. A bounded search keeps malformed/off-map snapshots
    /// from turning into an unbounded per-frame scan.
    pub fn nearest_land_position(&self, position: Vec3, max_radius: i32) -> Option<Vec3> {
        if self.is_renderable_land_at(position) {
            return Some(Vec3::new(position.x, 0.0, position.z));
        }
        let origin_x = position.x.round() as i32;
        let origin_z = position.z.round() as i32;
        let max_radius = max_radius.max(0);
        let mut best: Option<(f32, Vec3)> = None;

        for x in (origin_x - max_radius)..=(origin_x + max_radius) {
            for z in (origin_z - max_radius)..=(origin_z + max_radius) {
                let candidate = Vec3::new(x as f32, 0.0, z as f32);
                if !self.is_renderable_land_at(candidate) {
                    continue;
                }
                let distance_squared =
                    candidate.distance_squared(Vec3::new(position.x, 0.0, position.z));
                if best.is_none_or(|(best_distance, _)| distance_squared < best_distance) {
                    best = Some((distance_squared, candidate));
                }
            }
        }

        best.map(|(_, candidate)| candidate)
    }

    /// Projects a local position onto the nearest land cell and grounds it.
    pub fn grounded_land_position(
        &self,
        position: Vec3,
        max_radius: i32,
        lift: f32,
    ) -> Option<Vec3> {
        let land = self.nearest_land_position(position, max_radius)?;
        Some(Vec3::new(land.x, self.ground_height(land) + lift, land.z))
    }

    pub fn surface_normal(&self, position: Vec3) -> Vec3 {
        const SAMPLE_DISTANCE: f32 = 1.0;
        let left = self.ground_height(position - Vec3::X * SAMPLE_DISTANCE);
        let right = self.ground_height(position + Vec3::X * SAMPLE_DISTANCE);
        let back = self.ground_height(position - Vec3::Z * SAMPLE_DISTANCE);
        let front = self.ground_height(position + Vec3::Z * SAMPLE_DISTANCE);
        Vec3::new(
            -(right - left) / (SAMPLE_DISTANCE * 2.0),
            1.0,
            -(front - back) / (SAMPLE_DISTANCE * 2.0),
        )
        .normalize()
    }
}

#[derive(Resource, Default)]
pub struct CodexSceneInput {
    pub motion: Vec2,
    pub jump: bool,
    pub mine: bool,
    pub attack: bool,
    pub pulse_secs: f32,
}

#[derive(Resource)]
pub struct LivingSceneState {
    pub elapsed: f32,
    pub frame: u64,
    pub auto_shot: bool,
    pub shot_requested: bool,
    pub exit_deadline: Option<Instant>,
    pub png_path: PathBuf,
    pub attack_flash: f32,
    pub camera_shake: f32,
    pub auto_demo: bool,
    pub iter_dir: Option<PathBuf>,
    pub frame_dt_over_50ms: u64,
    pub frame_dt_max_ms: f32,
}

#[derive(Resource)]
pub struct SceneMaterials {
    pub ground: Handle<StylizedTerrainMaterial>,
    pub hit_effect: Handle<bevy_hanabi::EffectAsset>,
    pub enemy_hit_effect: Handle<bevy_hanabi::EffectAsset>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    FirstPerson,
    ThirdPerson,
    FreeCam,
    GodView,
}

impl CameraMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FirstPerson => "FirstPerson",
            Self::ThirdPerson => "ThirdPerson",
            Self::FreeCam => "FreeCam",
            Self::GodView => "GodView",
        }
    }
}

#[derive(Resource)]
pub struct LivingCameraRig {
    pub mode: CameraMode,
    pub yaw: f32,
    pub pitch: f32,
    pub free_position: Vec3,
    pub free_yaw: f32,
    pub free_pitch: f32,
    pub free_return_mode: CameraMode,
    /// The map focus and orbit state used by the world overview camera.
    pub god_focus: Vec3,
    pub god_yaw: f32,
    pub god_pitch: f32,
    pub god_distance: f32,
    pub god_return_mode: CameraMode,
}

impl Default for LivingCameraRig {
    fn default() -> Self {
        Self {
            // The opening scene is a spatial readability test. Show the
            // avatar and the authored landmarks before offering first-person
            // as an optional camera mode.
            mode: CameraMode::ThirdPerson,
            yaw: std::f32::consts::PI,
            // Keep the initial first-person composition slightly above the
            // horizon so the opening view does not become mostly foreground.
            pitch: 0.02,
            free_position: Vec3::new(0.0, 4.0, 6.0),
            free_yaw: std::f32::consts::PI,
            free_pitch: -0.2,
            free_return_mode: CameraMode::FirstPerson,
            god_focus: Vec3::ZERO,
            god_yaw: std::f32::consts::PI,
            god_pitch: -1.18,
            god_distance: 140.0,
            god_return_mode: CameraMode::FirstPerson,
        }
    }
}

#[derive(Component)]
pub struct PlayerActor;

/// Presentation-only identity for objects that can be identified by the
/// center-screen target probe. This deliberately lives on the client root;
/// imported GLTF mesh children do not need to carry gameplay semantics.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inspectable {
    pub display_name: &'static str,
    pub category: &'static str,
}

#[derive(Component, Default)]
pub struct PlayerSwimState {
    pub active: bool,
    /// Presentation/heading velocity for the manually-stepped kinematic
    /// swimmer. This must not be copied into Avian's `LinearVelocity`, or the
    /// physics step would advance the same body a second time.
    pub velocity: Vec3,
}

#[derive(Component, Clone, Copy)]
pub struct WaterSeaweed {
    pub origin: Vec3,
    pub phase: f32,
    pub height: f32,
}

#[derive(Component, Clone, Copy)]
pub struct WaterFish {
    pub origin: Vec3,
    pub phase: f32,
}

#[derive(Component)]
pub struct ExplorableBuilding {
    pub interior: Option<Entity>,
}

#[derive(Component)]
pub struct BuildingInteriorRoot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildingPrompt {
    Enter,
    Exit,
}

#[derive(Resource, Default)]
pub struct BuildingExplorationState {
    pub active: Option<Entity>,
    pub prompt: Option<BuildingPrompt>,
}

#[derive(Component, Default)]
pub struct PlayerMotion {
    pub planar_velocity: Vec3,
    pub planar_speed: f32,
    pub smoothed_speed: f32,
    pub stride_phase: f32,
    pub crouch_amount: f32,
    pub crouch_target: f32,
    pub collider_crouch_amount: f32,
    pub airborne_time: f32,
    pub grounded: bool,
    pub moving: bool,
    /// World-space contacts used by the presentation IK. These are not
    /// authoritative movement state; they only keep a support foot planted
    /// while the body moves past it.
    pub left_foot_target: Vec3,
    pub right_foot_target: Vec3,
    pub step_start: Vec3,
    pub step_goal: Vec3,
    pub step_progress: f32,
    pub step_duration: f32,
    pub stepping_left: bool,
    pub foot_targets_initialized: bool,
    pub left_foot_lift: f32,
    pub right_foot_lift: f32,
}

#[derive(Component, Default)]
pub struct PlayerJump {
    pub vertical_velocity: f32,
    pub coyote_timer: f32,
    pub jump_buffer_timer: f32,
}

#[derive(Component, Default)]
pub struct PlayerSkillState {
    pub cooldown_remaining: f32,
}

impl PlayerSkillState {
    pub fn ready(&self) -> bool {
        self.cooldown_remaining <= 0.0
    }

    pub fn begin(&mut self, cooldown_secs: f32) -> bool {
        if !self.ready() {
            return false;
        }
        self.cooldown_remaining = cooldown_secs.max(0.0);
        true
    }

    pub fn tick(&mut self, delta_secs: f32) {
        self.cooldown_remaining = (self.cooldown_remaining - delta_secs.max(0.0)).max(0.0);
    }
}

#[derive(Component)]
pub struct BossActor {
    pub base: Vec3,
}

#[derive(Component)]
pub struct LivingSceneCamera;

#[derive(Component)]
pub struct LivingSun;

#[derive(Component)]
pub struct PlayerGroundShadow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerIkPartKind {
    Torso,
    Pants,
    Neck,
    Head,
    Hair,
    HairSideL,
    HairSideR,
    EyeL,
    EyeR,
    ArmUpperL,
    ArmLowerL,
    ArmUpperR,
    ArmLowerR,
    HandL,
    HandR,
    LegUpperL,
    LegLowerL,
    LegUpperR,
    LegLowerR,
    BootL,
    BootR,
    Stick,
}

#[derive(Component)]
pub struct PlayerIkPart {
    pub kind: PlayerIkPartKind,
}

#[derive(Component)]
pub struct DragonKatanaPickup;

#[derive(Component)]
pub struct SettlementCampVisual;

#[derive(Component)]
pub struct ReaperScythePickup;

#[derive(Component)]
pub struct HeldWeaponVisual {
    pub weapon: LegendaryWeapon,
}

#[derive(Component)]
pub struct GrassTuft {
    pub growth_threshold: f32,
    pub mature_scale: Vec3,
}

#[derive(Component)]
pub struct BerryBush {
    pub id: u32,
    pub mature_scale: Vec3,
}

#[derive(Component)]
pub struct Rabbit {
    pub id: u32,
    pub phase: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RabbitMood {
    Idle,
    Forage,
    Graze,
    Flee,
}

impl Default for RabbitMood {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Component, Default)]
pub struct RabbitAi {
    pub mood: RabbitMood,
    pub target: Vec3,
    pub hop_phase: f32,
}

#[derive(Component)]
pub struct Wolf {
    pub id: u32,
    pub phase: f32,
}

/// Keeps a defeated nature visual in reconciliation's id set while hiding it
/// from animation, AI targeting, physics, and future melee queries.
#[derive(Component)]
pub struct DefeatedCreature;

#[derive(Component)]
pub struct WildlifeAnimal {
    pub id: u32,
}

#[derive(Component)]
pub struct WildlifeAi {
    pub mood: AiMood,
    pub target: Vec3,
    pub phase: f32,
}

impl Default for WildlifeAi {
    fn default() -> Self {
        Self {
            mood: AiMood::Idle,
            target: Vec3::ZERO,
            phase: 0.0,
        }
    }
}

#[derive(Component)]
pub struct PlantNode {
    pub id: u32,
}

pub const FARM_PLOT_POSITIONS: [Vec3; 3] = [
    Vec3::new(-8.0, 0.0, 4.5),
    Vec3::new(-8.0, 0.0, 8.0),
    Vec3::new(-4.0, 0.0, 6.25),
];

#[derive(Component)]
pub struct FarmCropVisual {
    pub id: u32,
}

#[derive(Resource)]
pub struct FarmVisualMaterials {
    pub crop_materials: [Handle<StandardMaterial>; 3],
}

#[must_use]
pub const fn crop_material_index(kind: CropKind) -> usize {
    match kind {
        CropKind::Wheat => 0,
        CropKind::Carrot => 1,
        CropKind::Potato => 2,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WolfMood {
    Idle,
    Chase,
    Pounce,
}

impl Default for WolfMood {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Component, Default)]
pub struct WolfAi {
    pub mood: WolfMood,
    pub target: Vec3,
    pub run_phase: f32,
}

#[derive(Component)]
pub struct Cloud {
    pub snapshot_index: usize,
    pub phase: f32,
}

#[derive(Component)]
pub struct RainDrop {
    pub cloud_index: usize,
    pub local: Vec3,
    pub phase: f32,
}

#[derive(Component)]
pub struct SlashFx;

#[derive(Component)]
pub struct HitImpactFx {
    pub age: f32,
    pub lifetime: f32,
    pub scale: f32,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HitReaction {
    pub timer: f32,
    pub duration_secs: f32,
    /// A tiny impact hold keeps the target at the peak of the recoil long
    /// enough for the hit to read before the recovery begins.
    pub impact_hold_secs: f32,
    pub direction: Vec3,
    pub strength: f32,
}

impl HitReaction {
    pub fn trigger(&mut self, direction: Vec3, strength: f32) {
        self.trigger_with_settings(direction, strength, CreatureHitSettings::default());
    }

    pub fn trigger_with_settings(
        &mut self,
        direction: Vec3,
        strength: f32,
        settings: CreatureHitSettings,
    ) {
        self.timer = settings.reaction_secs.max(0.0);
        self.duration_secs = self.timer;
        self.impact_hold_secs = settings.impact_hold_secs.max(0.0);
        self.direction = direction.normalize_or_zero();
        self.strength = strength.max(0.0);
    }
}

/// Shared tuning for creature hit feedback. The same setting drives the
/// offline damage gate and the presentation IK recoil, so a blocked hit never
/// produces a false hit animation.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CreatureHitSettings {
    pub invulnerability_ticks: u32,
    pub reaction_secs: f32,
    pub impact_hold_secs: f32,
    pub ik_recoil: f32,
    pub ik_lift: f32,
}

impl Default for CreatureHitSettings {
    fn default() -> Self {
        Self {
            invulnerability_ticks: CREATURE_HIT_INVULNERABILITY_TICKS,
            reaction_secs: 0.22,
            impact_hold_secs: 0.045,
            ik_recoil: 0.12,
            ik_lift: 0.07,
        }
    }
}
