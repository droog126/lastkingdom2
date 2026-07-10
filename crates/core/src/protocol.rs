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
    pub struct PingMessage {
        pub client_id: u64,
        pub sequence: u64,
        pub client_time_secs: f64,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct PongMessage {
        pub client_id: u64,
        pub sequence: u64,
        pub client_time_secs: f64,
        pub server_tick: u32,
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
    pub const ECO_SNAPSHOT_MAX_CLOUDS: usize = 8;

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

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
    pub struct EcoCloudNet {
        pub id: u32,
        pub x: f32,
        pub z: f32,
        pub rain: f32,
        pub phase: f32,
    }

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct EcoSnapshot {
        pub tick: u64,
        pub co2: f32,
        pub rain: f32,
        pub rainfall: f32,
        pub fruit_eaten: u64,
        pub fruit_grown: u64,
        pub plants_grown: u64,
        pub rabbits_born: u64,
        pub wildlife_born: u64,
        pub clouds: Vec<EcoCloudNet>,
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

    pub const VOXEL_CHUNK_SIZE_XZ: i32 = 16;

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct VoxelChunkSnapshot {
        pub revision: u64,
        pub chunk_x: i32,
        pub chunk_z: i32,
        pub y_min: i32,
        pub y_size: i32,
        pub blocks: Vec<u8>,
    }
}

/// Commands and acknowledgements that must arrive exactly and in order.
pub struct ControlChannel;

/// High-frequency snapshots where only the newest value is useful.
pub struct StateChannel;

fn control_channel_settings() -> lightyear::prelude::ChannelSettings {
    lightyear::prelude::ChannelSettings {
        mode: lightyear::prelude::ChannelMode::OrderedReliable(
            lightyear::prelude::ReliableSettings::default(),
        ),
        priority: 10.0,
        ..default()
    }
}

fn state_channel_settings() -> lightyear::prelude::ChannelSettings {
    lightyear::prelude::ChannelSettings {
        mode: lightyear::prelude::ChannelMode::SequencedUnreliable,
        priority: 1.0,
        ..default()
    }
}

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        use lightyear::prelude::*;

        app.init_resource::<MessageRegistry>();
        app.add_channel::<ControlChannel>(control_channel_settings())
            .add_direction(NetworkDirection::Bidirectional);
        app.add_channel::<StateChannel>(state_channel_settings())
            .add_direction(NetworkDirection::Bidirectional);

        // NOTE: do NOT add `lightyear_inputs_leafwing::prelude::InputPlugin`
        // here. Movement uses a raw UDP fast path (port = server_port + 1),
        // while discrete gameplay commands use MessageSender<GameplayCommand>.
        // Registering InputPlugin<PlayerAction> on the server
        // installs ServerInputPlugin, which adds a MessageReceiver<InputMessage<...>>
        // to each client entity and tries to deserialize every incoming packet
        // as an InputMessage. The client never sends lightyear InputMessages,
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
        app.register_message::<messages::PingMessage>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<messages::PongMessage>()
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
        app.component::<components::VoxelChunkSnapshot>().replicate();
    }
}

/// Wire-format helpers shared by client and server.
///
/// golab teaches the lesson that floats crossing the network need an explicit
/// `is_finite()` boundary — a single NaN sneaking through serde will corrupt
/// every downstream Bevy transform. These helpers expose the same check on
/// every wire type in `messages` / `components`, so the server can drop bad
/// packets before they hit authoritative state and the client can guard
/// replicated components before they touch render code.
pub mod wire_format {

    use super::components::{
        EcoBerryNet, EcoCloudNet, EcoPlantNet, EcoRabbitNet, EcoSnapshot, EcoWildlifeNet,
        PlayerPos, VoxelDelta,
    };
    use super::messages::{
        AttackInput, DamageResult, HitConfirm, KnockbackEvent, PingMessage, PongMessage,
        ServerPosUpdate,
    };

    /// True iff every component of `v` is a finite float (no NaN, no Infinity).
    #[must_use]
    pub fn is_finite_vec3(v: &bevy::prelude::Vec3) -> bool {
        v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
    }

    /// True iff `value` is a finite f32 (no NaN, no Infinity).
    #[must_use]
    pub fn is_finite_f32(value: f32) -> bool {
        value.is_finite()
    }

