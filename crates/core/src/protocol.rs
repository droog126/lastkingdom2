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
        MoveWorld { dx_milli: i16, dz_milli: i16 },
        Jump,
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

    pub const ECO_SNAPSHOT_MAX_RABBITS: usize = 8;
    pub const ECO_SNAPSHOT_MAX_WILDLIFE: usize = 16;
    pub const ECO_SNAPSHOT_MAX_BERRIES: usize = 16;
    pub const ECO_SNAPSHOT_MAX_PLANTS: usize = 24;

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
    pub struct EcoRabbitNet {
        pub id: u32,
        pub x: f32,
        pub z: f32,
        pub energy: f32,
    }

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
    pub struct EcoWildlifeNet {
        pub id: u32,
        pub kind: u8,
        pub x: f32,
        pub z: f32,
        pub energy: f32,
    }

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
    pub struct EcoBerryNet {
        pub id: u32,
        pub x: f32,
        pub z: f32,
        pub fruit: u32,
    }

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
    pub struct EcoPlantNet {
        pub id: u32,
        pub kind: u8,
        pub x: f32,
        pub z: f32,
        pub stock: u32,
    }

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct EcoSnapshot {
        pub tick: u64,
        pub co2: f32,
        pub fruit_eaten: u64,
        pub fruit_grown: u64,
        pub rabbits: Vec<EcoRabbitNet>,
        pub wildlife: Vec<EcoWildlifeNet>,
        pub berries: Vec<EcoBerryNet>,
        pub plants: Vec<EcoPlantNet>,
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

        // NOTE: do NOT add `lightyear_inputs_leafwing::prelude::InputPlugin`
        // here. We send gameplay commands over a raw UDP socket
        // (port = server_port + 1) instead of via lightyear's InputMessage
        // channel. Registering InputPlugin<PlayerAction> on the server
        // installs ServerInputPlugin, which adds a MessageReceiver<InputMessage<...>>
        // to each client entity and tries to deserialize every incoming packet
        // as an InputMessage. The client never sends lightyear InputMessages
        // (its `MessageWriter<GameplayCommand>` only writes GameplayCommand),
        // so every packet is misinterpreted, the embedded `InputTarget::Entity`
        // references deserialize as 0 (PLACEHOLDER) and the server logs
        // "Attempting to deserialize an invalid entity." ~5ms — flooding
        // the log without affecting gameplay. Removing InputPlugin from the
        // shared ProtocolPlugin drops that receiver and silences the storm.
        // The client collects ActionState<PlayerAction> manually in
        // `collect_keys_to_action_state`, so no leafwing InputManagerPlugin
        // is required there either.

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
        app.component::<components::EcoSnapshot>().replicate();
        app.component::<components::VoxelDelta>().replicate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn player_action_variants_count() {
        use strum::IntoEnumIterator;
        let count = [
            PlayerAction::MoveForward,
            PlayerAction::MoveBackward,
            PlayerAction::MoveLeft,
            PlayerAction::MoveRight,
            PlayerAction::Jump,
            PlayerAction::Sprint,
            PlayerAction::Attack,
            PlayerAction::Block,
            PlayerAction::Gather,
            PlayerAction::Place,
            PlayerAction::Craft,
            PlayerAction::FoundNation,
            PlayerAction::KillCreature,
        ].len();
        assert_eq!(count, 13);
    }

    #[test]
    fn build_recipe_variants() {
        use messages::BuildRecipe;
        assert_eq!(BuildRecipe::PlankPack as u8, 0);
        assert_eq!(BuildRecipe::Campfire as u8, 1);
    }

    #[test]
    fn gameplay_command_kind_variants() {
        use messages::GameplayCommandKind;
        let move_cmd = GameplayCommandKind::MoveWorld { dx_milli: 100, dz_milli: -50 };
        match move_cmd {
            GameplayCommandKind::MoveWorld { dx_milli, dz_milli } => {
                assert_eq!(dx_milli, 100);
                assert_eq!(dz_milli, -50);
            }
            _ => panic!("expected MoveWorld"),
        }
        assert!(matches!(GameplayCommandKind::Jump, GameplayCommandKind::Jump));
        assert!(matches!(GameplayCommandKind::GatherFootBlock, GameplayCommandKind::GatherFootBlock));
        assert!(matches!(GameplayCommandKind::PlaceWoodFootBlock, GameplayCommandKind::PlaceWoodFootBlock));
        assert!(matches!(
            GameplayCommandKind::Craft(messages::BuildRecipe::PlankPack),
            GameplayCommandKind::Craft(messages::BuildRecipe::PlankPack)
        ));
        assert!(matches!(GameplayCommandKind::FoundNation, GameplayCommandKind::FoundNation));
        assert!(matches!(GameplayCommandKind::KillNearestCreature, GameplayCommandKind::KillNearestCreature));
    }

    #[test]
    fn attack_input_json_roundtrip() {
        use messages::AttackInput;
        let input = AttackInput {
            tick: 42,
            input_dir: Vec3::new(1.0, 0.0, 0.0),
            is_falling: true,
            combo_count: 3,
        };
        let json = serde_json::to_string(&input).unwrap();
        let decoded: AttackInput = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.tick, 42);
        assert_eq!(decoded.is_falling, true);
        assert_eq!(decoded.combo_count, 3);
        assert!((decoded.input_dir.x - 1.0).abs() < 0.001);
    }

    #[test]
    fn gameplay_command_json_roundtrip() {
        use messages::{GameplayCommand, GameplayCommandKind};
        let cmd = GameplayCommand {
            tick: 1000,
            player_block: [16, 8, 32],
            kind: GameplayCommandKind::FoundNation,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let decoded: GameplayCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.tick, 1000);
        assert_eq!(decoded.player_block, [16, 8, 32]);
        assert!(matches!(decoded.kind, GameplayCommandKind::FoundNation));
    }

    #[test]
    fn kill_feed_entry_json_roundtrip() {
        use messages::KillFeedEntry;
        let entry = KillFeedEntry {
            killer_name: "Alice".into(),
            victim_name: "Bob".into(),
            weapon_id: 3,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let decoded: KillFeedEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.killer_name, "Alice");
        assert_eq!(decoded.victim_name, "Bob");
        assert_eq!(decoded.weapon_id, 3);
    }

    #[test]
    fn components_wrap_values() {
        use components::*;
        assert_eq!(Health(100.0).0, 100.0);
        assert_eq!(EquippedWeapon(2).0, 2);
        assert_eq!(KnockbackImmunity(0.5).0, 0.5);
        assert_eq!(MonsterKind(1).0, 1);
        assert_eq!(MonsterHealth(50.0).0, 50.0);
        assert_eq!(PlayerPos(Vec3::ONE).0, Vec3::ONE);
        assert_eq!(PlayerRot(90.0).0, 90.0);
    }

    #[test]
    fn eco_snapshot_constants_are_reasonable() {
        use components::*;
        assert_eq!(ECO_SNAPSHOT_MAX_RABBITS, 8);
        assert_eq!(ECO_SNAPSHOT_MAX_WILDLIFE, 16);
        assert_eq!(ECO_SNAPSHOT_MAX_BERRIES, 16);
        assert_eq!(ECO_SNAPSHOT_MAX_PLANTS, 24);
    }

    #[test]
    fn voxel_delta_fields() {
        use components::VoxelDelta;
        let d = VoxelDelta {
            revision: 123,
            x: 5,
            y: 10,
            z: -3,
            block: 7,
        };
        assert_eq!(d.revision, 123);
        assert_eq!(d.x, 5);
        assert_eq!(d.y, 10);
        assert_eq!(d.z, -3);
        assert_eq!(d.block, 7);
    }
}
