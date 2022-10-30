use bevy::prelude::*;
use crossbeam_channel::{unbounded, Sender, Receiver};

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    Loading,
    Playing,
    Menu,
}
pub struct SetAppState(GameState);

use super::factory::CreateInsEnum;
pub fn init_state(app: &mut App) {
    // let (s,r)=unbounded::<SetAppState>();
    // app.add_system(index);
    // app.insert_resource(s).insert_resource(r);
    app.add_state(GameState::Loading);
  
}

// pub fn index(mut appState:ResMut<State<GameState>>,r:ResMut<Receiver<SetAppState>>) {
//     loop {
//         if r.is_empty() {
//             break;
//         }
//         if let Ok(newState)=r.recv(){
//             appState.set(newState.0);
//             match  {
                
//             }
//         }
//     }}