use bevy::prelude::*;
use crossbeam_channel::Sender;

use super::{factory::CreateInsEnum, state::GameState};
pub fn init_test_system(app: &mut App) {
    app.add_system_set(SystemSet::on_enter(GameState::Playing).with_system(start));
}

pub fn start(create_ins: ResMut<Sender<CreateInsEnum>>) {
    // for _ in 0..20000 {
    //     create_ins.send(CreateInsEnum::Player { x: 1.0, y: 2.0, z: 0.0 }).unwrap();
    // }
    create_ins.send(CreateInsEnum::Player { x: 1.0, y: 2.0, z: 0.0 }).unwrap();

    create_ins.send(CreateInsEnum::Player { x: 1.0, y: 3.0, z: 0.0 }).unwrap();
}

pub fn index(create_ins: ResMut<Sender<CreateInsEnum>>) {
    // create_ins.send(CreateInsEnum::Player { x: 1.0, y: 2.0, z: 0.0 }).unwrap();
}
