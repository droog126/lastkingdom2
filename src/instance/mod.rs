use bevy::prelude::App;

pub mod player;
pub fn ins_added_dependence(app: &mut App) {
    app.add_system(player::player_add);
}
