//! Bevy resources and components shared by the living forest scene.

use std::path::PathBuf;
use std::time::Instant;

use bevy::prelude::*;

#[derive(Resource)]
pub struct LivingSceneState {
    pub elapsed: f32,
    pub frame: u64,
    pub auto_shot: bool,
    pub shot_requested: bool,
    pub exit_deadline: Option<Instant>,
    pub png_path: PathBuf,
    pub attack_flash: f32,
    pub auto_demo: bool,
    pub iter_dir: Option<PathBuf>,
    pub frame_dt_over_50ms: u64,
    pub frame_dt_max_ms: f32,
}

#[derive(Resource)]
pub struct SceneMaterials {
    pub ground: Handle<StandardMaterial>,
    pub slash_mesh: Handle<Mesh>,
    pub slash_material: Handle<StandardMaterial>,
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

#[derive(Component, Default)]
pub struct PlayerMotion {
    pub planar_velocity: Vec3,
    pub planar_speed: f32,
    pub smoothed_speed: f32,
    pub stride_phase: f32,
    pub airborne_time: f32,
    pub grounded: bool,
    pub moving: bool,
}

#[derive(Component, Default)]
pub struct PlayerJump {
    pub vertical_velocity: f32,
}

#[derive(Component)]
pub struct BossActor {
    pub base: Vec3,
}

#[derive(Component)]
pub struct LivingSceneCamera;

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
