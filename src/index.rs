#[allow(dead_code, unused_variables, unused_mut, unused_imports)]
use bevy::app::App;
use bevy::{prelude::*, render::texture::ImageSettings};
pub fn entry(app: &mut App) {
    app.insert_resource(ImageSettings::default_nearest()) // prevents blurry sprites
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb(0.4, 0.4, 0.4)))
        .add_plugins(DefaultPlugins)
        .add_startup_system(init);
}

fn init(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}
