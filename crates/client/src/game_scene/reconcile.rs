//! Reconciles the offline authority's `NatureSnapshot` with the visible
//! entities: despawns berry bushes and rabbits that no longer exist, and
//! spawns the new ones.

use std::collections::HashSet;

use bevy::prelude::*;
use lk2_core::ecology::{ResourceNodeKind, WildlifeKind};

use super::offline::OfflineNature;
use super::state::{BerryBush, PlantNode, Rabbit, RabbitAi, WildlifeAnimal, Wolf, WolfAi};
use super::util::{
    BEAR_PATH, BERRY_PATH, CRYSTAL_BLUE_PATH, CRYSTAL_PINK_PATH, DEER_FAWN_PATH, DEER_PATH,
    FLOWER_PATH, FOX_PATH, FOX_SILVER_PATH, MUSHROOM_BROWN_PATH, MUSHROOM_RED_PATH,
    RABBIT_BROWN_PATH, RABBIT_PATH, ROCK_MID_PATH, ROCK_PATH, WOLF_PATH, spawn_asset,
    wildlife_physics_components,
};

pub fn reconcile_nature_entities(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    nature: Res<OfflineNature>,
    mut last_tick: Local<Option<u64>>,
    berries: Query<(Entity, &BerryBush)>,
    rabbits: Query<(Entity, &Rabbit)>,
    wolves: Query<(Entity, &Wolf)>,
    wildlife: Query<(Entity, &WildlifeAnimal)>,
    plants: Query<(Entity, &PlantNode)>,
) {
    if *last_tick == Some(nature.tick) {
        return;
    }
    *last_tick = Some(nature.tick);

    let berry_ids = nature
        .snapshot
        .detailed_ecology
        .berries
        .iter()
        .map(|berry| berry.id)
        .collect::<HashSet<_>>();
    let existing_berry_ids = berries
        .iter()
        .map(|(_, berry)| berry.id)
        .collect::<HashSet<_>>();
    for (entity, berry) in &berries {
        if !berry_ids.contains(&berry.id) {
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .despawn();
        }
    }
    for (index, berry) in nature.snapshot.detailed_ecology.berries.iter().enumerate() {
        if existing_berry_ids.contains(&berry.id) {
            continue;
        }
        spawn_asset(
            &mut commands,
            &asset_server,
            BERRY_PATH,
            Vec3::new(berry.x, 0.0, berry.z),
            0.0,
            index as f32 * 0.8,
            "grass_spawned_berry",
        )
        .insert(BerryBush {
            id: berry.id,
            mature_scale: Vec3::splat(0.92 + index as f32 * 0.025),
        });
    }

    let rabbit_ids = nature
        .snapshot
        .detailed_ecology
        .rabbits
        .iter()
        .map(|rabbit| rabbit.id)
        .collect::<HashSet<_>>();
    let existing_rabbit_ids = rabbits
        .iter()
        .map(|(_, rabbit)| rabbit.id)
        .collect::<HashSet<_>>();
    for (entity, rabbit) in &rabbits {
        if !rabbit_ids.contains(&rabbit.id) {
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .despawn();
        }
    }
    for (index, rabbit) in nature.snapshot.detailed_ecology.rabbits.iter().enumerate() {
        if existing_rabbit_ids.contains(&rabbit.id) {
            continue;
        }
        let rabbit_path = if rabbit.id % 3 == 0 {
            RABBIT_BROWN_PATH
        } else {
            RABBIT_PATH
        };
        spawn_asset(
            &mut commands,
            &asset_server,
            rabbit_path,
            Vec3::new(rabbit.x, 0.0, rabbit.z),
            0.62,
            -0.6,
            "berry_spawned_rabbit",
        )
        .insert(wildlife_physics_components())
        .insert((
            Rabbit {
                id: rabbit.id,
                phase: index as f32 * 1.17,
            },
            RabbitAi {
                target: Vec3::new(rabbit.x, 0.0, rabbit.z),
                ..default()
            },
        ));
    }

    let wolf_ids = nature
        .snapshot
        .detailed_ecology
        .wildlife
        .iter()
        .filter(|animal| animal.kind == WildlifeKind::Wolf.to_u8())
        .map(|animal| animal.id)
        .collect::<HashSet<_>>();
    let existing_wolf_ids = wolves
        .iter()
        .map(|(_, wolf)| wolf.id)
        .collect::<HashSet<_>>();
    for (entity, wolf) in &wolves {
        if !wolf_ids.contains(&wolf.id) {
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .despawn();
        }
    }
    for (index, wolf) in nature
        .snapshot
        .detailed_ecology
        .wildlife
        .iter()
        .filter(|animal| animal.kind == WildlifeKind::Wolf.to_u8())
        .enumerate()
    {
        if existing_wolf_ids.contains(&wolf.id) {
            continue;
        }
        spawn_asset(
            &mut commands,
            &asset_server,
            WOLF_PATH,
            Vec3::new(wolf.x, 0.0, wolf.z),
            0.62,
            0.4,
            "rabbit_hunting_wolf",
        )
        .insert(wildlife_physics_components())
        .insert((
            Wolf {
                id: wolf.id,
                phase: index as f32 * 1.91,
            },
            WolfAi {
                target: Vec3::new(wolf.x, 0.0, wolf.z),
                ..default()
            },
        ));
    }

    let wildlife_ids = nature
        .snapshot
        .detailed_ecology
        .wildlife
        .iter()
        .filter(|animal| animal.kind != WildlifeKind::Wolf.to_u8())
        .map(|animal| animal.id)
        .collect::<HashSet<_>>();
    let existing_wildlife_ids = wildlife
        .iter()
        .map(|(_, animal)| animal.id)
        .collect::<HashSet<_>>();
    for (entity, animal) in &wildlife {
        if !wildlife_ids.contains(&animal.id) {
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .despawn();
        }
    }
    for animal in nature
        .snapshot
        .detailed_ecology
        .wildlife
        .iter()
        .filter(|animal| animal.kind != WildlifeKind::Wolf.to_u8())
    {
        if existing_wildlife_ids.contains(&animal.id) {
            continue;
        }
        let kind = WildlifeKind::from_u8(animal.kind);
        spawn_asset(
            &mut commands,
            &asset_server,
            wildlife_path(kind, animal.id),
            Vec3::new(animal.x, 0.0, animal.z),
            0.62,
            0.0,
            "ecology_wildlife",
        )
        .insert(wildlife_physics_components())
        .insert(WildlifeAnimal { id: animal.id });
    }

    let plant_ids = nature
        .snapshot
        .detailed_ecology
        .plants
        .iter()
        .map(|plant| plant.id)
        .collect::<HashSet<_>>();
    let existing_plant_ids = plants
        .iter()
        .map(|(_, plant)| plant.id)
        .collect::<HashSet<_>>();
    for (entity, plant) in &plants {
        if !plant_ids.contains(&plant.id) {
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .despawn();
        }
    }
    for plant in &nature.snapshot.detailed_ecology.plants {
        if existing_plant_ids.contains(&plant.id) {
            continue;
        }
        let kind = ResourceNodeKind::from_u8(plant.kind);
        spawn_asset(
            &mut commands,
            &asset_server,
            plant_path(kind),
            Vec3::new(plant.x, 0.0, plant.z),
            0.58,
            plant.id as f32 * 0.43,
            "ecology_plant_node",
        )
        .insert(PlantNode { id: plant.id });
    }
}

