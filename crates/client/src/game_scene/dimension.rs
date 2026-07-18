//! The playable starfall dimension: a second local realm reached through a
//! portal in the living forest.

use avian3d::prelude::{Collider, ColliderDisabled, LinearVelocity, RigidBody};
use bevy::prelude::*;

use super::offline::OfflineNature;
use super::state::{
    DefeatedCreature, DimensionId, DimensionPortal, DimensionTravelState, ForestRealmCollider,
    ForestRealmVisual, PlayerActor, ProceduralTerrainSurface, StarfallGuardian, StarfallProgress,
    StarfallGuardianAttack, StarfallRealmCollider, StarfallRealmVisual, StarfallRelic,
    StarfallShard,
};
use super::util::{AETHER_WRAITH_PATH, PLAYER_PHYSICS_CENTER_HEIGHT, spawn_asset};
use lk2_core::pvp::{Health, Hitbox};
use lk2_core::resource::ResourceKind;

const PORTAL_INTERACT_RANGE: f32 = 3.4;
const STARFALL_PLATFORM_RADIUS: f32 = 11.0;

pub fn setup_dimension(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    terrain: Res<ProceduralTerrainSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let forest_portal_position = Vec3::new(
        9.0,
        terrain.ground_height(Vec3::new(9.0, 0.0, -8.0)),
        -8.0,
    );
    let starfall_floor = terrain.ground_height(Vec3::new(0.0, 0.0, 0.0)) + 0.06;
    let starfall_portal_position = Vec3::new(0.0, starfall_floor, 2.8);

    let portal_pillar_mesh = meshes.add(Cuboid::new(0.28, 2.6, 0.28));
    let portal_beam_mesh = meshes.add(Cuboid::new(2.35, 0.24, 0.28));
    let portal_orb_mesh = meshes.add(
        Sphere::new(0.34)
            .mesh()
            .ico(2)
            .expect("dimension portal orb mesh should be valid"),
    );
    let portal_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.06, 0.30),
        emissive: Color::srgb(0.42, 0.08, 0.95).into(),
        unlit: true,
        ..default()
    });
    let portal_accent = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.55, 0.90),
        emissive: Color::srgb(0.12, 0.65, 1.0).into(),
        unlit: true,
        ..default()
    });

    spawn_portal(
        &mut commands,
        forest_portal_position,
        DimensionId::Starfall,
        ForestRealmVisual,
        portal_pillar_mesh.clone(),
        portal_beam_mesh.clone(),
        portal_orb_mesh.clone(),
        portal_material.clone(),
        portal_accent.clone(),
        "forest_to_starfall_portal",
    );
    spawn_portal(
        &mut commands,
        starfall_portal_position,
        DimensionId::Forest,
        StarfallRealmVisual,
        portal_pillar_mesh,
        portal_beam_mesh,
        portal_orb_mesh,
        portal_material,
        portal_accent,
        "starfall_return_portal",
    );

    let platform_mesh = meshes.add(
        Cylinder::new(STARFALL_PLATFORM_RADIUS, 0.35)
            .mesh()
            .resolution(32),
    );
    let platform_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.035, 0.16),
        emissive: Color::srgb(0.025, 0.01, 0.10).into(),
        metallic: 0.15,
        perceptual_roughness: 0.72,
        ..default()
    });
    commands.spawn((
        Mesh3d(platform_mesh),
        MeshMaterial3d(platform_material),
        Transform::from_xyz(0.0, starfall_floor - 0.18, 0.0),
        Visibility::Hidden,
        StarfallRealmVisual,
        Name::new("starfall_platform"),
    ));
    commands.spawn((
        RigidBody::Static,
        Collider::cylinder(STARFALL_PLATFORM_RADIUS, 0.35),
        ColliderDisabled,
        Transform::from_xyz(0.0, starfall_floor - 0.18, 0.0),
        StarfallRealmCollider,
        Name::new("starfall_platform_collider"),
    ));

    let crystal_mesh = meshes.add(Cone::new(0.28, 1.8).mesh().resolution(6));
    let crystal_materials = [
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.16, 0.55, 0.95),
            emissive: Color::srgb(0.08, 0.42, 1.0).into(),
            unlit: true,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.70, 0.18, 0.92),
            emissive: Color::srgb(0.46, 0.05, 0.90).into(),
            unlit: true,
            ..default()
        }),
    ];
    for index in 0..12 {
        let angle = index as f32 * std::f32::consts::TAU / 12.0;
        let radius = 5.0 + (index % 3) as f32 * 1.15;
        let height = 1.0 + (index % 4) as f32 * 0.35;
        commands.spawn((
            Mesh3d(crystal_mesh.clone()),
            MeshMaterial3d(crystal_materials[index % 2].clone()),
            Transform::from_xyz(
                angle.cos() * radius,
                starfall_floor + height * 0.5,
                angle.sin() * radius,
            )
            .with_scale(Vec3::new(1.0, height, 1.0))
            .with_rotation(Quat::from_rotation_y(angle)),
            Visibility::Hidden,
            StarfallRealmVisual,
            Name::new(format!("starfall_crystal_{index}")),
        ));
    }

    let shard_mesh = meshes.add(
        Sphere::new(0.18)
            .mesh()
            .ico(1)
            .expect("starfall shard mesh should be valid"),
    );
    let shard_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.74, 0.24),
        emissive: Color::srgb(0.85, 0.32, 0.04).into(),
        unlit: true,
        ..default()
    });
    for index in 0..8 {
        let angle = index as f32 * std::f32::consts::TAU / 8.0 + 0.2;
        let base = Vec3::new(
            angle.cos() * 3.25,
            starfall_floor + 0.72 + (index % 2) as f32 * 0.16,
            angle.sin() * 3.25,
        );
        commands.spawn((
            Mesh3d(shard_mesh.clone()),
            MeshMaterial3d(shard_material.clone()),
            Transform::from_translation(base),
            Visibility::Hidden,
            StarfallRealmVisual,
            StarfallShard {
                base,
                phase: index as f32 * 0.73,
            },
            Name::new(format!("starfall_shard_{index}")),
        ));
    }

    let guardian_base = Vec3::new(0.0, starfall_floor + 0.95, -7.2);
    spawn_asset(
        &mut commands,
        &asset_server,
        AETHER_WRAITH_PATH,
        guardian_base,
        0.78,
        std::f32::consts::PI,
        "starfall_guardian",
    )
    .insert((
        Visibility::Hidden,
        StarfallRealmVisual,
        StarfallGuardian {
            base: guardian_base,
            phase: 0.0,
        },
        StarfallGuardianAttack::default(),
        Health {
            current: 36.0,
            max: 36.0,
            invuln_until_tick: 0,
        },
        Hitbox {
            center_offset: Vec3::new(0.0, 0.9, 0.0),
            radius: 0.70,
        },
        super::state::HitReaction::default(),
    ))
    .remove::<ForestRealmVisual>();

    let relic_mesh = meshes.add(
        Sphere::new(0.42)
            .mesh()
            .ico(2)
            .expect("starfall relic mesh should be valid"),
    );
    let relic_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.48, 0.16),
        emissive: Color::srgb(0.92, 0.10, 0.02).into(),
        metallic: 0.28,
        perceptual_roughness: 0.34,
        ..default()
    });
    commands.spawn((
        Mesh3d(relic_mesh),
        MeshMaterial3d(relic_material),
        Transform::from_xyz(0.0, starfall_floor + 1.10, 0.0),
        Visibility::Hidden,
        StarfallRealmVisual,
        StarfallRelic,
        Name::new("starfall_relic"),
    ));

    // A small local light gives the new realm a readable silhouette even when
    // the forest lighting and terrain are disabled.
    commands.spawn((
        PointLight {
            color: Color::srgb(0.30, 0.18, 1.0),
            intensity: 5_000.0,
            range: 24.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, starfall_floor + 5.5, 0.0),
        Visibility::Hidden,
        StarfallRealmVisual,
        Name::new("starfall_core_light"),
    ));
}

