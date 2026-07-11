//! Reconciles the offline authority's `NatureSnapshot` with the visible
//! entities: despawns berry bushes and rabbits that no longer exist, and
//! spawns the new ones.

use std::collections::HashSet;

use bevy::prelude::*;

use super::offline::OfflineNature;
use super::state::{BerryBush, Rabbit, RabbitAi};
use super::util::{spawn_asset, BERRY_PATH, RABBIT_PATH};

pub fn reconcile_nature_entities(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    nature: Res<OfflineNature>,
    mut last_tick: Local<Option<u64>>,
    berries: Query<(Entity, &BerryBush)>,
    rabbits: Query<(Entity, &Rabbit)>,
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
            commands.entity(entity).despawn();
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
            commands.entity(entity).despawn();
        }
    }
    for (index, rabbit) in nature.snapshot.detailed_ecology.rabbits.iter().enumerate() {
        if existing_rabbit_ids.contains(&rabbit.id) {
            continue;
        }
        spawn_asset(
            &mut commands,
            &asset_server,
            RABBIT_PATH,
            Vec3::new(rabbit.x, 0.0, rabbit.z),
            0.0,
            -0.6,
            "berry_spawned_rabbit",
        )
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
}
