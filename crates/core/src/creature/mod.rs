




use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::combat::{
    AttackState, BlockState, Downed, Health as CombatHealth, InputBuffer, Knockback, ParryWindow,
    Stamina, StunState,
};
use crate::player::PlayerState;
use crate::resource::{GlobalResourcePool, PoolError, ResourceKind};
use crate::world::BlockType;
use crate::world::World as GameWorld;





#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreatureKind {
    Pig,
    Sheep,
    Cow,
    Chicken,
}

impl CreatureKind {
    pub fn color(self) -> Color {
        match self {
            CreatureKind::Pig => Color::srgb(0.95, 0.75, 0.78),
            CreatureKind::Sheep => Color::srgb(0.96, 0.96, 0.92),
            CreatureKind::Cow => Color::srgb(0.65, 0.45, 0.30),
            CreatureKind::Chicken => Color::srgb(1.00, 0.90, 0.30),
        }
    }
    pub const fn size(self) -> Vec3 {
        match self {
            CreatureKind::Pig => Vec3::new(0.58, 0.44, 0.78),
            CreatureKind::Sheep => Vec3::new(0.62, 0.58, 0.78),
            CreatureKind::Cow => Vec3::new(0.78, 0.70, 0.98),
            CreatureKind::Chicken => Vec3::new(0.34, 0.42, 0.36),
        }
    }
    pub fn label_zh(self) -> &'static str {
        match self {
            CreatureKind::Pig => "猪",
            CreatureKind::Sheep => "羊",
            CreatureKind::Cow => "牛",
            CreatureKind::Chicken => "鸡",
        }
    }
}

pub fn creature_material(kind: CreatureKind) -> StandardMaterial {
    let color = kind.color();
    StandardMaterial {
        base_color: color,
        emissive: (color.to_linear() * 0.35).into(),
        unlit: true,
        perceptual_roughness: 0.85,
        metallic: 0.0,
        ..default()
    }
}

pub const fn creature_drop_kind(kind: CreatureKind) -> ResourceKind {
    match kind {
        CreatureKind::Pig | CreatureKind::Sheep | CreatureKind::Cow => ResourceKind::Food,
        CreatureKind::Chicken => ResourceKind::Apple,
    }
}

pub fn award_creature_drop(
    pool: &mut GlobalResourcePool,
    player: &mut PlayerState,
    kind: CreatureKind,
) -> Result<ResourceKind, PoolError> {
    let drop = creature_drop_kind(kind);
    pool.try_add(drop, 3)?;
    player.monsters_killed += 1;
    Ok(drop)
}





pub const CREATURE_TRAINING_ATTACK_RANGE_SQ: f32 = 25.0;

pub fn creature_attack_distance_sq(
    player_block_pos: [i32; 3],
    creature_block_pos: [i32; 3],
) -> f32 {
    let dx = (creature_block_pos[0] - player_block_pos[0]) as f32;
    let dz = (creature_block_pos[2] - player_block_pos[2]) as f32;
    dx * dx + dz * dz
}

#[derive(Component)]
pub struct Creature {
    pub kind: CreatureKind,
    pub block_pos: [i32; 3],
}

#[derive(Component)]
pub struct CreatureAI {
    pub wander_timer: f32,
    pub next_wander_secs: f32,
    pub bob_phase: f32,
}





#[derive(Resource, Default)]
pub struct CreatureSpawnerDone(pub bool);