fn spawn_portal<M: Component>(
    commands: &mut Commands,
    position: Vec3,
    destination: DimensionId,
    realm_marker: M,
    pillar_mesh: Handle<Mesh>,
    beam_mesh: Handle<Mesh>,
    orb_mesh: Handle<Mesh>,
    pillar_material: Handle<StandardMaterial>,
    accent_material: Handle<StandardMaterial>,
    name: &'static str,
) {
    commands
        .spawn((
            Transform::from_translation(position),
            if destination == DimensionId::Forest {
                Visibility::Hidden
            } else {
                Visibility::Visible
            },
            DimensionPortal { destination },
            realm_marker,
            Name::new(name),
        ))
        .with_children(|portal| {
            portal.spawn((
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(pillar_material.clone()),
                Transform::from_xyz(-1.0, 1.3, 0.0),
            ));
            portal.spawn((
                Mesh3d(pillar_mesh),
                MeshMaterial3d(pillar_material),
                Transform::from_xyz(1.0, 1.3, 0.0),
            ));
            portal.spawn((
                Mesh3d(beam_mesh),
                MeshMaterial3d(accent_material.clone()),
                Transform::from_xyz(0.0, 2.6, 0.0),
            ));
            portal.spawn((
                Mesh3d(orb_mesh),
                MeshMaterial3d(accent_material),
                Transform::from_xyz(0.0, 2.85, 0.0),
            ));
        });
}

