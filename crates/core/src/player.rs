use bevy::prelude::*;
use std::collections::HashMap;

use crate::nation::NationId;
use crate::resource::ResourceKind;

#[derive(Resource, Default)]
pub struct PlayerState {
    pub pos: Vec3,
    pub block_pos: [i32; 3],
    pub inventory: HashMap<ResourceKind, i64>,
    pub nation_id: Option<NationId>,
    pub monsters_killed: u32,
    pub blocks_gathered: u32,
    pub nations_founded: u32,
}

#[derive(Component, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PlayerTag(pub u32);

impl PlayerTag {
    pub fn id(&self) -> u32 {
        self.0
    }
}