pub fn spawn_creatures(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<GameWorld>,
    mut done: ResMut<CreatureSpawnerDone>,
) {
    if done.0 {
        return;
    }
    done.0 = true;

    let s = world.size;
    let mut rng = StdRng::seed_from_u64(0xC0FF_EE01);
    let kinds = [
        CreatureKind::Pig,
        CreatureKind::Sheep,
        CreatureKind::Cow,
        CreatureKind::Chicken,
    ];
    let count = 30;
    let mut placed = 0;
    let mut attempts = 0;
    let spawn_cx = s / 2;
    let spawn_cz = s / 2;

    if try_spawn_training_creature_near_spawn(
        &mut commands,
        &mut meshes,
        &mut materials,
        &world,
        &kinds,
        spawn_cx,
        spawn_cz,
        &mut rng,
    ) {
        placed += 1;
    } else if spawn_debug_training_creature(
        &mut commands,
        &mut meshes,
        &mut materials,
        spawn_cx,
        spawn_cz,
    ) {
        placed += 1;
    }


    let mut starter_attempts = 0;
    while placed < 12 && starter_attempts < 400 {
        starter_attempts += 1;
        let x = spawn_cx + rng.random_range(-8..9);
        let z = spawn_cz + rng.random_range(-8..9);
        if try_spawn_creature(
            &mut commands,
            &mut meshes,
            &mut materials,
            &world,
            &kinds,
            x,
            z,
            None,
            None,
            &mut rng,
        ) {
            placed += 1;
        }
    }
    info!("🐄 起始牧场 spawn {} 只", placed);


    let target = count;
    while placed < target && attempts < count * 20 {
        attempts += 1;
        let x = rng.random_range(2..(s - 2));
        let z = rng.random_range(2..(s - 2));

        if (x - spawn_cx).abs() + (z - spawn_cz).abs() < 8 {
            continue;
        }
        if try_spawn_creature(
            &mut commands,
            &mut meshes,
            &mut materials,
            &world,
            &kinds,
            x,
            z,
            None,
            None,
            &mut rng,
        ) {
            placed += 1;
        }
    }
    info!("🐄 总共 spawn {} 只动物 (尝试 {} 次)", placed, attempts);
}


fn try_spawn_training_creature_near_spawn(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    world: &GameWorld,
    kinds: &[CreatureKind; 4],
    spawn_cx: i32,
    spawn_cz: i32,
    rng: &mut StdRng,
) -> bool {
    const OFFSETS: &[(i32, i32)] = &[
        (0, -1),
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -2),
        (2, 0),
        (-2, 0),
        (0, 2),
        (1, -1),
        (-1, -1),
        (1, 1),
        (-1, 1),
    ];
    for (dx, dz) in OFFSETS {
        if try_spawn_creature(
            commands,
            meshes,
            materials,
            world,
            kinds,
            spawn_cx + dx,
            spawn_cz + dz,
            Some(CreatureKind::Cow),
            Some(9999.0),
            rng,
        ) {
            return true;
        }
    }
    false
}

fn spawn_debug_training_creature(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    spawn_cx: i32,
    spawn_cz: i32,
) -> bool {
    spawn_debug_creature_at(
        commands,
        meshes,
        materials,
        spawn_cx,
        crate::constant::SEA_LEVEL + 4,
        spawn_cz - 2,
        CreatureKind::Cow,
        9999.0,
    )
}

fn spawn_debug_creature_at(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    x: i32,
    y: i32,
    z: i32,
    kind: CreatureKind,
    fixed_wander_secs: f32,
) -> bool {
    let mesh_h = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mat_h = materials.add(creature_material(kind));
    commands.spawn((
        Creature { kind, block_pos: [x, y, z] },
        CreatureAI { wander_timer: 0.0, next_wander_secs: fixed_wander_secs, bob_phase: 0.0 },
        Mesh3d(mesh_h),
        MeshMaterial3d(mat_h),
        Transform::from_translation(Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5))
            .with_scale(kind.size()),
        CombatHealth { current: 12.0, max: 12.0, invuln_until_tick: 0 },
        Stamina::default(),
        BlockState::default(),
        ParryWindow::default(),
        StunState::default(),
        Knockback::default(),
        AttackState::default(),
        Downed::default(),
        InputBuffer::default(),
    ));
    true
}