    /// True iff `value` is a finite f64 (no NaN, no Infinity).
    #[must_use]
    pub fn is_finite_f64(value: f64) -> bool {
        value.is_finite()
    }

    /// True iff every element of `slice` is a finite float.
    #[must_use]
    pub fn is_finite_f32_slice(slice: &[f32]) -> bool {
        slice.iter().all(|value| value.is_finite())
    }

    impl PlayerPos {
        /// True iff the inner position is finite (no NaN/Inf).
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_vec3(&self.0)
        }
    }

    impl ServerPosUpdate {
        /// True iff the wire-position payload is finite.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_vec3(&self.pos)
        }
    }

    impl AttackInput {
        /// True iff the input direction vector is finite.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_vec3(&self.input_dir)
        }
    }

    impl HitConfirm {
        /// True iff the world-space hit position and damage amount are both
        /// finite. A NaN damage value would propagate into the HUD HP bar
        /// and the knockback velocity — drop the message at the network
        /// boundary instead of letting it corrupt authoritative state.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_vec3(&self.hit_pos) && is_finite_f32(self.damage)
        }
    }

    impl KnockbackEvent {
        /// True iff the knockback velocity vector is finite. A NaN velocity
        /// would freeze the victim in place or send them to (0,0,0) every
        /// frame after reconciliation, so it must be filtered at the wire.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_vec3(&self.velocity)
        }
    }

    impl DamageResult {
        /// True iff the resulting health value is finite. The HUD reads this
        /// directly, so a NaN health makes the bar disappear or render as
        /// gibberish.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_f32(self.new_health)
        }
    }

    impl PingMessage {
        /// True iff the timestamp payload is finite.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            self.client_time_secs.is_finite()
        }
    }

    impl PongMessage {
        /// True iff the timestamp payload is finite.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            self.client_time_secs.is_finite()
        }
    }

    impl VoxelDelta {
        /// `VoxelDelta` carries only `u64` / `i32` / `u8` fields, which
        /// cannot be NaN or Inf. The method exists so every replicated
        /// component participates in the same finite-check contract; today
        /// it is a compile-time guarantee, not a runtime branch.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            true
        }
    }

    impl EcoRabbitNet {
        /// True iff x, z, and energy are finite. Positions and energy flow
        /// from server-side simulation; a NaN coordinate would teleport the
        /// rabbit to `(0, 0)` or render it inside a chunk each frame.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_f32(self.x) && is_finite_f32(self.z) && is_finite_f32(self.energy)
        }
    }

    impl EcoWildlifeNet {
        /// True iff x, z, and energy are finite.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_f32(self.x) && is_finite_f32(self.z) && is_finite_f32(self.energy)
        }
    }

    impl EcoBerryNet {
        /// True iff x and z are finite.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_f32(self.x) && is_finite_f32(self.z)
        }
    }

    impl EcoPlantNet {
        /// True iff x and z are finite.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_f32(self.x) && is_finite_f32(self.z)
        }
    }

    impl EcoCloudNet {
        /// True iff position, rain, and phase are finite. The cloud animator
        /// drives `rain` from accumulated precipitation; if a divider hits
        /// zero it can spill NaN into the loop and freeze the rain sprite.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_f32(self.x)
                && is_finite_f32(self.z)
                && is_finite_f32(self.rain)
                && is_finite_f32(self.phase)
        }
    }

    impl EcoSnapshot {
        /// True iff every per-tick float (co2, rain, rainfall) is finite.
        /// Counter fields are integers and cannot misbehave. Cloud/berry/
        /// plant/inner lists are validated by their own `is_finite()` and
        /// are not re-checked here — the call site can iterate `clouds`,
        /// `rabbits`, etc. independently.
        #[must_use]
        pub fn is_finite(&self) -> bool {
            is_finite_f32(self.co2) && is_finite_f32(self.rain) && is_finite_f32(self.rainfall)
        }
    }

    /// Build a `ServerPosUpdate` from a Bevy transform, returning `None`
    /// when the translation contains NaN or Infinity. Call at every server
    /// broadcast boundary so corrupted engine state never reaches the wire.
    ///
    /// golab does the same `if !is_finite(...) { skip }` check inline in
    /// `broadcast_snapshot`; promoting it to a free function keeps the call
    /// site tidy and lets the unit tests cover the contract once.
    #[must_use]
    pub fn try_make_server_pos_update(
        transform: &bevy::prelude::Transform,
        server_tick: u32,
    ) -> Option<ServerPosUpdate> {
        let pos = transform.translation;
        if !is_finite_vec3(&pos) {
            return None;
        }
        Some(ServerPosUpdate { server_tick, pos })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use bevy::prelude::Vec3;

        #[test]
        fn finite_vec3_passes() {
            assert!(is_finite_vec3(&Vec3::ZERO));
            assert!(is_finite_vec3(&Vec3::new(1.0, -2.5, 3.14)));
        }

        #[test]
        fn nan_or_inf_vec3_fails() {
            assert!(!is_finite_vec3(&Vec3::new(f32::NAN, 0.0, 0.0)));
            assert!(!is_finite_vec3(&Vec3::new(0.0, f32::INFINITY, 0.0)));
            assert!(!is_finite_vec3(&Vec3::new(0.0, 0.0, f32::NEG_INFINITY)));
        }

        #[test]
        fn finite_f32_slice_passes() {
            assert!(is_finite_f32_slice(&[1.0, 2.0, 3.0]));
            assert!(is_finite_f32_slice(&[]));
            assert!(!is_finite_f32_slice(&[1.0, f32::NAN]));
        }

        #[test]
        fn player_pos_is_finite_matches_inner() {
            assert!(PlayerPos(Vec3::new(1.0, 2.0, 3.0)).is_finite());
            assert!(!PlayerPos(Vec3::new(f32::NAN, 0.0, 0.0)).is_finite());
        }

        #[test]
        fn server_pos_update_is_finite_matches_inner() {
            let ok = ServerPosUpdate { server_tick: 1, pos: Vec3::new(0.5, 1.5, 2.5) };
            assert!(ok.is_finite());
            let bad = ServerPosUpdate { server_tick: 1, pos: Vec3::new(f32::INFINITY, 0.0, 0.0) };
            assert!(!bad.is_finite());
        }

        #[test]
        fn voxel_delta_is_always_finite() {
            let d = VoxelDelta { revision: 0, x: 0, y: 0, z: 0, block: 0 };
            assert!(d.is_finite());
        }

        #[test]
        fn ping_pong_finite_checks_timestamp() {
            assert!(PingMessage { client_id: 7, sequence: 1, client_time_secs: 1.0 }.is_finite());
            assert!(
                !PingMessage { client_id: 7, sequence: 1, client_time_secs: f64::NAN }.is_finite()
            );
            assert!(
                PongMessage { client_id: 7, sequence: 1, client_time_secs: 1.0, server_tick: 2 }
                    .is_finite()
            );
            assert!(
                !PongMessage {
                    client_id: 7,
                    sequence: 1,
                    client_time_secs: f64::INFINITY,
                    server_tick: 2,
                }
                .is_finite()
            );
        }

        #[test]
        fn try_make_server_pos_update_accepts_finite_transform() {
            let t = bevy::prelude::Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
            let msg = try_make_server_pos_update(&t, 42).expect("finite transform must succeed");
            assert_eq!(msg.server_tick, 42);
            assert_eq!(msg.pos, Vec3::new(1.0, 2.0, 3.0));
        }

        #[test]
        fn try_make_server_pos_update_rejects_nan_translation() {
            let t = bevy::prelude::Transform::from_translation(Vec3::new(f32::NAN, 0.0, 0.0));
            assert!(try_make_server_pos_update(&t, 1).is_none());
        }

        #[test]
        fn try_make_server_pos_update_rejects_infinity_translation() {
            let t = bevy::prelude::Transform::from_translation(Vec3::new(0.0, f32::INFINITY, 0.0));
            assert!(try_make_server_pos_update(&t, 1).is_none());
        }

        #[test]
        fn hit_confirm_is_finite_checks_position_and_damage() {
            use super::super::messages::HitConfirm;
            use lightyear::prelude::PeerId;
            let ok = HitConfirm {
                victim_id: PeerId::Netcode(1),
                damage: 20.0,
                is_critical: false,
                hit_pos: Vec3::new(1.0, 2.0, 3.0),
                server_tick: 0,
            };
            assert!(ok.is_finite());
            let bad_pos = HitConfirm { hit_pos: Vec3::new(f32::NAN, 0.0, 0.0), ..ok.clone() };
            assert!(!bad_pos.is_finite());
            let bad_damage = HitConfirm { damage: f32::INFINITY, ..ok };
            assert!(!bad_damage.is_finite());
        }

        #[test]
        fn knockback_event_is_finite_checks_velocity() {
            use super::super::messages::KnockbackEvent;
            use lightyear::prelude::PeerId;
            let ok = KnockbackEvent {
                victim_id: PeerId::Netcode(1),
                velocity: Vec3::new(1.0, 0.0, 0.0),
                server_tick: 0,
            };
            assert!(ok.is_finite());
            let bad = KnockbackEvent { velocity: Vec3::new(0.0, f32::INFINITY, 0.0), ..ok };
            assert!(!bad.is_finite());
        }

        #[test]
        fn damage_result_is_finite_checks_health() {
            use super::super::messages::DamageResult;
            use lightyear::prelude::PeerId;
            let ok = DamageResult {
                victim_id: PeerId::Netcode(1),
                new_health: 80.0,
                is_dead: false,
                server_tick: 0,
            };
            assert!(ok.is_finite());
            let bad = DamageResult { new_health: f32::NAN, ..ok };
            assert!(!bad.is_finite());
        }

        #[test]
        fn eco_rabbit_net_is_finite_checks_position_and_energy() {
            use super::super::components::EcoRabbitNet;
            let ok = EcoRabbitNet { id: 1, x: 0.0, z: 0.0, energy: 1.0 };
            assert!(ok.is_finite());
            let bad = EcoRabbitNet { x: f32::NAN, ..ok };
            assert!(!bad.is_finite());
            let bad_e = EcoRabbitNet {
                energy: f32::INFINITY,
                ..EcoRabbitNet { id: 2, x: 0.0, z: 0.0, energy: 1.0 }
            };
            assert!(!bad_e.is_finite());
        }

        #[test]
        fn eco_wildlife_net_is_finite_checks_position_and_energy() {
            use super::super::components::EcoWildlifeNet;
            let ok = EcoWildlifeNet { id: 1, kind: 0, x: 1.0, z: 2.0, energy: 0.5 };
            assert!(ok.is_finite());
            let bad = EcoWildlifeNet { z: f32::NAN, ..ok };
            assert!(!bad.is_finite());
        }

        #[test]
        fn eco_berry_net_is_finite_checks_position() {
            use super::super::components::EcoBerryNet;
            let ok = EcoBerryNet { id: 1, x: 0.0, z: 0.0, fruit: 5 };
            assert!(ok.is_finite());
            let bad = EcoBerryNet { x: f32::INFINITY, ..ok };
            assert!(!bad.is_finite());
        }

        #[test]
        fn eco_plant_net_is_finite_checks_position() {
            use super::super::components::EcoPlantNet;
            let ok = EcoPlantNet { id: 1, kind: 0, x: 0.0, z: 0.0, stock: 3 };
            assert!(ok.is_finite());
            let bad = EcoPlantNet { z: f32::NAN, ..ok };
            assert!(!bad.is_finite());
        }

        #[test]
        fn eco_cloud_net_is_finite_checks_rain_and_phase() {
            use super::super::components::EcoCloudNet;
            let ok = EcoCloudNet { id: 1, x: 0.0, z: 0.0, rain: 0.5, phase: 1.2 };
            assert!(ok.is_finite());
            let bad_rain = EcoCloudNet { rain: f32::NAN, ..ok };
            assert!(!bad_rain.is_finite());
            let bad_phase = EcoCloudNet {
                phase: f32::INFINITY,
                ..EcoCloudNet { id: 2, x: 0.0, z: 0.0, rain: 0.0, phase: 0.0 }
            };
            assert!(!bad_phase.is_finite());
        }

        #[test]
        fn eco_snapshot_is_finite_checks_aggregate_floats() {
            use super::super::components::{EcoCloudNet, EcoSnapshot};
            let ok = EcoSnapshot {
                tick: 0,
                co2: 400.0,
                rain: 0.0,
                rainfall: 0.1,
                fruit_eaten: 0,
                fruit_grown: 0,
                plants_grown: 0,
                rabbits_born: 0,
                wildlife_born: 0,
                clouds: Vec::new(),
                rabbits: Vec::new(),
                wildlife: Vec::new(),
                berries: Vec::new(),
                plants: Vec::new(),
            };
            assert!(ok.is_finite());
            let bad = EcoSnapshot { co2: f32::NAN, ..ok.clone() };
            assert!(!bad.is_finite());
            // A non-finite inner cloud invalidates the snapshot when iterated,
            // even though `is_finite()` only spot-checks the aggregate fields.
            let cloud_with_nan = EcoCloudNet { id: 1, x: 0.0, z: 0.0, rain: f32::NAN, phase: 0.0 };
            let snapshot_with_nan_cloud = EcoSnapshot { clouds: vec![cloud_with_nan], ..ok };
            assert!(
                snapshot_with_nan_cloud.is_finite(),
                "aggregate check ignores inner lists"
            );
            assert!(!snapshot_with_nan_cloud.clouds.iter().all(|c| c.is_finite()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn player_action_variants_count() {
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
        ]
        .len();
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
        assert!(matches!(
            GameplayCommandKind::Jump,
            GameplayCommandKind::Jump
        ));
        assert!(matches!(
            GameplayCommandKind::GatherFootBlock,
            GameplayCommandKind::GatherFootBlock
        ));
        assert!(matches!(
            GameplayCommandKind::PlaceWoodFootBlock,
            GameplayCommandKind::PlaceWoodFootBlock
        ));
        assert!(matches!(
            GameplayCommandKind::Craft(messages::BuildRecipe::PlankPack),
            GameplayCommandKind::Craft(messages::BuildRecipe::PlankPack)
        ));
        assert!(matches!(
            GameplayCommandKind::FoundNation,
            GameplayCommandKind::FoundNation
        ));
        assert!(matches!(
            GameplayCommandKind::KillNearestCreature,
            GameplayCommandKind::KillNearestCreature
        ));
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
    fn ping_pong_json_roundtrip() {
        let ping = messages::PingMessage { client_id: 9, sequence: 77, client_time_secs: 12.5 };
        let json = serde_json::to_string(&ping).unwrap();
        let decoded: messages::PingMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.client_id, 9);
        assert_eq!(decoded.sequence, 77);
        assert_eq!(decoded.client_time_secs, 12.5);

        let pong = messages::PongMessage {
            client_id: 9,
            sequence: 77,
            client_time_secs: 12.5,
            server_tick: 1234,
        };
        let json = serde_json::to_string(&pong).unwrap();
        let decoded: messages::PongMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.client_id, 9);
        assert_eq!(decoded.sequence, 77);
        assert_eq!(decoded.client_time_secs, 12.5);
        assert_eq!(decoded.server_tick, 1234);
    }

    #[test]
    fn gameplay_channels_keep_control_reliable_and_state_latest_only() {
        use lightyear::prelude::ChannelMode;

        assert!(matches!(
            control_channel_settings().mode,
            ChannelMode::OrderedReliable(_)
        ));
        assert!(matches!(
            state_channel_settings().mode,
            ChannelMode::SequencedUnreliable
        ));
        assert!(control_channel_settings().priority > state_channel_settings().priority);
    }

    #[test]
    fn kill_feed_entry_json_roundtrip() {
        use messages::KillFeedEntry;
        let entry =
            KillFeedEntry { killer_name: "Alice".into(), victim_name: "Bob".into(), weapon_id: 3 };
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
        assert_eq!(ECO_SNAPSHOT_MAX_CLOUDS, 8);
    }

    #[test]
    fn voxel_delta_fields() {
        use components::VoxelDelta;
        let d = VoxelDelta { revision: 123, x: 5, y: 10, z: -3, block: 7 };
        assert_eq!(d.revision, 123);
        assert_eq!(d.x, 5);
        assert_eq!(d.y, 10);
        assert_eq!(d.z, -3);
        assert_eq!(d.block, 7);
    }
}
