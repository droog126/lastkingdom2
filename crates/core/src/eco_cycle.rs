use bevy::prelude::*;

use crate::ecology::{EcologyKind, ResourceNodeKind, WildlifeKind};
use crate::protocol::components::{
    EcoBerryNet, EcoPlantNet, EcoRabbitNet, EcoSnapshot, EcoWildlifeNet, ECO_SNAPSHOT_MAX_BERRIES,
    ECO_SNAPSHOT_MAX_PLANTS, ECO_SNAPSHOT_MAX_RABBITS, ECO_SNAPSHOT_MAX_WILDLIFE,
};
use crate::resource::{GlobalResourcePool, ResourceKind};

pub const RABBIT_CAP: usize = 5;
pub const BERRY_BUSH_CAP: usize = 10;
pub const WILDLIFE_CAP: usize = 12;
pub const PLANT_NODE_CAP: usize = 18;
pub const DEFAULT_ECOLOGY_CENTER: Vec2 = Vec2::new(
    crate::constant::WORLD_SIZE as f32 * 0.62,
    crate::constant::WORLD_SIZE as f32 * 0.43,
);

const RABBIT_SPEED_CELLS_PER_TICK: f32 = 1.6;
const RABBIT_EAT_DISTANCE: f32 = 0.85;
const CO2_PER_CELL_MOVED: f32 = 0.35;
const BERRY_CO2_PER_FRUIT: f32 = 1.0;
const BERRY_MAX_FRUIT: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcoRabbit {
    pub id: u32,
    pub pos: Vec2,
    pub energy: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcoBerryBush {
    pub id: u32,
    pub pos: Vec2,
    pub fruit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcoWildlife {
    pub id: u32,
    pub kind: WildlifeKind,
    pub pos: Vec2,
    pub energy: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcoPlantNode {
    pub id: u32,
    pub kind: ResourceNodeKind,
    pub pos: Vec2,
    pub stock: u32,
}

#[derive(Resource, Debug, Clone)]
pub struct EcoCycle {
    pub rabbits: Vec<EcoRabbit>,
    pub berries: Vec<EcoBerryBush>,
    pub wildlife: Vec<EcoWildlife>,
    pub plants: Vec<EcoPlantNode>,
    pub co2: f32,
    pub fruit_eaten: u64,
    pub fruit_grown: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EcoTickReport {
    pub fruit_eaten: u32,
    pub fruit_grown: u32,
    pub food_produced: i64,
    pub apples_reserved: i64,
}

impl Default for EcoCycle {
    fn default() -> Self {
        Self::demo_at(DEFAULT_ECOLOGY_CENTER)
    }
}

impl EcoCycle {
    pub fn demo_at(center: Vec2) -> Self {
        let rabbits = (0..RABBIT_CAP)
            .map(|i| EcoRabbit {
                id: i as u32,
                pos: center + Vec2::new(-2.0 + i as f32, if i % 2 == 0 { -1.8 } else { 1.8 }),
                energy: 6.0,
            })
            .collect();

        let berries = (0..BERRY_BUSH_CAP)
            .map(|i| {
                let angle = i as f32 / BERRY_BUSH_CAP as f32 * std::f32::consts::TAU;
                let ring = if i % 2 == 0 { 2.0 } else { 3.0 };
                EcoBerryBush {
                    id: i as u32,
                    pos: center + Vec2::new(angle.cos() * ring, angle.sin() * ring),
                    fruit: 1,
                }
            })
            .collect();

        let wildlife_kinds = [
            WildlifeKind::Deer,
            WildlifeKind::Fox,
            WildlifeKind::Bear,
            WildlifeKind::Wolf,
        ];
        let wildlife = wildlife_kinds
            .iter()
            .enumerate()
            .map(|(i, kind)| {
                let angle = i as f32 / wildlife_kinds.len() as f32 * std::f32::consts::TAU + 0.45;
                EcoWildlife {
                    id: i as u32,
                    kind: *kind,
                    pos: center + Vec2::new(angle.cos() * 5.0, angle.sin() * 5.0),
                    energy: 8.0,
                }
            })
            .collect();

        let plant_kinds = [
            ResourceNodeKind::MushroomRed,
            ResourceNodeKind::MushroomBrown,
            ResourceNodeKind::Flower,
            ResourceNodeKind::RockMid,
            ResourceNodeKind::RockMoss,
            ResourceNodeKind::SunstoneCrystal,
            ResourceNodeKind::FrostCrystal,
        ];
        let plants = plant_kinds
            .iter()
            .enumerate()
            .map(|(i, kind)| {
                let angle = i as f32 / plant_kinds.len() as f32 * std::f32::consts::TAU + 0.2;
                let ring = if i % 2 == 0 { 4.0 } else { 6.0 };
                EcoPlantNode {
                    id: i as u32,
                    kind: *kind,
                    pos: center + Vec2::new(angle.cos() * ring, angle.sin() * ring),
                    stock: 1,
                }
            })
            .collect();

        Self { rabbits, berries, wildlife, plants, co2: 0.0, fruit_eaten: 0, fruit_grown: 0 }
    }

    pub fn tick(&mut self, pool: &mut GlobalResourcePool) -> EcoTickReport {
        self.keep_caps();
        self.wander_wildlife();
        let mut report = EcoTickReport::default();
        let berry_targets: Vec<(usize, Vec2)> = self
            .berries
            .iter()
            .enumerate()
            .filter(|(_, berry)| berry.fruit > 0)
            .map(|(idx, berry)| (idx, berry.pos))
            .collect();

        for rabbit in &mut self.rabbits {
            let Some((target_idx, target_pos)) = nearest_target(rabbit.pos, &berry_targets) else {
                continue;
            };

            let delta = target_pos - rabbit.pos;
            let distance = delta.length();
            if distance > f32::EPSILON {
                let step = distance.min(RABBIT_SPEED_CELLS_PER_TICK);
                rabbit.pos += delta / distance * step;
                rabbit.energy = (rabbit.energy - step * 0.08).max(0.0);
                self.co2 += step * CO2_PER_CELL_MOVED;
            }

            if rabbit.pos.distance(target_pos) <= RABBIT_EAT_DISTANCE
                && self.berries[target_idx].fruit > 0
                && pool.try_add(ResourceKind::Food, 1).is_ok()
            {
                self.berries[target_idx].fruit -= 1;
                rabbit.energy = (rabbit.energy + 2.0).min(12.0);
                self.fruit_eaten += 1;
                report.fruit_eaten += 1;
                report.food_produced += 1;
            }
        }

        for berry in &mut self.berries {
            while berry.fruit < BERRY_MAX_FRUIT && self.co2 + f32::EPSILON >= BERRY_CO2_PER_FRUIT {
                if pool.try_sub(ResourceKind::Apple, 1).is_err() {
                    break;
                }
                berry.fruit += 1;
                self.co2 -= BERRY_CO2_PER_FRUIT;
                self.fruit_grown += 1;
                report.fruit_grown += 1;
                report.apples_reserved += 1;
            }
        }
        report
    }

    pub fn rabbit_count(&self) -> usize {
        self.rabbits.len()
    }

    pub fn berry_count(&self) -> usize {
        self.berries.len()
    }

    pub fn total_fruit(&self) -> u32 {
        self.berries.iter().map(|berry| berry.fruit).sum()
    }

    pub fn wildlife_count(&self) -> usize {
        self.wildlife.len()
    }

    pub fn plant_count(&self) -> usize {
        self.plants.len()
    }

    pub fn visible_ecology_kinds(&self) -> Vec<EcologyKind> {
        let mut kinds = Vec::new();
        kinds.extend(self.rabbits.iter().map(|_| EcologyKind::Wildlife(WildlifeKind::Rabbit)));
        kinds.extend(self.wildlife.iter().map(|animal| EcologyKind::Wildlife(animal.kind)));
        kinds.extend(
            self.berries
                .iter()
                .map(|_| EcologyKind::ResourceNode(ResourceNodeKind::BerryBush)),
        );
        kinds.extend(self.plants.iter().map(|plant| EcologyKind::ResourceNode(plant.kind)));
        kinds
    }

    pub fn to_snapshot(&self, tick: u64) -> EcoSnapshot {
        EcoSnapshot {
            tick,
            co2: self.co2,
            fruit_eaten: self.fruit_eaten,
            fruit_grown: self.fruit_grown,
            rabbits: self
                .rabbits
                .iter()
                .take(ECO_SNAPSHOT_MAX_RABBITS)
                .map(|rabbit| EcoRabbitNet {
                    id: rabbit.id,
                    x: rabbit.pos.x,
                    z: rabbit.pos.y,
                    energy: rabbit.energy,
                })
                .collect(),
            wildlife: self
                .wildlife
                .iter()
                .take(ECO_SNAPSHOT_MAX_WILDLIFE)
                .map(|animal| EcoWildlifeNet {
                    id: animal.id,
                    kind: animal.kind.to_u8(),
                    x: animal.pos.x,
                    z: animal.pos.y,
                    energy: animal.energy,
                })
                .collect(),
            berries: self
                .berries
                .iter()
                .take(ECO_SNAPSHOT_MAX_BERRIES)
                .map(|berry| EcoBerryNet {
                    id: berry.id,
                    x: berry.pos.x,
                    z: berry.pos.y,
                    fruit: berry.fruit,
                })
                .collect(),
            plants: self
                .plants
                .iter()
                .take(ECO_SNAPSHOT_MAX_PLANTS)
                .map(|plant| EcoPlantNet {
                    id: plant.id,
                    kind: plant.kind.to_u8(),
                    x: plant.pos.x,
                    z: plant.pos.y,
                    stock: plant.stock,
                })
                .collect(),
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: &EcoSnapshot) {
        self.co2 = snapshot.co2;
        self.fruit_eaten = snapshot.fruit_eaten;
        self.fruit_grown = snapshot.fruit_grown;
        self.rabbits = snapshot
            .rabbits
            .iter()
            .map(|rabbit| EcoRabbit {
                id: rabbit.id,
                pos: Vec2::new(rabbit.x, rabbit.z),
                energy: rabbit.energy,
            })
            .collect();
        self.wildlife = snapshot
            .wildlife
            .iter()
            .map(|animal| EcoWildlife {
                id: animal.id,
                kind: WildlifeKind::from_u8(animal.kind),
                pos: Vec2::new(animal.x, animal.z),
                energy: animal.energy,
            })
            .collect();
        self.berries = snapshot
            .berries
            .iter()
            .map(|berry| EcoBerryBush {
                id: berry.id,
                pos: Vec2::new(berry.x, berry.z),
                fruit: berry.fruit,
            })
            .collect();
        self.plants = snapshot
            .plants
            .iter()
            .map(|plant| EcoPlantNode {
                id: plant.id,
                kind: ResourceNodeKind::from_u8(plant.kind),
                pos: Vec2::new(plant.x, plant.z),
                stock: plant.stock,
            })
            .collect();
        self.keep_caps();
    }

    fn keep_caps(&mut self) {
        self.rabbits.truncate(RABBIT_CAP);
        self.berries.truncate(BERRY_BUSH_CAP);
        self.wildlife.truncate(WILDLIFE_CAP);
        self.plants.truncate(PLANT_NODE_CAP);
    }

    fn wander_wildlife(&mut self) {
        for animal in &mut self.wildlife {
            let phase = animal.id as f32 * 1.37 + animal.energy * 0.19;
            let dir = Vec2::new(phase.cos(), phase.sin());
            animal.pos += dir * 0.08;
            animal.energy = (animal.energy - 0.01).max(0.0);
            self.co2 += 0.02;
        }
    }
}

fn nearest_target(from: Vec2, targets: &[(usize, Vec2)]) -> Option<(usize, Vec2)> {
    targets
        .iter()
        .copied()
        .min_by(|(_, a), (_, b)| from.distance_squared(*a).total_cmp(&from.distance_squared(*b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_starts_with_fixed_population() {
        let eco = EcoCycle::default();

        assert_eq!(eco.rabbit_count(), RABBIT_CAP);
        assert_eq!(eco.berry_count(), BERRY_BUSH_CAP);
        assert_eq!(eco.total_fruit(), BERRY_BUSH_CAP as u32);
        assert!(eco.wildlife_count() >= 4);
        assert!(eco.plant_count() >= 7);
    }

    #[test]
    fn demo_visible_ecology_includes_animals_and_plants() {
        let eco = EcoCycle::default();
        let kinds = eco.visible_ecology_kinds();

        for kind in [
            EcologyKind::Wildlife(WildlifeKind::Rabbit),
            EcologyKind::Wildlife(WildlifeKind::Deer),
            EcologyKind::Wildlife(WildlifeKind::Fox),
            EcologyKind::Wildlife(WildlifeKind::Bear),
            EcologyKind::Wildlife(WildlifeKind::Wolf),
            EcologyKind::ResourceNode(ResourceNodeKind::BerryBush),
            EcologyKind::ResourceNode(ResourceNodeKind::MushroomRed),
            EcologyKind::ResourceNode(ResourceNodeKind::MushroomBrown),
            EcologyKind::ResourceNode(ResourceNodeKind::Flower),
            EcologyKind::ResourceNode(ResourceNodeKind::RockMid),
            EcologyKind::ResourceNode(ResourceNodeKind::RockMoss),
            EcologyKind::ResourceNode(ResourceNodeKind::SunstoneCrystal),
            EcologyKind::ResourceNode(ResourceNodeKind::FrostCrystal),
        ] {
            assert!(kinds.contains(&kind), "missing visible ecology kind {kind:?}");
        }
    }

    #[test]
    fn default_ecology_starts_in_initial_camera_corridor() {
        let eco = EcoCycle::default();
        let player_z = crate::constant::WORLD_SIZE as f32 * 0.43;
        let player_x = crate::constant::WORLD_SIZE as f32 * 0.5;

        let nearby_rabbits = eco
            .rabbits
            .iter()
            .filter(|rabbit| rabbit.pos.x > player_x + 6.0 && (rabbit.pos.y - player_z).abs() <= 4.0)
            .count();
        let nearby_wildlife = eco
            .wildlife
            .iter()
            .filter(|animal| animal.pos.x > player_x + 6.0 && (animal.pos.y - player_z).abs() <= 6.0)
            .count();
        let nearby_plants = eco
            .plants
            .iter()
            .filter(|plant| plant.pos.x > player_x + 6.0 && (plant.pos.y - player_z).abs() <= 7.0)
            .count();

        assert!(nearby_rabbits >= 3, "rabbits should be visible in the initial view");
        assert!(nearby_wildlife >= 2, "wildlife should be visible in the initial view");
        assert!(nearby_plants >= 3, "plant/resource nodes should be visible in the initial view");
    }

    #[test]
    fn rabbits_move_emit_co2_and_eat_fruit() {
        let mut eco = EcoCycle {
            rabbits: vec![EcoRabbit { id: 0, pos: Vec2::ZERO, energy: 6.0 }],
            berries: vec![EcoBerryBush { id: 0, pos: Vec2::new(0.5, 0.0), fruit: 1 }],
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
        };
        let mut pool = GlobalResourcePool::new();

        let report = eco.tick(&mut pool);

        assert_eq!(eco.rabbit_count(), 1);
        assert_eq!(eco.berry_count(), 1);
        assert!(eco.fruit_eaten >= 1);
        assert_eq!(report.food_produced, 1);
        assert_eq!(pool.get(ResourceKind::Food), 1);
        assert!(eco.co2 > 0.0 || eco.fruit_grown > 0);
    }

    #[test]
    fn berries_consume_co2_and_reserved_apples_to_regrow_fruit() {
        let mut eco = EcoCycle {
            rabbits: Vec::new(),
            berries: vec![EcoBerryBush { id: 0, pos: Vec2::ZERO, fruit: 0 }],
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: BERRY_CO2_PER_FRUIT,
            fruit_eaten: 0,
            fruit_grown: 0,
        };
        let mut pool = GlobalResourcePool::new();
        pool.try_add(ResourceKind::Apple, 1).unwrap();

        let report = eco.tick(&mut pool);

        assert_eq!(eco.total_fruit(), 1);
        assert_eq!(eco.fruit_grown, 1);
        assert_eq!(report.apples_reserved, 1);
        assert_eq!(pool.get(ResourceKind::Apple), 0);
        assert!(eco.co2.abs() <= f32::EPSILON);
    }

    #[test]
    fn berries_do_not_regrow_without_available_apples() {
        let mut eco = EcoCycle {
            rabbits: Vec::new(),
            berries: vec![EcoBerryBush { id: 0, pos: Vec2::ZERO, fruit: 0 }],
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: BERRY_CO2_PER_FRUIT,
            fruit_eaten: 0,
            fruit_grown: 0,
        };
        let mut pool = GlobalResourcePool::new();

        let report = eco.tick(&mut pool);

        assert_eq!(eco.total_fruit(), 0);
        assert_eq!(report.fruit_grown, 0);
        assert_eq!(pool.get(ResourceKind::Apple), 0);
        assert!((eco.co2 - BERRY_CO2_PER_FRUIT).abs() <= f32::EPSILON);
    }

    #[test]
    fn rabbits_do_not_eat_when_food_pool_is_full() {
        let mut eco = EcoCycle {
            rabbits: vec![EcoRabbit { id: 0, pos: Vec2::ZERO, energy: 6.0 }],
            berries: vec![EcoBerryBush { id: 0, pos: Vec2::new(0.5, 0.0), fruit: 1 }],
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
        };
        let mut pool = GlobalResourcePool::new();
        pool.try_add(ResourceKind::Food, ResourceKind::Food.max()).unwrap();

        let report = eco.tick(&mut pool);

        assert_eq!(eco.total_fruit(), 1);
        assert_eq!(eco.fruit_eaten, 0);
        assert_eq!(report.food_produced, 0);
        assert_eq!(pool.get(ResourceKind::Food), ResourceKind::Food.max());
    }

    #[test]
    fn tick_preserves_world_caps() {
        let mut eco = EcoCycle::default();
        eco.rabbits.push(EcoRabbit { id: 99, pos: Vec2::ZERO, energy: 1.0 });
        eco.berries.push(EcoBerryBush { id: 99, pos: Vec2::ZERO, fruit: 1 });
        eco.wildlife.push(EcoWildlife {
            id: 99,
            kind: WildlifeKind::Deer,
            pos: Vec2::ZERO,
            energy: 1.0,
        });
        eco.plants.push(EcoPlantNode {
            id: 99,
            kind: ResourceNodeKind::Flower,
            pos: Vec2::ZERO,
            stock: 1,
        });
        let mut pool = GlobalResourcePool::new();

        eco.tick(&mut pool);

        assert_eq!(eco.rabbit_count(), RABBIT_CAP);
        assert_eq!(eco.berry_count(), BERRY_BUSH_CAP);
        assert_eq!(eco.wildlife_count(), WILDLIFE_CAP.min(5));
        assert_eq!(eco.plant_count(), PLANT_NODE_CAP.min(8));
    }

    #[test]
    fn eco_snapshot_round_trips_authoritative_state() {
        let mut source = EcoCycle::default();
        source.co2 = 3.5;
        source.fruit_eaten = 7;
        source.fruit_grown = 4;

        let snapshot = source.to_snapshot(99);
        let mut applied = EcoCycle {
            rabbits: Vec::new(),
            berries: Vec::new(),
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
        };

        applied.apply_snapshot(&snapshot);

        assert_eq!(snapshot.tick, 99);
        assert_eq!(applied.rabbit_count(), source.rabbit_count());
        assert_eq!(applied.wildlife_count(), source.wildlife_count());
        assert_eq!(applied.berry_count(), source.berry_count());
        assert_eq!(applied.plant_count(), source.plant_count());
        assert_eq!(applied.visible_ecology_kinds(), source.visible_ecology_kinds());
        assert_eq!(applied.co2, source.co2);
        assert_eq!(applied.fruit_eaten, source.fruit_eaten);
        assert_eq!(applied.fruit_grown, source.fruit_grown);
    }
}