fn try_spawn_creature(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    world: &GameWorld,
    kinds: &[CreatureKind; 4],
    x: i32,
    z: i32,
    fixed_kind: Option<CreatureKind>,
    fixed_wander_secs: Option<f32>,
    rng: &mut StdRng,
) -> bool {
    let s = world.size;
    if x < 2 || x >= s - 2 || z < 2 || z >= s - 2 {
        return false;
    }

    let mut surface_y = None;
    for y in (1..s).rev() {
        if world.get(x, y, z).is_solid() {
            surface_y = Some(y);
            break;
        }
    }
    let Some(y) = surface_y else {
        return false;
    };
    if !world.get(x, y, z).is_surface() {
        return false;
    }
    let y = y + 1;

    let kind = fixed_kind.unwrap_or_else(|| kinds[rng.random_range(0..kinds.len())]);

    spawn_debug_creature_at(
        commands,
        meshes,
        materials,
        x,
        y,
        z,
        kind,
        fixed_wander_secs.unwrap_or_else(|| rng.random_range(0.5..1.5)),
    )
}

pub fn despawn_dead_creatures(
    mut commands: Commands,
    mut pool: ResMut<crate::resource::GlobalResourcePool>,
    mut player: ResMut<PlayerState>,
    q: Query<(Entity, &Creature, &CombatHealth)>,
) {
    for (entity, creature, health) in q.iter() {
        if !health.is_dead() {
            continue;
        }
        if let Err(err) = award_creature_drop(&mut pool, &mut player, creature.kind) {
            warn!(
                "[creature] failed to award drop for dead {:?}: {}",
                creature.kind, err
            );
        }
        commands.entity(entity).despawn();
    }
}



pub fn player_attack_creatures(
    keys: Res<ButtonInput<KeyCode>>,
    mut player: ResMut<PlayerState>,
    mut pool: ResMut<crate::resource::GlobalResourcePool>,
    mut commands: Commands,
    q: Query<(Entity, &Creature, &CreatureAI)>,
) {
    if !keys.just_pressed(KeyCode::KeyK) {
        return;
    }

    let mut best: Option<(Entity, f32, CreatureKind)> = None;
    for (e, c, _) in q.iter() {
        let d2 = creature_attack_distance_sq(player.block_pos, c.block_pos);
        if d2 <= CREATURE_TRAINING_ATTACK_RANGE_SQ && (best.is_none() || d2 < best.unwrap().1) {
            best = Some((e, d2, c.kind));
        }
    }
    if let Some((e, _d, kind)) = best {

        match award_creature_drop(&mut pool, &mut player, kind) {
            Ok(drop) => {
                info!("⚔ 你杀了一只{}（+3 {:?}）", kind.label_zh(), drop);
            }
            Err(err) => {
                warn!(
                    "[creature] failed to award drop for attacked {:?}: {}",
                    kind, err
                );
            }
        }
        commands.entity(e).despawn();
    } else {
        info!("⚔ 挥空（范围内没有动物）");
    }
}