fn wildlife_path(kind: WildlifeKind, id: u32) -> &'static str {
    match kind {
        WildlifeKind::Deer => {
            if id % 4 == 0 {
                DEER_FAWN_PATH
            } else {
                DEER_PATH
            }
        }
        WildlifeKind::Fox => {
            if id % 3 == 0 {
                FOX_SILVER_PATH
            } else {
                FOX_PATH
            }
        }
        WildlifeKind::Bear => BEAR_PATH,
        WildlifeKind::Wolf | WildlifeKind::Rabbit => WOLF_PATH,
    }
}

fn plant_path(kind: ResourceNodeKind) -> &'static str {
    match kind {
        ResourceNodeKind::MushroomRed => MUSHROOM_RED_PATH,
        ResourceNodeKind::MushroomBrown => MUSHROOM_BROWN_PATH,
        ResourceNodeKind::Flower => FLOWER_PATH,
        ResourceNodeKind::RockMid => ROCK_MID_PATH,
        ResourceNodeKind::RockMoss => ROCK_PATH,
        ResourceNodeKind::SunstoneCrystal => CRYSTAL_PINK_PATH,
        ResourceNodeKind::FrostCrystal => CRYSTAL_BLUE_PATH,
        ResourceNodeKind::BerryBush => BERRY_PATH,
    }
}