pub fn toggle_dimension_travel(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<super::keybindings::KeyBindings>,
    terrain: Res<ProceduralTerrainSurface>,
    mut travel: ResMut<DimensionTravelState>,
    mut queries: ParamSet<(
        Query<
            (
                &mut Transform,
                &mut LinearVelocity,
                Option<&mut super::state::PlayerSwimState>,
            ),
            (With<PlayerActor>, Without<DimensionPortal>),
        >,
        Query<(&Transform, &DimensionPortal), Without<PlayerActor>>,
    )>,
) {
    if bindings.menu_open
        || !bindings.just_pressed(super::keybindings::GameAction::Interact, &keys, &mouse)
    {
        return;
    }
    let Ok((player_translation, player_rotation)) = queries
        .p0()
        .single()
        .map(|(player, _, _)| (player.translation, player.rotation))
    else {
        return;
    };
    let destination = match travel.current {
        DimensionId::Forest => DimensionId::Starfall,
        DimensionId::Starfall => DimensionId::Forest,
    };
    let portal_in_range = queries
        .p1()
        .iter()
        .filter(|(transform, portal)| {
            portal.destination == destination
                && transform.translation.distance_squared(player_translation)
                    <= PORTAL_INTERACT_RANGE.powi(2)
        })
        .next()
        .is_some();
    if !portal_in_range {
        return;
    }

    let mut players = queries.p0();
    let Ok((mut player, mut velocity, swim)) = players.single_mut() else {
        return;
    };

    match destination {
        DimensionId::Starfall => {
            travel.return_position = Some(player_translation);
            travel.return_rotation = Some(player_rotation);
            let floor = terrain.ground_height(Vec3::ZERO) + 0.06;
            player.translation = Vec3::new(0.0, floor + PLAYER_PHYSICS_CENTER_HEIGHT + 0.4, 0.0);
            info!("[dimension] entered starfall dimension");
        }
        DimensionId::Forest => {
            if let Some(position) = travel.return_position.take() {
                player.translation = position;
            }
            if let Some(rotation) = travel.return_rotation.take() {
                player.rotation = rotation;
            }
            info!("[dimension] returned to the living forest");
        }
    }
    velocity.0 = Vec3::ZERO;
    if let Some(mut swim) = swim {
        swim.active = false;
        swim.velocity = Vec3::ZERO;
    }
    travel.current = destination;
}

