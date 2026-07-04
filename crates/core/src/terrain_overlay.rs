use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OverlayKind {
    Ditch,

    WoodWall,

    StoneWall,

    Bridge,

    Road,

    RuneSlot,

    Altar,
}

impl OverlayKind {
    pub fn id_str(self) -> &'static str {
        match self {
            Self::Ditch => "ditch",
            Self::WoodWall => "wood_wall",
            Self::StoneWall => "stone_wall",
            Self::Bridge => "bridge",
            Self::Road => "road",
            Self::RuneSlot => "rune_slot",
            Self::Altar => "altar",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Ditch => "沟",
            Self::WoodWall => "木墙",
            Self::StoneWall => "石墙",
            Self::Bridge => "桥",
            Self::Road => "道路",
            Self::RuneSlot => "符文槽",
            Self::Altar => "阵眼",
        }
    }

    pub fn default_durability(self) -> f32 {
        match self {
            Self::Ditch => f32::INFINITY,
            Self::WoodWall => 100.0,
            Self::StoneWall => 400.0,
            Self::Bridge => 150.0,
            Self::Road => f32::INFINITY,
            Self::RuneSlot => 200.0,
            Self::Altar => 80.0,
        }
    }

    pub fn blocks_movement(self) -> bool {
        match self {
            Self::Ditch => true,
            Self::WoodWall => true,
            Self::StoneWall => true,
            Self::Bridge => false,
            Self::Road => false,
            Self::RuneSlot => false,
            Self::Altar => false,
        }
    }

    pub fn is_destructible(self) -> bool {
        match self {
            Self::WoodWall | Self::StoneWall | Self::Bridge | Self::Altar => true,
            Self::Ditch | Self::Road | Self::RuneSlot => false,
        }
    }

    pub fn default_footprint_radius(self) -> f32 {
        match self {
            Self::Ditch => 0.6,
            Self::WoodWall => 0.5,
            Self::StoneWall => 0.5,
            Self::Bridge => 1.5,
            Self::Road => 1.0,
            Self::RuneSlot => 0.8,
            Self::Altar => 1.6,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct TerrainOverlayEntity {
    pub id: u32,
    pub kind: OverlayKind,

    pub world_pos: [f32; 3],

    pub yaw_radians: f32,

    pub footprint_radius: f32,

    pub durability: f32,

    pub owner: Option<u32>,
}

impl TerrainOverlayEntity {
    pub fn new(id: u32, kind: OverlayKind, world_pos: [f32; 3]) -> Self {
        Self {
            id,
            kind,
            world_pos,
            yaw_radians: 0.0,
            footprint_radius: kind.default_footprint_radius(),
            durability: kind.default_durability(),
            owner: None,
        }
    }

    pub fn is_destroyed(&self) -> bool {
        self.kind.is_destructible() && self.durability <= 0.0
    }
}

#[derive(Resource, Debug, Default, Clone)]
pub struct TerrainOverlayRegistry {
    pub entries: std::collections::HashMap<u32, [f32; 3]>,

    pub total_spawned: u32,
}

impl TerrainOverlayRegistry {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn register(&mut self, overlay: &TerrainOverlayEntity) -> bool {
        if self.entries.contains_key(&overlay.id) {
            return false;
        }
        self.entries.insert(overlay.id, overlay.world_pos);
        self.total_spawned += 1;
        true
    }

    pub fn unregister(&mut self, id: u32) -> bool {
        self.entries.remove(&id).is_some()
    }
}

#[derive(Message, Debug, Clone)]
pub struct OverlaySpawnedMsg {
    pub id: u32,
    pub kind: OverlayKind,
    pub world_pos: [f32; 3],
    pub yaw_radians: f32,
    pub owner: Option<u32>,
}

#[derive(Message, Debug, Clone)]
pub struct OverlayDamagedMsg {
    pub id: u32,
    pub new_durability: f32,
}

#[derive(Message, Debug, Clone)]
pub struct OverlayDestroyedMsg {
    pub id: u32,
    pub kind: OverlayKind,

    pub reason: OverlayDestroyReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayDestroyReason {
    Damaged,

    Deconstructed,

    OwnerLost,

    Expired,
}

pub struct TerrainOverlayPlugin;

impl Plugin for TerrainOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainOverlayRegistry>()
            .add_message::<OverlaySpawnedMsg>()
            .add_message::<OverlayDamagedMsg>()
            .add_message::<OverlayDestroyedMsg>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_id_str_matches_doc() {
        assert_eq!(OverlayKind::Ditch.id_str(), "ditch");
        assert_eq!(OverlayKind::WoodWall.id_str(), "wood_wall");
        assert_eq!(OverlayKind::StoneWall.id_str(), "stone_wall");
        assert_eq!(OverlayKind::Bridge.id_str(), "bridge");
        assert_eq!(OverlayKind::Road.id_str(), "road");
        assert_eq!(OverlayKind::RuneSlot.id_str(), "rune_slot");
        assert_eq!(OverlayKind::Altar.id_str(), "altar");
    }

    #[test]
    fn label_zh_covers_all_7_kinds() {
        let labels: Vec<&str> = [
            OverlayKind::Ditch,
            OverlayKind::WoodWall,
            OverlayKind::StoneWall,
            OverlayKind::Bridge,
            OverlayKind::Road,
            OverlayKind::RuneSlot,
            OverlayKind::Altar,
        ]
        .iter()
        .map(|k| k.label_zh())
        .collect();
        assert_eq!(labels.len(), 7);
        for l in &labels {
            assert!(!l.is_empty(), "label_zh 不能为空");

            assert!(
                l.chars().any(|c| c as u32 > 127),
                "label_zh 应是中文: {:?}",
                l
            );
        }
    }

    #[test]
    fn stone_wall_stronger_than_wood() {
        assert!(
            OverlayKind::StoneWall.default_durability()
                > OverlayKind::WoodWall.default_durability(),
            "石墙应比木墙耐久高"
        );
    }

    #[test]
    fn walls_block_but_road_bridge_dont() {
        assert!(OverlayKind::WoodWall.blocks_movement());
        assert!(OverlayKind::StoneWall.blocks_movement());
        assert!(OverlayKind::Ditch.blocks_movement());
        assert!(!OverlayKind::Road.blocks_movement());
        assert!(!OverlayKind::Bridge.blocks_movement());
        assert!(!OverlayKind::RuneSlot.blocks_movement());
    }

    #[test]
    fn only_4_kinds_destructible() {
        let destructible: Vec<OverlayKind> = [
            OverlayKind::Ditch,
            OverlayKind::WoodWall,
            OverlayKind::StoneWall,
            OverlayKind::Bridge,
            OverlayKind::Road,
            OverlayKind::RuneSlot,
            OverlayKind::Altar,
        ]
        .iter()
        .copied()
        .filter(|k| k.is_destructible())
        .collect();
        assert_eq!(destructible.len(), 4);
        assert!(destructible.contains(&OverlayKind::WoodWall));
        assert!(destructible.contains(&OverlayKind::StoneWall));
        assert!(destructible.contains(&OverlayKind::Bridge));
        assert!(destructible.contains(&OverlayKind::Altar));
    }

    #[test]
    fn new_overlay_uses_kind_defaults() {
        let o = TerrainOverlayEntity::new(42, OverlayKind::WoodWall, [10.0, 0.0, 5.0]);
        assert_eq!(o.id, 42);
        assert_eq!(o.kind, OverlayKind::WoodWall);
        assert_eq!(o.world_pos, [10.0, 0.0, 5.0]);
        assert!((o.durability - 100.0).abs() < 0.001);
        assert!(!o.is_destroyed());
    }

    #[test]
    fn destroyed_when_durability_zero() {
        let mut o = TerrainOverlayEntity::new(1, OverlayKind::Altar, [0.0, 0.0, 0.0]);
        o.durability = 0.0;
        assert!(o.is_destroyed());

        let mut r = TerrainOverlayEntity::new(2, OverlayKind::Road, [0.0, 0.0, 0.0]);
        r.durability = 0.0;
        assert!(
            !r.is_destroyed(),
            "Road 是不可破坏覆盖层,durability=0 也不应算 destroyed"
        );
        let _ = r;
    }

    #[test]
    fn registry_register_and_unregister() {
        let mut reg = TerrainOverlayRegistry::default();
        assert!(reg.is_empty());

        let o1 = TerrainOverlayEntity::new(1, OverlayKind::WoodWall, [0.0, 0.0, 0.0]);
        assert!(reg.register(&o1));
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.total_spawned, 1);

        let o1_dup = TerrainOverlayEntity::new(1, OverlayKind::StoneWall, [5.0, 0.0, 5.0]);
        assert!(!reg.register(&o1_dup));
        assert_eq!(reg.len(), 1);

        assert!(reg.unregister(1));
        assert_eq!(reg.len(), 0);

        assert!(!reg.unregister(1));
    }

    #[test]
    fn registry_total_spawned_monotonic() {
        let mut reg = TerrainOverlayRegistry::default();
        for i in 0..5 {
            let o = TerrainOverlayEntity::new(i, OverlayKind::Road, [i as f32, 0.0, 0.0]);
            reg.register(&o);
        }
        assert_eq!(reg.total_spawned, 5);

        reg.unregister(0);
        reg.unregister(1);
        assert_eq!(reg.total_spawned, 5);
        assert_eq!(reg.len(), 3);
    }

    #[test]
    fn all_kinds_have_positive_footprint_radius() {
        for k in [
            OverlayKind::Ditch,
            OverlayKind::WoodWall,
            OverlayKind::StoneWall,
            OverlayKind::Bridge,
            OverlayKind::Road,
            OverlayKind::RuneSlot,
            OverlayKind::Altar,
        ] {
            assert!(
                k.default_footprint_radius() > 0.0,
                "{:?} footprint 必须 > 0",
                k
            );
        }
    }
}
