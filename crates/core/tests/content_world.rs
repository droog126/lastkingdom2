use lk2_core::constant::{SEA_LEVEL, WORLD_CENTER};
use lk2_core::monster::MonsterEcosystem;
use lk2_core::world::content::content_route_is_walkable;
use lk2_core::world::{BlockType, WorldConfig, generate_world, player_body_clear};

#[test]
fn shared_world_materializes_playable_3d_content() {
    let world = generate_world(&WorldConfig::default());
    let content = world.content.as_ref().expect("default world content");

    assert_eq!(content.volume.dimensions, [5, 3, 5]);
    assert_eq!(
        content.settlement_position,
        [WORLD_CENTER[0], SEA_LEVEL + 3, WORLD_CENTER[1]]
    );
    assert!(player_body_clear(
        &world,
        content.settlement_position[0],
        content.settlement_position[1],
        content.settlement_position[2]
    ));
    assert!(content_route_is_walkable(&world, content));
    assert_eq!(
        world.get(
            content.treasure_position[0],
            content.treasure_position[1] - 1,
            content.treasure_position[2]
        ),
        BlockType::SunstoneOre
    );
}

#[test]
fn generated_monsters_use_the_content_territory() {
    let world = generate_world(&WorldConfig::default());
    let content = world.content.as_ref().expect("default world content");
    let mut monsters = MonsterEcosystem::new();

    monsters.demo_init_at(content.monster_position);

    let kingdom = monsters.kingdoms.values().next().expect("demo monster kingdom");
    assert_eq!(kingdom.center, content.monster_position);
}

#[test]
fn client_and_server_world_generation_is_byte_for_byte_deterministic_at_content_edits() {
    let client = generate_world(&WorldConfig::default());
    let server = generate_world(&WorldConfig::default());

    assert_eq!(client.content, server.content);
    assert_eq!(client.blocks, server.blocks);
    assert_eq!(client.edited, server.edited);
}
