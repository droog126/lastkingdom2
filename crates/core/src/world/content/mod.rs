mod catalog;
mod game;
mod materialize;
mod solver;

pub use catalog::{
    ContentArchetype, ContentCatalog, ContentCatalogBuilder, ContentCatalogError, ContentId,
    Direction3,
};
pub use game::{
    GAME_CONTENT_CAVERN, GAME_CONTENT_DUNGEON, GAME_CONTENT_EMPTY, GAME_CONTENT_MONSTER_TERRITORY,
    GAME_CONTENT_SETTLEMENT, GAME_CONTENT_TREASURE_VAULT, GAME_CONTENT_VERTICAL_PASSAGE,
    GAME_CONTENT_WILDERNESS, GameContentTheme, GameContentVolumeConfig, game_content_catalog,
    generate_game_content_volume,
};
pub use materialize::{
    CONTENT_CELL_SIZE, ContentAnchor, MaterializedContent, content_route_is_walkable,
    generate_and_materialize_content,
};
pub use solver::{
    CellConstraint, ContentSolveConfig, ContentSolveError, ContentVolume, solve_content_volume,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constant::{SEA_LEVEL, WORLD_CENTER};
    use crate::world::{BlockType, WorldConfig, generate_world, player_body_clear};

    fn line_catalog() -> ContentCatalog {
        let sealed = ContentId(0);
        let west_cap = ContentId(1);
        let corridor = ContentId(2);
        let east_cap = ContentId(3);

        ContentCatalog::builder()
            .archetype(sealed, "sealed", 1)
            .archetype(west_cap, "west_cap", 1)
            .archetype(corridor, "corridor", 4)
            .archetype(east_cap, "east_cap", 1)
            .allow_all(sealed, sealed)
            .allow_bidirectional(west_cap, Direction3::PosX, corridor)
            .allow_bidirectional(corridor, Direction3::PosX, corridor)
            .allow_bidirectional(corridor, Direction3::PosX, east_cap)
            .allow_same_position_neighbors(west_cap)
            .allow_same_position_neighbors(corridor)
            .allow_same_position_neighbors(east_cap)
            .build()
            .expect("test catalog should be valid")
    }

    #[test]
    fn same_seed_produces_the_same_3d_content_volume() {
        let catalog = game_content_catalog(GameContentTheme::Frontier);
        let config = GameContentVolumeConfig::new([6, 3, 6], 0xA11CE);

        let first = generate_game_content_volume(&catalog, &config).expect("first solve");
        let second = generate_game_content_volume(&catalog, &config).expect("second solve");

        assert_eq!(first, second);
    }

    #[test]
    fn propagation_respects_all_six_directions() {
        let catalog = line_catalog();
        let config = ContentSolveConfig::new([3, 1, 1], 7)
            .with_constraint(CellConstraint::only([0, 0, 0], ContentId(1)))
            .with_constraint(CellConstraint::only([1, 0, 0], ContentId(2)))
            .with_constraint(CellConstraint::only([2, 0, 0], ContentId(3)));

        let volume = solve_content_volume(&catalog, &config).expect("line should solve");

        assert_eq!(volume.get([0, 0, 0]), Some(ContentId(1)));
        assert_eq!(volume.get([1, 0, 0]), Some(ContentId(2)));
        assert_eq!(volume.get([2, 0, 0]), Some(ContentId(3)));
        assert!(volume.all_neighbors_are_compatible(&catalog));
    }

    #[test]
    fn every_3d_direction_propagates_to_its_neighbor() {
        let source = ContentId(10);
        let neighbor = ContentId(11);

        for direction in Direction3::ALL {
            let catalog = ContentCatalog::builder()
                .archetype(source, "source", 1)
                .archetype(neighbor, "neighbor", 1)
                .allow_bidirectional(source, direction, neighbor)
                .build()
                .expect("direction catalog should be valid");
            let (dimensions, source_position, neighbor_position) = match direction {
                Direction3::NegX => ([2, 1, 1], [1, 0, 0], [0, 0, 0]),
                Direction3::PosX => ([2, 1, 1], [0, 0, 0], [1, 0, 0]),
                Direction3::NegY => ([1, 2, 1], [0, 1, 0], [0, 0, 0]),
                Direction3::PosY => ([1, 2, 1], [0, 0, 0], [0, 1, 0]),
                Direction3::NegZ => ([1, 1, 2], [0, 0, 1], [0, 0, 0]),
                Direction3::PosZ => ([1, 1, 2], [0, 0, 0], [0, 0, 1]),
            };
            let config = ContentSolveConfig::new(dimensions, 17)
                .with_constraint(CellConstraint::only(source_position, source));

            let volume = solve_content_volume(&catalog, &config).expect("pair should solve");

            assert_eq!(volume.get(source_position), Some(source));
            assert_eq!(volume.get(neighbor_position), Some(neighbor));
            assert!(volume.all_neighbors_are_compatible(&catalog));
        }
    }

    #[test]
    fn contradiction_returns_an_error_instead_of_looping() {
        let catalog = line_catalog();
        let config = ContentSolveConfig::new([2, 1, 1], 9)
            .with_constraint(CellConstraint::only([0, 0, 0], ContentId(1)))
            .with_constraint(CellConstraint::only([1, 0, 0], ContentId(3)));

        let error =
            solve_content_volume(&catalog, &config).expect_err("caps cannot touch directly");

        assert!(matches!(error, ContentSolveError::Contradiction { .. }));
    }

    #[test]
    fn frontier_profile_places_surface_and_underground_game_content() {
        let catalog = game_content_catalog(GameContentTheme::Frontier);
        let config = GameContentVolumeConfig::new([7, 3, 7], 1234);

        let volume =
            generate_game_content_volume(&catalog, &config).expect("frontier should solve");

        assert!(volume.contains(GAME_CONTENT_SETTLEMENT));
        assert!(volume.contains(GAME_CONTENT_MONSTER_TERRITORY));
        assert!(volume.contains(GAME_CONTENT_TREASURE_VAULT));
        assert!(volume.all_neighbors_are_compatible(&catalog));
    }

    #[test]
    fn default_world_materializes_3d_game_content_around_spawn() {
        let world = generate_world(&WorldConfig::default());
        let content = world.content.as_ref().expect("default world should contain game content");

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
