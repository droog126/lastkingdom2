use bevy::prelude::*;
use leafwing_input_manager::Actionlike;
use serde::{Deserialize, Serialize};

#[derive(Reflect, Actionlike, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Sprint,
    Attack,
    Block,
    Gather,
    Place,
    Craft,
    FoundNation,
    KillCreature,
}

pub mod messages {

    use super::*;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct AttackInput {
        pub tick: u32,
        pub input_dir: Vec3,
        pub is_falling: bool,
        pub combo_count: u8,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct HitConfirm {
        pub victim_id: lightyear::prelude::PeerId,
        pub damage: f32,
        pub is_critical: bool,
        pub hit_pos: Vec3,
        pub server_tick: u32,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct KnockbackEvent {
        pub victim_id: lightyear::prelude::PeerId,
        pub velocity: Vec3,
        pub server_tick: u32,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct DamageResult {
        pub victim_id: lightyear::prelude::PeerId,
        pub new_health: f32,
        pub is_dead: bool,
        pub server_tick: u32,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct KillFeedEntry {
        pub killer_name: String,
        pub victim_name: String,
        pub weapon_id: u8,
    }

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
    pub enum BuildRecipe {
        PlankPack,
        Campfire,
    }

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
    pub enum GameplayCommandKind {
        GatherFootBlock,
        PlaceWoodFootBlock,
        Craft(BuildRecipe),
        FoundNation,
        KillNearestCreature,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct GameplayCommand {
        pub tick: u64,
        pub player_block: [i32; 3],
        pub kind: GameplayCommandKind,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct GameplayFeedback {
        pub ok: bool,
        pub summary: String,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct ServerPosUpdate {
        pub server_tick: u32,
        pub pos: Vec3,
    }
}

pub mod components {

    use super::*;

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    #[component(storage = "SparseSet")]
    pub struct Health(pub f32);

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct WeaponStatsRaw {
        pub reach: f32,
        pub damage: f32,
        pub knockback: f32,
        pub attack_speed: f32,
        pub sweep_angle_deg: f32,
    }

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct EquippedWeapon(pub u8);

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct CombatReady {
        pub weapon_id: u8,
        pub reach: f32,
        pub damage: f32,
        pub knockback: f32,
        pub attack_speed: f32,
        pub sweep_angle_deg: f32,
        pub attack_cooldown: f32,
    }

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct KnockbackImmunity(pub f32);

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct PlayerPos(pub Vec3);

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct PlayerRot(pub f32);

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct MonsterKind(pub u8);

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct MonsterHealth(pub f32);

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct GameplayHudState {
        pub tick: u64,
        pub player_block_pos: [i32; 3],
        pub player_pos: [f32; 3],
        pub nation_id: Option<u32>,
        pub monsters_killed: u32,
        pub blocks_gathered: u32,
        pub nations_founded: u32,
        pub inventory_wood: i64,
        pub inventory_food: i64,
        pub inventory_apple: i64,
        pub inventory_soul: i64,
        pub pool_wood: i64,
        pub pool_food: i64,
        pub pool_apple: i64,
        pub pool_soul: i64,
        pub flag_count: u32,
        pub total_nations: u32,
        pub monster_count: u32,
        pub observer_anomalies: u64,
        pub observer_invariant_violations: u64,
        pub status_line: String,
    }

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct VoxelDelta {
        pub revision: u64,
        pub x: i32,
        pub y: i32,
        pub z: i32,
        pub block: u8,
    }
}

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        use lightyear::prelude::*;

        app.init_resource::<MessageRegistry>();

        app.add_plugins(lightyear_inputs_leafwing::prelude::InputPlugin::<
            PlayerAction,
        >::default());

        app.register_message::<messages::AttackInput>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<messages::GameplayCommand>()
            .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<messages::HitConfirm>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<messages::KnockbackEvent>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<messages::DamageResult>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<messages::KillFeedEntry>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<messages::GameplayFeedback>()
            .add_direction(NetworkDirection::ServerToClient);

        app.register_message::<messages::ServerPosUpdate>()
            .add_direction(NetworkDirection::ServerToClient);

        app.component::<components::Health>().replicate();
        app.component::<components::WeaponStatsRaw>().replicate();
        app.component::<components::EquippedWeapon>().replicate();
        app.component::<components::CombatReady>().replicate();
        app.component::<components::KnockbackImmunity>().replicate();
        app.component::<components::PlayerPos>().replicate();
        app.component::<components::PlayerRot>().replicate();
        app.component::<components::MonsterKind>().replicate();
        app.component::<components::MonsterHealth>().replicate();
        app.component::<components::GameplayHudState>().replicate();
        app.component::<components::VoxelDelta>().replicate();
    }
}
