use bevy::prelude::*;
use crossbeam_channel::*;

use crate::instance::{ins_added_dependence, player::Player};

pub enum CreateInsEnum {
    Player { x: f32, y: f32, z: f32 },
}

pub struct InsFactoryData {}
pub fn init_ins_factory_dependence(app: &mut App) {
    let (s, r) = unbounded::<CreateInsEnum>();
    app.insert_resource(r);
    app.insert_resource(s); 
    app.add_system_to_stage(CoreStage::PostUpdate, ins_factory.exclusive_system());
    ins_added_dependence(app);
}
fn ins_factory(world: &mut World) {
    let event_receiver = world.resource_mut::<Receiver<CreateInsEnum>>().clone();
    loop {
        if event_receiver.is_empty() {
            break;
        }
        match event_receiver.recv().unwrap() {
            CreateInsEnum::Player { x, y, z } => {
                world
                    .spawn()
                    .insert_bundle(SpatialBundle {
                        transform: Transform {
                            translation: Vec3 { x, y, ..default() },
                            ..default()
                        },
                        ..default()
                    })
                    .insert(Player {});
            }
        }
    }
}