pub fn update_creatures(
    time: Res<Time>,
    world: Res<GameWorld>,
    mut rng: Local<Option<StdRng>>,
    mut q: Query<(&mut Creature, &mut CreatureAI, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let rng = rng.get_or_insert_with(|| StdRng::seed_from_u64(0xC0FF_EE02));
    for (mut creature, mut ai, mut tf) in q.iter_mut() {
        ai.wander_timer += dt;
        ai.bob_phase += dt * 3.0;
        let bob = (ai.bob_phase).sin() * 0.05;
        tf.translation.y = creature.block_pos[1] as f32 + 0.5 + bob;

        if ai.wander_timer < ai.next_wander_secs {
            continue;
        }
        ai.wander_timer = 0.0;
        ai.next_wander_secs = rng.random_range(1.5..3.0);


        let dirs: [[i32; 3]; 4] = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]];
        let d = dirs[rng.random_range(0..dirs.len())];
        let nx = creature.block_pos[0] + d[0];
        let ny = creature.block_pos[1] + d[1];
        let nz = creature.block_pos[2] + d[2];
        if !world.in_bounds(nx, ny, nz) {
            continue;
        }

        if world.get(nx, ny, nz) != BlockType::Air {
            continue;
        }
        if !world.get(nx, ny - 1, nz).is_solid() {
            continue;
        }

        let yaw = match (d[0], d[2]) {
            (1, 0) => std::f32::consts::FRAC_PI_2,
            (-1, 0) => -std::f32::consts::FRAC_PI_2,
            (0, 1) => std::f32::consts::PI,
            (0, -1) => 0.0,
            _ => 0.0,
        };
        creature.block_pos = [nx, ny, nz];
        tf.translation = Vec3::new(nx as f32 + 0.5, ny as f32 + 0.5 + bob, nz as f32 + 0.5);
        tf.rotation = Quat::from_rotation_y(yaw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creature_drop_kind_matches_food_rules() {
        assert_eq!(creature_drop_kind(CreatureKind::Pig), ResourceKind::Food);
        assert_eq!(creature_drop_kind(CreatureKind::Sheep), ResourceKind::Food);
        assert_eq!(creature_drop_kind(CreatureKind::Cow), ResourceKind::Food);
        assert_eq!(
            creature_drop_kind(CreatureKind::Chicken),
            ResourceKind::Apple
        );
    }

    #[test]
    fn award_creature_drop_adds_resources_and_kill_count() {
        let mut pool = GlobalResourcePool::new();
        let mut player = PlayerState::default();

        let drop = award_creature_drop(&mut pool, &mut player, CreatureKind::Cow).unwrap();

        assert_eq!(drop, ResourceKind::Food);
        assert_eq!(pool.get(ResourceKind::Food), 3);
        assert_eq!(player.monsters_killed, 1);
    }

    #[test]
    fn award_creature_drop_chicken_adds_apple_and_accumulates_kill_count() {
        let mut pool = GlobalResourcePool::new();
        let mut player = PlayerState::default();

        let first = award_creature_drop(&mut pool, &mut player, CreatureKind::Chicken).unwrap();
        let second = award_creature_drop(&mut pool, &mut player, CreatureKind::Pig).unwrap();

        assert_eq!(first, ResourceKind::Apple);
        assert_eq!(second, ResourceKind::Food);
        assert_eq!(pool.get(ResourceKind::Apple), 3);
        assert_eq!(pool.get(ResourceKind::Food), 3);
        assert_eq!(player.monsters_killed, 2);
    }

    #[test]
    fn award_creature_drop_reports_full_pool_without_counting_kill() {
        let mut pool = GlobalResourcePool::new();
        let mut player = PlayerState::default();
        pool.try_add(ResourceKind::Food, ResourceKind::Food.max()).unwrap();

        let err = award_creature_drop(&mut pool, &mut player, CreatureKind::Cow).unwrap_err();

        assert!(matches!(
            err,
            PoolError::WouldExceedMax { kind: ResourceKind::Food, .. }
        ));
        assert_eq!(pool.get(ResourceKind::Food), ResourceKind::Food.max());
        assert_eq!(player.monsters_killed, 0);
    }

    #[test]
    fn creature_attack_distance_covers_training_target_two_blocks_away() {
        let d2 = creature_attack_distance_sq([48, 16, 48], [48, 16, 46]);
        assert_eq!(d2, 4.0);
        assert!(d2 <= CREATURE_TRAINING_ATTACK_RANGE_SQ);
    }

    #[test]
    fn creature_material_keeps_small_animals_readable() {
        let material = creature_material(CreatureKind::Cow);

        assert_eq!(material.base_color, CreatureKind::Cow.color());
        assert!(material.unlit);
        assert_eq!(material.metallic, 0.0);
    }

    #[test]
    fn creature_combat_health_uses_small_animal_hp() {
        let health = CombatHealth { current: 12.0, max: 12.0, invuln_until_tick: 0 };
        assert!(!health.is_dead());
        assert_eq!(health.current, health.max);
    }

    #[test]
    fn creature_combat_health_dead_at_zero() {
        let health = CombatHealth { current: 0.0, max: 12.0, invuln_until_tick: 0 };
        assert!(health.is_dead());
    }
}
