use bevy::prelude::Component;
use bevy::prelude::Vec3;

use crate::creature::CreatureKind;
use crate::resource::ResourceKind;
use crate::world::{Biome, BlockType};

pub mod nature;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EcologyKind {
    Creature(CreatureKind),
    Wildlife(WildlifeKind),
    Tree(TreeKind),
    ResourceNode(ResourceNodeKind),
    ResourceDrop(ResourceDropKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WildlifeKind {
    Rabbit,
    Deer,
    Fox,
    Bear,
    Wolf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TreeKind {
    Sokpop,
    FallenStick,
    Palm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceNodeKind {
    BerryBush,
    MushroomRed,
    MushroomBrown,
    Flower,
    RockMid,
    RockMoss,
    SunstoneCrystal,
    FrostCrystal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceDropKind {
    BerryFruit,
    Wood,
    Stone,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcologyCatalogEntry {
    pub kind: EcologyKind,
    pub model_path: &'static str,
    pub visual_scale: Vec3,
    pub produced_resource: Option<ResourceKind>,
    pub preferred_biome: Option<Biome>,
    pub source_block: Option<BlockType>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcologyEntity {
    pub kind: EcologyKind,
    pub block_pos: [i32; 3],
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct EcologyModel {
    pub path: &'static str,
    pub scale: Vec3,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarvestYield {
    pub kind: ResourceKind,
    pub amount: i64,
}

pub fn ecology_components(
    kind: EcologyKind,
    block_pos: [i32; 3],
) -> (EcologyEntity, EcologyModel, Option<HarvestYield>) {
    let entry = ecology_entry(kind);
    (
        EcologyEntity { kind, block_pos },
        EcologyModel { path: entry.model_path, scale: entry.visual_scale },
        entry.produced_resource.map(|resource| HarvestYield { kind: resource, amount: 1 }),
    )
}

pub const ECOLOGY_CATALOG: &[EcologyCatalogEntry] = &[
    EcologyCatalogEntry {
        kind: EcologyKind::Wildlife(WildlifeKind::Rabbit),
        model_path: "procedural/eco/rabbit.glb",
        visual_scale: Vec3::splat(0.48),
        produced_resource: None,
        preferred_biome: Some(Biome::Jungle),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Wildlife(WildlifeKind::Deer),
        model_path: "kenney/curated/animals/kenney_deer.glb",
        visual_scale: Vec3::splat(0.85),
        produced_resource: None,
        preferred_biome: Some(Biome::Jungle),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Wildlife(WildlifeKind::Fox),
        model_path: "kenney/curated/animals/kenney_fox.glb",
        visual_scale: Vec3::splat(0.80),
        produced_resource: None,
        preferred_biome: Some(Biome::Tundra),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Wildlife(WildlifeKind::Bear),
        model_path: "procedural/pretty/bear.glb",
        visual_scale: Vec3::splat(0.85),
        produced_resource: None,
        preferred_biome: Some(Biome::Tundra),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Wildlife(WildlifeKind::Wolf),
        model_path: "procedural/pretty/wolf.glb",
        visual_scale: Vec3::splat(0.70),
        produced_resource: None,
        preferred_biome: Some(Biome::Tundra),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Creature(CreatureKind::Pig),
        model_path: "animals/pig.glb",
        visual_scale: Vec3::splat(0.34),
        produced_resource: Some(ResourceKind::Food),
        preferred_biome: Some(Biome::Jungle),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Creature(CreatureKind::Sheep),
        model_path: "animals/sheep.glb",
        visual_scale: Vec3::splat(0.34),
        produced_resource: Some(ResourceKind::Food),
        preferred_biome: Some(Biome::Tundra),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Creature(CreatureKind::Cow),
        model_path: "animals/cow.glb",
        visual_scale: Vec3::splat(0.38),
        produced_resource: Some(ResourceKind::Food),
        preferred_biome: Some(Biome::Jungle),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Creature(CreatureKind::Chicken),
        model_path: "animals/chicken.glb",
        visual_scale: Vec3::splat(0.30),
        produced_resource: Some(ResourceKind::Apple),
        preferred_biome: Some(Biome::Jungle),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Tree(TreeKind::Sokpop),
        model_path: "procedural/pretty/sokpop_tree.glb",
        visual_scale: Vec3::splat(0.72),
        produced_resource: Some(ResourceKind::Wood),
        preferred_biome: Some(Biome::Jungle),
        source_block: Some(BlockType::Wood),
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Tree(TreeKind::FallenStick),
        model_path: "procedural/pretty/fallen_stick.glb",
        visual_scale: Vec3::splat(0.62),
        produced_resource: Some(ResourceKind::Wood),
        preferred_biome: None,
        source_block: Some(BlockType::Wood),
    },
    EcologyCatalogEntry {
        kind: EcologyKind::Tree(TreeKind::Palm),
        model_path: "kenney/curated/coastal_and_pirate/kenney_palm_straight.glb",
        visual_scale: Vec3::splat(1.0),
        produced_resource: Some(ResourceKind::Wood),
        preferred_biome: Some(Biome::Desert),
        source_block: Some(BlockType::Wood),
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceNode(ResourceNodeKind::BerryBush),
        model_path: "procedural/eco/berry_bush.glb",
        visual_scale: Vec3::splat(0.82),
        produced_resource: Some(ResourceKind::Apple),
        preferred_biome: Some(Biome::Jungle),
        source_block: Some(BlockType::BerryThicket),
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceNode(ResourceNodeKind::MushroomRed),
        model_path: "procedural/pretty/mushroom_red.glb",
        visual_scale: Vec3::splat(0.58),
        produced_resource: Some(ResourceKind::Food),
        preferred_biome: Some(Biome::Jungle),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceNode(ResourceNodeKind::MushroomBrown),
        model_path: "procedural/pretty/mushroom_brown.glb",
        visual_scale: Vec3::splat(0.58),
        produced_resource: Some(ResourceKind::Food),
        preferred_biome: Some(Biome::Jungle),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceNode(ResourceNodeKind::Flower),
        model_path: "procedural/pretty/flower_0.glb",
        visual_scale: Vec3::splat(0.46),
        produced_resource: None,
        preferred_biome: Some(Biome::Jungle),
        source_block: None,
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceNode(ResourceNodeKind::RockMid),
        model_path: "procedural/pretty/rock_mid.glb",
        visual_scale: Vec3::splat(0.82),
        produced_resource: Some(ResourceKind::Wood),
        preferred_biome: None,
        source_block: Some(BlockType::Stone),
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceNode(ResourceNodeKind::RockMoss),
        model_path: "procedural/pretty/rock_moss.glb",
        visual_scale: Vec3::splat(0.82),
        produced_resource: Some(ResourceKind::LivingRoot),
        preferred_biome: Some(Biome::Jungle),
        source_block: Some(BlockType::LivingRoot),
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceNode(ResourceNodeKind::SunstoneCrystal),
        model_path: "procedural/pretty/crystal_pink.glb",
        visual_scale: Vec3::splat(0.9),
        produced_resource: Some(ResourceKind::Sunstone),
        preferred_biome: Some(Biome::Desert),
        source_block: Some(BlockType::SunstoneOre),
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceNode(ResourceNodeKind::FrostCrystal),
        model_path: "procedural/pretty/crystal_blue.glb",
        visual_scale: Vec3::splat(0.9),
        produced_resource: Some(ResourceKind::Frostcore),
        preferred_biome: Some(Biome::Tundra),
        source_block: Some(BlockType::FrostcoreOre),
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceDrop(ResourceDropKind::BerryFruit),
        model_path: "procedural/eco/berry_fruit.glb",
        visual_scale: Vec3::splat(0.52),
        produced_resource: Some(ResourceKind::Apple),
        preferred_biome: Some(Biome::Jungle),
        source_block: Some(BlockType::BerryThicket),
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceDrop(ResourceDropKind::Wood),
        model_path: "kenney/curated/survival_props/kenney_resource_wood.glb",
        visual_scale: Vec3::splat(0.8),
        produced_resource: Some(ResourceKind::Wood),
        preferred_biome: None,
        source_block: Some(BlockType::Wood),
    },
    EcologyCatalogEntry {
        kind: EcologyKind::ResourceDrop(ResourceDropKind::Stone),
        model_path: "kenney/curated/survival_props/kenney_resource_stone.glb",
        visual_scale: Vec3::splat(0.8),
        produced_resource: Some(ResourceKind::Wood),
        preferred_biome: None,
        source_block: Some(BlockType::Stone),
    },
];

pub fn ecology_entry(kind: EcologyKind) -> &'static EcologyCatalogEntry {
    ECOLOGY_CATALOG
        .iter()
        .find(|entry| entry.kind == kind)
        .unwrap_or_else(|| panic!("missing ecology catalog entry for {kind:?}"))
}

impl CreatureKind {
    pub fn ecology_entry(self) -> &'static EcologyCatalogEntry {
        ecology_entry(EcologyKind::Creature(self))
    }
}

#[cfg(test)]
mod smoke_tests {
    use super::*;

    #[test]
    fn ecology_kind_variants() {
        assert!(matches!(
            EcologyKind::Creature(CreatureKind::Pig),
            EcologyKind::Creature(_)
        ));
        assert!(matches!(
            EcologyKind::Wildlife(WildlifeKind::Rabbit),
            EcologyKind::Wildlife(_)
        ));
        assert!(matches!(
            EcologyKind::Tree(TreeKind::Sokpop),
            EcologyKind::Tree(_)
        ));
        assert!(matches!(
            EcologyKind::ResourceNode(ResourceNodeKind::BerryBush),
            EcologyKind::ResourceNode(_)
        ));
        assert!(matches!(
            EcologyKind::ResourceDrop(ResourceDropKind::Wood),
            EcologyKind::ResourceDrop(_)
        ));
    }

    #[test]
    fn ecology_entry_returns_correct_entry() {
        let entry = ecology_entry(EcologyKind::Creature(CreatureKind::Pig));
        assert_eq!(entry.kind, EcologyKind::Creature(CreatureKind::Pig));
        assert_eq!(entry.produced_resource, Some(ResourceKind::Food));
        assert_eq!(entry.preferred_biome, Some(Biome::Jungle));

        let entry = ecology_entry(EcologyKind::Tree(TreeKind::Sokpop));
        assert_eq!(entry.kind, EcologyKind::Tree(TreeKind::Sokpop));
        assert_eq!(entry.produced_resource, Some(ResourceKind::Wood));
    }

    #[test]
    fn ecology_components_returns_all_parts() {
        let (entity, model, yield_) =
            ecology_components(EcologyKind::Tree(TreeKind::Sokpop), [10, 5, 10]);
        assert_eq!(entity.kind, EcologyKind::Tree(TreeKind::Sokpop));
        assert_eq!(entity.block_pos, [10, 5, 10]);
        assert_eq!(model.path, "procedural/pretty/sokpop_tree.glb");
        assert!(yield_.is_some());
        assert_eq!(yield_.unwrap().kind, ResourceKind::Wood);
    }

    #[test]
    fn ecology_entity_eq_hash() {
        let e1 = EcologyEntity {
            kind: EcologyKind::Creature(CreatureKind::Pig),
            block_pos: [10, 5, 10],
        };
        let e2 = EcologyEntity {
            kind: EcologyKind::Creature(CreatureKind::Pig),
            block_pos: [10, 5, 10],
        };
        let e3 = EcologyEntity {
            kind: EcologyKind::Creature(CreatureKind::Sheep),
            block_pos: [10, 5, 10],
        };

        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    #[test]
    fn harvest_yield_fields() {
        let hy = HarvestYield { kind: ResourceKind::Wood, amount: 5 };
        assert_eq!(hy.kind, ResourceKind::Wood);
        assert_eq!(hy.amount, 5);
    }

    #[test]
    fn ecology_model_fields() {
        let m = EcologyModel { path: "test.glb", scale: Vec3::splat(2.0) };
        assert_eq!(m.path, "test.glb");
        assert_eq!(m.scale, Vec3::splat(2.0));
    }

    #[test]
    fn wildlife_kind_variants() {
        let _ = WildlifeKind::Rabbit;
        let _ = WildlifeKind::Deer;
        let _ = WildlifeKind::Fox;
        let _ = WildlifeKind::Bear;
        let _ = WildlifeKind::Wolf;
    }

    #[test]
    fn tree_kind_variants() {
        let _ = TreeKind::Sokpop;
        let _ = TreeKind::FallenStick;
        let _ = TreeKind::Palm;
    }

    #[test]
    fn resource_node_kind_variants() {
        let _ = ResourceNodeKind::BerryBush;
        let _ = ResourceNodeKind::MushroomRed;
        let _ = ResourceNodeKind::MushroomBrown;
        let _ = ResourceNodeKind::Flower;
        let _ = ResourceNodeKind::RockMid;
        let _ = ResourceNodeKind::RockMoss;
        let _ = ResourceNodeKind::SunstoneCrystal;
        let _ = ResourceNodeKind::FrostCrystal;
    }

    #[test]
    fn resource_drop_kind_variants() {
        let _ = ResourceDropKind::BerryFruit;
        let _ = ResourceDropKind::Wood;
        let _ = ResourceDropKind::Stone;
    }
}

impl WildlifeKind {
    pub const fn to_u8(self) -> u8 {
        match self {
            WildlifeKind::Rabbit => 0,
            WildlifeKind::Deer => 1,
            WildlifeKind::Fox => 2,
            WildlifeKind::Bear => 3,
            WildlifeKind::Wolf => 4,
        }
    }

    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => WildlifeKind::Deer,
            2 => WildlifeKind::Fox,
            3 => WildlifeKind::Bear,
            4 => WildlifeKind::Wolf,
            _ => WildlifeKind::Rabbit,
        }
    }
}

impl ResourceNodeKind {
    pub const fn to_u8(self) -> u8 {
        match self {
            ResourceNodeKind::BerryBush => 0,
            ResourceNodeKind::MushroomRed => 1,
            ResourceNodeKind::MushroomBrown => 2,
            ResourceNodeKind::Flower => 3,
            ResourceNodeKind::RockMid => 4,
            ResourceNodeKind::RockMoss => 5,
            ResourceNodeKind::SunstoneCrystal => 6,
            ResourceNodeKind::FrostCrystal => 7,
        }
    }

    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => ResourceNodeKind::MushroomRed,
            2 => ResourceNodeKind::MushroomBrown,
            3 => ResourceNodeKind::Flower,
            4 => ResourceNodeKind::RockMid,
            5 => ResourceNodeKind::RockMoss,
            6 => ResourceNodeKind::SunstoneCrystal,
            7 => ResourceNodeKind::FrostCrystal,
            _ => ResourceNodeKind::BerryBush,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::creature_drop_kind;

    #[test]
    fn catalog_has_entries_for_all_creature_kinds() {
        for kind in [
            CreatureKind::Pig,
            CreatureKind::Sheep,
            CreatureKind::Cow,
            CreatureKind::Chicken,
        ] {
            let entry = kind.ecology_entry();
            assert_eq!(entry.kind, EcologyKind::Creature(kind));
            assert_eq!(entry.produced_resource, Some(creature_drop_kind(kind)));
            assert!(entry.model_path.ends_with(".glb"));
        }
    }

    #[test]
    fn catalog_has_visible_wildlife_and_plant_variety() {
        for kind in [
            WildlifeKind::Rabbit,
            WildlifeKind::Deer,
            WildlifeKind::Fox,
            WildlifeKind::Bear,
            WildlifeKind::Wolf,
        ] {
            assert!(ecology_entry(EcologyKind::Wildlife(kind)).model_path.ends_with(".glb"));
        }

        for kind in [
            ResourceNodeKind::BerryBush,
            ResourceNodeKind::MushroomRed,
            ResourceNodeKind::MushroomBrown,
            ResourceNodeKind::Flower,
            ResourceNodeKind::RockMid,
            ResourceNodeKind::RockMoss,
            ResourceNodeKind::SunstoneCrystal,
            ResourceNodeKind::FrostCrystal,
        ] {
            assert!(ecology_entry(EcologyKind::ResourceNode(kind)).model_path.ends_with(".glb"));
        }
    }

    #[test]
    fn network_kind_ids_round_trip() {
        for kind in [
            WildlifeKind::Rabbit,
            WildlifeKind::Deer,
            WildlifeKind::Fox,
            WildlifeKind::Bear,
            WildlifeKind::Wolf,
        ] {
            assert_eq!(WildlifeKind::from_u8(kind.to_u8()), kind);
        }

        for kind in [
            ResourceNodeKind::BerryBush,
            ResourceNodeKind::MushroomRed,
            ResourceNodeKind::MushroomBrown,
            ResourceNodeKind::Flower,
            ResourceNodeKind::RockMid,
            ResourceNodeKind::RockMoss,
            ResourceNodeKind::SunstoneCrystal,
            ResourceNodeKind::FrostCrystal,
        ] {
            assert_eq!(ResourceNodeKind::from_u8(kind.to_u8()), kind);
        }
    }

    #[test]
    fn catalog_links_gatherable_world_blocks_to_resource_models() {
        for block in [
            BlockType::Wood,
            BlockType::BerryThicket,
            BlockType::SunstoneOre,
            BlockType::FrostcoreOre,
            BlockType::LivingRoot,
        ] {
            assert!(
                ECOLOGY_CATALOG.iter().any(|entry| entry.source_block == Some(block)),
                "missing catalog entry for {block:?}"
            );
        }
    }

    #[test]
    fn catalog_paths_are_asset_server_relative() {
        for entry in ECOLOGY_CATALOG {
            assert!(!entry.model_path.contains(':'));
            assert!(!entry.model_path.starts_with('/'));
            assert!(entry.model_path.ends_with(".glb"));
        }
    }

    #[test]
    fn ecology_components_package_sim_and_visual_identity() {
        let (entity, model, harvest) =
            ecology_components(EcologyKind::Tree(TreeKind::Sokpop), [4, 12, 7]);

        assert_eq!(entity.kind, EcologyKind::Tree(TreeKind::Sokpop));
        assert_eq!(entity.block_pos, [4, 12, 7]);
        assert_eq!(model.path, "procedural/pretty/sokpop_tree.glb");
        assert_eq!(harvest.unwrap().kind, ResourceKind::Wood);
    }
}
