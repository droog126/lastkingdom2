use bevy::asset::HandleId;
use bevy::utils::HashMap;
use bevy::{asset::LoadState, prelude::*};

use super::state::GameState;

pub struct TextureAtlasCenter(pub HashMap<String, Handle<TextureAtlas>>);
impl FromWorld for TextureAtlasCenter {
    fn from_world(world: &mut World) -> Self {
        let mut assertCenter: HashMap<String, Handle<TextureAtlas>> = HashMap::new();
        TextureAtlasCenter(assertCenter)
    }
}

pub struct ImageCenter(pub HashMap<String, Handle<Image>>);
impl FromWorld for ImageCenter {
    fn from_world(world: &mut World) -> Self {
        let mut assertCenter: HashMap<String, Handle<Image>> = HashMap::new();
        ImageCenter(assertCenter)
    }
}

pub struct FontCenter(pub HashMap<String, Handle<Font>>);
impl FromWorld for FontCenter {
    fn from_world(world: &mut World) -> Self {
        let mut assertCenter: HashMap<String, Handle<Font>> = HashMap::new();
        FontCenter(assertCenter)
    }
}
pub struct AssertLoadState(Vec<Handle<Image>>);

pub fn init_assets(app: &mut App) {
    app.init_resource::<TextureAtlasCenter>()
        .init_resource::<ImageCenter>()
        .init_resource::<FontCenter>();

    app.add_system_set(SystemSet::on_enter(GameState::Loading).with_system(loading_enter));
    app.add_system_set(SystemSet::on_update(GameState::Loading).with_system(loading_update));
}

fn loading_enter(
    mut textureAtlasCenter: ResMut<TextureAtlasCenter>,
    mut imageCenter: ResMut<ImageCenter>,
    mut fontCenter: ResMut<FontCenter>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    // snake
    let texture_handle = asset_server.load("sprite/snake_sheet.png");
    let sprite_atlas = TextureAtlas::from_grid_with_padding(
        texture_handle.clone(),
        Vec2::new(32.0, 32.0),
        8,
        5,
        Vec2::new(0.0, 0.0),
        Vec2::ZERO,
    );
    let sprite_handle = texture_atlases.add(sprite_atlas);
    textureAtlasCenter.0.insert("snake".to_string(), sprite_handle);
    imageCenter.0.insert("snake".to_string(), texture_handle.clone());

    // player

    let texture_handle = asset_server.load("sprite/player_sheet.png");
    let sprite_atlas = TextureAtlas::from_grid_with_padding(
        texture_handle.clone(),
        Vec2::new(32.0, 50.0),
        8,
        2,
        Vec2::new(0.0, 0.0),
        Vec2::ZERO,
    );
    let sprite_handle = texture_atlases.add(sprite_atlas);
    textureAtlasCenter.0.insert("player".to_string(), sprite_handle);
    imageCenter.0.insert("player".to_string(), texture_handle.clone());

    // hand
    let texture_handle = asset_server.load("sprite/twoHand_sheet.png");
    let sprite_atlas = TextureAtlas::from_grid_with_padding(
        texture_handle.clone(),
        Vec2::new(12.0, 7.0),
        8,
        2,
        Vec2::new(0.0, 0.0),
        Vec2::ZERO,
    );
    let sprite_handle = texture_atlases.add(sprite_atlas);
    textureAtlasCenter.0.insert("twoHand".to_string(), sprite_handle);
    imageCenter.0.insert("twoHand".to_string(), texture_handle.clone());

    // circle
    let mut imageHandle = asset_server.load("basicShape/circle.png");
    imageCenter.0.insert("circle".to_string(), imageHandle.clone());

    // shadow
    let mut imageHandle = asset_server.load("shadow/shadow.png");
    imageCenter.0.insert("shadow".to_string(), imageHandle.clone());

    // map
    let mut imageHandle: Handle<Image> = asset_server.load("background/main1.png");
    imageCenter.0.insert("map".to_string(), imageHandle.clone());

    // tree
    let mut imageHandle: Handle<Image> = asset_server.load("sprite/staInstance/tree1.png");
    imageCenter.0.insert("tree1".to_string(), imageHandle.clone());

    let mut imageHandle: Handle<Image> = asset_server.load("sprite/staInstance/tree2.png");
    imageCenter.0.insert("tree2".to_string(), imageHandle.clone());

    // font
    let mut fontHandle = asset_server.load("fonts/FiraSans-Bold.ttf");
    fontCenter.0.insert("default".to_string(), fontHandle.clone());
}

fn loading_update(
    mut gameState: ResMut<State<GameState>>,
    imageCenter: Res<ImageCenter>,
    textureAtlasCenter: Res<TextureAtlasCenter>,
    asset_server: Res<AssetServer>,
) {
    let mut handles = imageCenter.0.iter().map(|(k, v)| HandleId::from(v));
    if let LoadState::Loaded = asset_server.get_group_load_state(handles) {
        gameState.set(GameState::Playing).unwrap();
    }
}
