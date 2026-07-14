//! Bevy resources and components shared by the living forest scene.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use avian3d::prelude::PhysicsGizmos;
use bevy::prelude::*;
use lk2_core::farming::CropKind;
use lk2_core::legendary::LegendaryWeapon;
use lk2_core::world::terrain::{self, LandformModule, TerrainPipeline};

use super::creature_ai::AiMood;
use super::stylized_material::StylizedTerrainMaterial;

pub const PROCEDURAL_TERRAIN_CENTER: i32 = 48;
pub const PROCEDURAL_TERRAIN_RADIUS: i32 = 64;
const PROCEDURAL_TERRAIN_REFERENCE_SURFACE: f32 = 20.0;
const PROCEDURAL_TERRAIN_HEIGHT_SCALE: f32 = 0.14;
const PROCEDURAL_TERRAIN_SEA_LEVEL: f32 = 12.0;
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

#[derive(Resource, Default)]
pub struct TerrainRebuildState {
    pub requested: bool,
}

#[derive(Resource, Default)]
pub struct TerrainUndergroundState {
    pub requested: bool,
    pub chunk: Option<lk2_core::world::TerrainChunkCoord>,
    pub target: Option<[i32; 3]>,
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

        // Match SurfaceMeshData::push_quad: the rendered quad is split on the
        // same diagonal, so collision and visible terrain stay on one plane.
        if tz <= tx {
            h00 + tx * (h10 - h00) + tz * (h11 - h10)
        } else {
            h00 + tz * (h01 - h00) + tx * (h11 - h01)
        }
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

    fn base_surface_at_world(&self, x: i32, z: i32) -> f32 {
        self.pipeline
            .surface_f32(x, z)
            .unwrap_or(PROCEDURAL_TERRAIN_SEA_LEVEL + 1.0)
    }

    fn deformed_surface_at_world(&self, x: f32, z: f32, base_surface: f32) -> f32 {
        self.deformations.iter().fold(base_surface, |surface, edit| {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    FirstPerson,
    ThirdPerson,
}

impl CameraMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FirstPerson => "FirstPerson",
            Self::ThirdPerson => "ThirdPerson",
        }
    }
}

#[derive(Resource)]
pub struct LivingCameraRig {
    pub mode: CameraMode,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for LivingCameraRig {
    fn default() -> Self {
        Self {
            mode: CameraMode::FirstPerson,
            yaw: std::f32::consts::PI,
            pitch: -0.08,
        }
    }
}

#[derive(Component)]
pub struct PlayerActor;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerIkPartKind {
    Torso,
    Pants,
    Neck,
    Head,
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
    Basket,
    Stick,
}

#[derive(Component)]
pub struct PlayerIkPart {
    pub kind: PlayerIkPartKind,
}

#[derive(Component)]
pub struct DragonKatanaPickup;

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
    Vec3::new(-5.0, 0.0, 7.0),
    Vec3::new(0.0, 0.0, 7.0),
    Vec3::new(5.0, 0.0, 7.0),
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
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HitReaction {
    pub timer: f32,
    pub direction: Vec3,
    pub strength: f32,
}

impl HitReaction {
    pub fn trigger(&mut self, direction: Vec3, strength: f32) {
        self.timer = 0.22;
        self.direction = direction.normalize_or_zero();
        self.strength = strength.max(0.0);
    }
}
