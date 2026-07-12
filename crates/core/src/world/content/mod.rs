mod catalog;
mod game;
mod materialize;
mod spice;

pub use catalog::{
    ContentArchetype, ContentCatalog, ContentCatalogBuilder, ContentCatalogError, ContentId,
    Direction3,
};
pub use game::{
    ContentSpiceProfile, GAME_CONTENT_CAVERN, GAME_CONTENT_DUNGEON, GAME_CONTENT_EMPTY,
    GAME_CONTENT_MONSTER_TERRITORY, GAME_CONTENT_SETTLEMENT, GAME_CONTENT_TREASURE_VAULT,
    GAME_CONTENT_VERTICAL_PASSAGE, GAME_CONTENT_WILDERNESS, GameContentTheme,
    GameContentVolumeConfig, game_content_catalog, generate_game_content_volume,
    resolve_content_spice_profile,
};
pub use materialize::{
    CONTENT_CELL_SIZE, ContentAnchor, ContentMaterializeError, MaterializedContent,
    content_route_is_walkable, generate_and_materialize_content,
};
pub use spice::{ContentGenerationError, ContentVolume};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constant::{SEA_LEVEL, WORLD_CENTER};
    use crate::world::{BlockType, WorldConfig, generate_world, player_body_clear};

    #[test]
    fn same_seed_produces_the_same_3d_content_volume() {
        let catalog = game_content_catalog(GameContentTheme::Frontier);
        let config = GameContentVolumeConfig::new([6, 3, 6], 0xA11CE);

        let first = generate_game_content_volume(&catalog, &config).expect("first generate");
        let second = generate_game_content_volume(&catalog, &config).expect("second generate");

        assert_eq!(first, second);
    }

    #[test]
    fn spice_profiles_make_large_correlated_decisions() {
        let catalog = game_content_catalog(GameContentTheme::Frontier);
        let wild = generate_game_content_volume(
            &catalog,
            &GameContentVolumeConfig::new([7, 3, 7], 1234)
                .with_profile(ContentSpiceProfile::HomesteadWilds),
        )
        .expect("wild profile should generate");
        let monster = generate_game_content_volume(
            &catalog,
            &GameContentVolumeConfig::new([7, 3, 7], 1234)
                .with_profile(ContentSpiceProfile::MonsterMarch),
        )
        .expect("monster profile should generate");
        let crystal = generate_game_content_volume(
            &catalog,
            &GameContentVolumeConfig::new([7, 3, 7], 1234)
                .with_profile(ContentSpiceProfile::CrystalDescent),
        )
        .expect("crystal profile should generate");

        assert!(
            monster.count(GAME_CONTENT_MONSTER_TERRITORY)
                > wild.count(GAME_CONTENT_MONSTER_TERRITORY) + 8
        );
        assert!(crystal.count(GAME_CONTENT_DUNGEON) > wild.count(GAME_CONTENT_DUNGEON) + 8);
        assert_ne!(wild.cells, monster.cells);
        assert_ne!(wild.cells, crystal.cells);
    }

    #[test]
    fn every_spice_profile_places_required_compatible_content() {
        let catalog = game_content_catalog(GameContentTheme::Frontier);
        for profile in ContentSpiceProfile::ALL {
            let config = GameContentVolumeConfig::new([7, 3, 7], 1234).with_profile(profile);
            let volume =
                generate_game_content_volume(&catalog, &config).expect("profile should generate");

            assert_eq!(volume.get([3, 1, 3]), Some(GAME_CONTENT_SETTLEMENT));
            assert_eq!(volume.get([1, 1, 1]), Some(GAME_CONTENT_MONSTER_TERRITORY));
            assert_eq!(volume.get([5, 0, 5]), Some(GAME_CONTENT_TREASURE_VAULT));
            assert!(volume.contains(GAME_CONTENT_VERTICAL_PASSAGE));
            assert!(volume.all_neighbors_are_compatible(&catalog));
        }
    }

    #[test]
    fn frontier_profile_places_surface_and_underground_game_content() {
        let catalog = game_content_catalog(GameContentTheme::Frontier);
        let config = GameContentVolumeConfig::new([7, 3, 7], 1234);

        let volume =
            generate_game_content_volume(&catalog, &config).expect("frontier should generate");

        assert!(volume.contains(GAME_CONTENT_SETTLEMENT));
        assert!(volume.contains(GAME_CONTENT_MONSTER_TERRITORY));
        assert!(volume.contains(GAME_CONTENT_TREASURE_VAULT));
        assert!(volume.all_neighbors_are_compatible(&catalog));
    }

    #[test]
    fn default_world_materializes_3d_game_content_around_spawn() {
        let world = generate_world(&WorldConfig::default());
        let content = world
            .content
            .as_ref()
            .expect("default world should contain game content");

        assert_eq!(content.volume.dimensions, [5, 3, 5]);
        assert_eq!(
            content.settlement_position,
            [WORLD_CENTER[0], SEA_LEVEL + 3, WORLD_CENTER[1]]
        );
        assert_eq!(
            world.get(WORLD_CENTER[0], SEA_LEVEL + 2, WORLD_CENTER[1]),
            BlockType::Grass
        );
        assert!(player_body_clear(
            &world,
            WORLD_CENTER[0],
            SEA_LEVEL + 3,
            WORLD_CENTER[1]
        ));
        assert!(content.anchor(GAME_CONTENT_MONSTER_TERRITORY).is_some());
        assert!(content.anchor(GAME_CONTENT_TREASURE_VAULT).is_some());
    }

    #[test]
    fn materialized_dungeon_has_a_walkable_route_to_treasure() {
        let world = generate_world(&WorldConfig::default());
        let content = world.content.as_ref().expect("content should be generated");

        assert!(player_body_clear(
            &world,
            content.entrance_position[0],
            content.entrance_position[1],
            content.entrance_position[2]
        ));
        assert!(player_body_clear(
            &world,
            content.treasure_position[0],
            content.treasure_position[1],
            content.treasure_position[2]
        ));
        assert!(content_route_is_walkable(&world, content));
        assert!(matches!(
            world.get(
                content.treasure_position[0],
                content.treasure_position[1] - 1,
                content.treasure_position[2]
            ),
            BlockType::SunstoneOre | BlockType::FrostcoreOre | BlockType::LivingRoot
        ));
    }

    #[test]
    fn materialized_content_is_deterministic_for_shared_client_server_worlds() {
        let client_world = generate_world(&WorldConfig::default());
        let server_world = generate_world(&WorldConfig::default());

        assert_eq!(client_world.content, server_world.content);
        for &(x, y, z) in &[
            (WORLD_CENTER[0], SEA_LEVEL + 2, WORLD_CENTER[1]),
            (WORLD_CENTER[0] + 5, SEA_LEVEL + 2, WORLD_CENTER[1]),
            (WORLD_CENTER[0], SEA_LEVEL - 4, WORLD_CENTER[1]),
            (WORLD_CENTER[0] + 14, SEA_LEVEL - 4, WORLD_CENTER[1] + 14),
        ] {
            assert_eq!(client_world.get(x, y, z), server_world.get(x, y, z));
        }
    }
}
