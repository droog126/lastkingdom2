use bevy::prelude::*;

#[derive(Component)]
pub struct Player {}
pub fn player_add(query: Query<Entity, Added<Player>>) {
    for (entity) in &query {
        println!("创建了一个player");
    }
}
