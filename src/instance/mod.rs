use std::hash::Hash;

use bevy::{
    prelude::{App, Component, SystemSet},
    time::FixedTimestep,
    utils::hashbrown::HashSet,
};

use crate::constant::FPS;
pub mod player;
pub fn ins_added_dependence(app: &mut App) {
    app.add_system(player::on_player_add);
    app.add_system_set(
        SystemSet::new()
            .with_run_criteria(FixedTimestep::step((1.0 / FPS).into()))
            .with_system(player::on_player_step),
    );
    app.insert_resource(InstancePermanentClassMap::default());
}

#[derive(Debug, Eq, PartialEq, Hash, Component, Clone, Copy)]
pub enum InstanceUnitType {
    Player,
    Snake,
    Wall,
}

#[derive(Debug)]
pub struct InstancePermanentClassMap {
    isFriendly: HashSet<InstanceUnitType>,
    isNeutral: HashSet<InstanceUnitType>,
    isHostile: HashSet<InstanceUnitType>,
    isDynamic: HashSet<InstanceUnitType>,
    isStatic: HashSet<InstanceUnitType>,
}
impl Default for InstancePermanentClassMap {
    fn default() -> Self {
        let mut classMap = Self {
            isFriendly: HashSet::new(),
            isNeutral: HashSet::new(),
            isHostile: HashSet::new(),
            isDynamic: HashSet::new(),
            isStatic: HashSet::new(),
        };
        classMap.isNeutral.insert(InstanceUnitType::Player);
        classMap.isDynamic.insert(InstanceUnitType::Player);

        classMap.isHostile.insert(InstanceUnitType::Snake);
        classMap.isDynamic.insert(InstanceUnitType::Snake);

        classMap.isStatic.insert(InstanceUnitType::Wall);
        classMap
    }
}
impl InstanceUnitType {
    pub fn isNeutral(&self, classMap: &InstancePermanentClassMap) -> bool {
        classMap.isNeutral.contains(self)
    }
    pub fn isHostile(&self, classMap: &InstancePermanentClassMap) -> bool {
        classMap.isHostile.contains(self)
    }
    pub fn isFriendly(&self, classMap: &InstancePermanentClassMap) -> bool {
        classMap.isFriendly.contains(self)
    }
    pub fn isDynamic(&self, classMap: &InstancePermanentClassMap) -> bool {
        classMap.isDynamic.contains(self)
    }
    pub fn isStatic(&self, classMap: &InstancePermanentClassMap) -> bool {
        classMap.isStatic.contains(self)
    }
}

pub struct InstanceRuntimeClassMap {}

// books.contains("The Winds of Winter")