pub fn update_dimension_presentation(
    travel: Res<DimensionTravelState>,
    mut clear_color: ResMut<ClearColor>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut forest_visuals: Query<
        &mut Visibility,
        (With<ForestRealmVisual>, Without<StarfallRealmVisual>),
    >,
    mut starfall_visuals: Query<
        &mut Visibility,
        (With<StarfallRealmVisual>, Without<ForestRealmVisual>),
    >,
    forest_colliders: Query<(Entity, Option<&ColliderDisabled>), With<ForestRealmCollider>>,
    starfall_colliders: Query<(Entity, Option<&ColliderDisabled>), With<StarfallRealmCollider>>,
    mut commands: Commands,
) {
    let in_starfall = travel.current == DimensionId::Starfall;
    for mut visibility in &mut forest_visuals {
        *visibility = if in_starfall {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    for mut visibility in &mut starfall_visuals {
        *visibility = if in_starfall {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if in_starfall {
        clear_color.0 = Color::srgb(0.015, 0.008, 0.07);
        ambient.color = Color::srgb(0.20, 0.16, 0.48);
        ambient.brightness = 120.0;
    } else {
        clear_color.0 = Color::srgb(0.47, 0.61, 0.72);
        ambient.color = Color::srgb(0.78, 0.86, 0.82);
        ambient.brightness = 190.0;
    }
    toggle_colliders(&mut commands, forest_colliders, in_starfall);
    toggle_colliders(&mut commands, starfall_colliders, !in_starfall);
}

pub fn animate_starfall_shards(
    time: Res<Time>,
    travel: Res<DimensionTravelState>,
    mut shards: Query<(&StarfallShard, &mut Transform)>,
) {
    if travel.current != DimensionId::Starfall {
        return;
    }
    for (shard, mut transform) in &mut shards {
        transform.translation =
            shard.base + Vec3::Y * (time.elapsed_secs() * 2.0 + shard.phase).sin() * 0.10;
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs() * 1.8 + shard.phase);
    }
}

pub fn animate_starfall_guardian(
    time: Res<Time>,
    travel: Res<DimensionTravelState>,
    progress: Res<StarfallProgress>,
    mut guardians: Query<(&StarfallGuardian, &mut Transform, Option<&DefeatedCreature>)>,
) {
    if travel.current != DimensionId::Starfall
        || progress.collected < progress.total
        || progress.guardian_defeated
    {
        return;
    }
    for (guardian, mut transform, defeated) in &mut guardians {
        if defeated.is_some() {
            continue;
        }
        transform.translation =
            guardian.base + Vec3::Y * (time.elapsed_secs() * 2.4 + guardian.phase).sin() * 0.18;
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs() * 0.8);
    }
}

pub fn update_starfall_guardian(
    travel: Res<DimensionTravelState>,
    mut progress: ResMut<StarfallProgress>,
    mut guardians: Query<(&mut Visibility, Option<&DefeatedCreature>), With<StarfallGuardian>>,
) {
    let mut defeated = false;
    for (_, marker) in &mut guardians {
        defeated |= marker.is_some();
    }
    if defeated {
        progress.guardian_defeated = true;
    }
    let visible = travel.current == DimensionId::Starfall
        && progress.collected >= progress.total
        && !progress.guardian_defeated;
    for (mut visibility, marker) in &mut guardians {
        *visibility = if visible && marker.is_none() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn starfall_guardian_attack(
    time: Res<Time>,
    state: Res<super::state::LivingSceneState>,
    travel: Res<DimensionTravelState>,
    progress: Res<StarfallProgress>,
    hit_settings: Option<Res<super::state::CreatureHitSettings>>,
    mut guardians: Query<
        (&Transform, &mut StarfallGuardianAttack, Option<&DefeatedCreature>),
        With<StarfallGuardian>,
    >,
    mut players: Query<(&Transform, &mut Health), With<PlayerActor>>,
) {
    if travel.current != DimensionId::Starfall || progress.collected < progress.total {
        return;
    }
    let Ok((player, mut player_health)) = players.single_mut() else {
        return;
    };
    let invulnerability_ticks = hit_settings
        .as_ref()
        .map_or(lk2_core::pvp::CREATURE_HIT_INVULNERABILITY_TICKS, |settings| {
            settings.invulnerability_ticks
        });
    for (guardian, mut attack, defeated) in &mut guardians {
        attack.cooldown_remaining =
            (attack.cooldown_remaining - time.delta_secs().max(0.0)).max(0.0);
        if defeated.is_some()
            || attack.cooldown_remaining > 0.0
            || guardian.translation.distance_squared(player.translation) > 3.0_f32.powi(2)
        {
            continue;
        }
        attack.cooldown_remaining = 1.6;
        let _ = player_health.damage(5.0, state.frame as u32, invulnerability_ticks);
    }
}

pub fn update_starfall_relic(
    travel: Res<DimensionTravelState>,
    progress: Res<StarfallProgress>,
    mut relics: Query<&mut Visibility, With<StarfallRelic>>,
) {
    let visible = travel.current == DimensionId::Starfall
        && progress.collected >= progress.total
        && progress.guardian_defeated
        && !progress.reward_claimed;
    for mut visibility in &mut relics {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn collect_starfall_shards(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<super::keybindings::KeyBindings>,
    travel: Res<DimensionTravelState>,
    mut progress: ResMut<StarfallProgress>,
    mut nature: ResMut<OfflineNature>,
    mut commands: Commands,
    players: Query<&Transform, With<PlayerActor>>,
    shards: Query<(Entity, &Transform), With<StarfallShard>>,
) {
    if travel.current != DimensionId::Starfall
        || bindings.menu_open
        || !bindings.just_pressed(super::keybindings::GameAction::Interact, &keys, &mouse)
    {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let Some((entity, _)) = shards
        .iter()
        .filter(|(_, transform)| {
            transform.translation.distance_squared(player.translation) <= 1.45_f32.powi(2)
        })
        .min_by(|left, right| {
            left.1
                .translation
                .distance_squared(player.translation)
                .total_cmp(&right.1.translation.distance_squared(player.translation))
        })
    else {
        return;
    };
    commands.entity(entity).despawn();
    let _ = nature.resources.try_add(ResourceKind::StarSand, 1);
    progress.collected = progress.collected.saturating_add(1).min(progress.total);
    info!(
        "[dimension] collected star sand {}/{}",
        progress.collected, progress.total
    );
}

pub fn claim_starfall_relic(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<super::keybindings::KeyBindings>,
    travel: Res<DimensionTravelState>,
    mut progress: ResMut<StarfallProgress>,
    mut nature: ResMut<OfflineNature>,
    mut commands: Commands,
    players: Query<&Transform, With<PlayerActor>>,
    relics: Query<(Entity, &Transform), With<StarfallRelic>>,
) {
    if travel.current != DimensionId::Starfall
        || progress.collected < progress.total
        || !progress.guardian_defeated
        || progress.reward_claimed
        || bindings.menu_open
        || !bindings.just_pressed(super::keybindings::GameAction::Interact, &keys, &mouse)
    {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let Some((entity, _)) = relics.iter().find(|(_, transform)| {
        transform.translation.distance_squared(player.translation) <= 2.0_f32.powi(2)
    }) else {
        return;
    };
    commands.entity(entity).despawn();
    let _ = nature.resources.try_add(ResourceKind::RelicCore, 1);
    progress.reward_claimed = true;
    info!("[dimension] claimed the starfall relic");
}

fn toggle_colliders<'a>(
    commands: &mut Commands,
    colliders: impl IntoIterator<Item = (Entity, Option<&'a ColliderDisabled>)>,
    disabled: bool,
) {
    for (entity, collider_disabled) in colliders {
        if disabled && collider_disabled.is_none() {
            commands.entity(entity).insert(ColliderDisabled);
        } else if !disabled && collider_disabled.is_some() {
            commands.entity(entity).remove::<ColliderDisabled>();
        }
    }
}
