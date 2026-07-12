use bevy::prelude::*;

use crate::ecology::{EcologyKind, ResourceNodeKind, WildlifeKind};
use crate::protocol::components::{
    ECO_SNAPSHOT_MAX_BERRIES, ECO_SNAPSHOT_MAX_CLOUDS, ECO_SNAPSHOT_MAX_PLANTS,
    ECO_SNAPSHOT_MAX_RABBITS, ECO_SNAPSHOT_MAX_WILDLIFE, EcoBerryNet, EcoCloudNet, EcoPlantNet,
    EcoRabbitNet, EcoSnapshot, EcoWildlifeNet,
};
use crate::resource::{GlobalResourcePool, ResourceKind};

pub const RABBIT_CAP: usize = 5;
pub const BERRY_BUSH_CAP: usize = 10;
pub const WILDLIFE_CAP: usize = 12;
pub const PLANT_NODE_CAP: usize = 18;
pub const CLOUD_CAP: usize = 3;
pub const DEFAULT_ECOLOGY_CENTER: Vec2 = Vec2::new(
    crate::constant::WORLD_SIZE as f32 * 0.62,
    crate::constant::WORLD_SIZE as f32 * 0.43,
);

const RABBIT_SPEED_CELLS_PER_TICK: f32 = 1.6;
const RABBIT_EAT_DISTANCE: f32 = 0.85;
const CO2_PER_CELL_MOVED: f32 = 0.35;
const BERRY_CO2_PER_FRUIT: f32 = 1.0;
const BERRY_MAX_FRUIT: u32 = 3;
const RAIN_PER_CLOUD_TICK: f32 = 0.34;
const RAIN_PER_PLANT: f32 = 1.0;
const RAIN_PER_BERRY_BUSH: f32 = 1.4;
const BERRY_BUSHES_PER_RABBIT: usize = 1;
const RABBITS_PER_WILDLIFE: usize = 3;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcoCloud {
    pub id: u32,
    pub pos: Vec2,
    pub rain: f32,
    pub phase: f32,
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct EcoCycle {
    pub clouds: Vec<EcoCloud>,
    pub rabbits: Vec<EcoRabbit>,
    pub berries: Vec<EcoBerryBush>,
    pub wildlife: Vec<EcoWildlife>,
    pub plants: Vec<EcoPlantNode>,
    pub co2: f32,
    pub rain: f32,
    pub rainfall: f32,
    pub fruit_eaten: u64,
    pub fruit_grown: u64,
    pub plants_grown: u64,
    pub rabbits_born: u64,
    pub wildlife_born: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EcoTickReport {
    pub rain_fell: f32,
    pub plants_grown: u32,
    pub rabbits_born: u32,
    pub wildlife_born: u32,
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

#[cfg(test)]
mod smoke_tests {
    use super::*;

    #[test]
    fn eco_cycle_default_initializes() {
        let ec = EcoCycle::default();
        assert_eq!(ec.clouds.len(), CLOUD_CAP);
        assert_eq!(ec.rabbits.len(), RABBIT_CAP);
        assert_eq!(ec.berries.len(), BERRY_BUSH_CAP);
        assert!(ec.wildlife.len() > 0);
        assert!(ec.plants.len() > 0);
    }

    #[test]
    fn eco_cycle_demo_at_creates_at_center() {
        let center = Vec2::new(50.0, 50.0);
        let ec = EcoCycle::demo_at(center);

        assert!(!ec.clouds.is_empty());
        assert!(!ec.rabbits.is_empty());
        assert!(!ec.berries.is_empty());

        for rabbit in &ec.rabbits {
            assert!((rabbit.pos - center).length() < 10.0);
        }
    }

    #[test]
    fn eco_rabbit_fields() {
        let r = EcoRabbit {
            id: 1,
            pos: Vec2::new(10.0, 20.0),
            energy: 5.0,
        };
        assert_eq!(r.id, 1);
        assert_eq!(r.pos, Vec2::new(10.0, 20.0));
        assert!((r.energy - 5.0).abs() < 0.001);
    }

    #[test]
    fn eco_berry_bush_fields() {
        let b = EcoBerryBush {
            id: 2,
            pos: Vec2::new(5.0, 5.0),
            fruit: 3,
        };
        assert_eq!(b.id, 2);
        assert_eq!(b.pos, Vec2::new(5.0, 5.0));
        assert_eq!(b.fruit, 3);
    }

    #[test]
    fn eco_wildlife_fields() {
        let w = EcoWildlife {
            id: 3,
            kind: WildlifeKind::Deer,
            pos: Vec2::new(15.0, 25.0),
            energy: 8.0,
        };
        assert_eq!(w.id, 3);
        assert_eq!(w.kind, WildlifeKind::Deer);
        assert_eq!(w.pos, Vec2::new(15.0, 25.0));
        assert!((w.energy - 8.0).abs() < 0.001);
    }

    #[test]
    fn eco_plant_node_fields() {
        let p = EcoPlantNode {
            id: 4,
            kind: ResourceNodeKind::BerryBush,
            pos: Vec2::new(8.0, 12.0),
            stock: 5,
        };
        assert_eq!(p.id, 4);
        assert_eq!(p.kind, ResourceNodeKind::BerryBush);
        assert_eq!(p.pos, Vec2::new(8.0, 12.0));
        assert_eq!(p.stock, 5);
    }

    #[test]
    fn eco_cloud_fields() {
        let c = EcoCloud {
            id: 5,
            pos: Vec2::new(20.0, 30.0),
            rain: 1.0,
            phase: 0.5,
        };
        assert_eq!(c.id, 5);
        assert_eq!(c.pos, Vec2::new(20.0, 30.0));
        assert!((c.rain - 1.0).abs() < 0.001);
        assert!((c.phase - 0.5).abs() < 0.001);
    }

    #[test]
    fn eco_tick_report_default() {
        let r = EcoTickReport::default();
        assert_eq!(r.rain_fell, 0.0);
        assert_eq!(r.plants_grown, 0);
        assert_eq!(r.rabbits_born, 0);
        assert_eq!(r.wildlife_born, 0);
        assert_eq!(r.fruit_eaten, 0);
        assert_eq!(r.fruit_grown, 0);
        assert_eq!(r.food_produced, 0);
        assert_eq!(r.apples_reserved, 0);
    }

    #[test]
    fn eco_tick_report_fields() {
        let r = EcoTickReport {
            rain_fell: 5.0,
            plants_grown: 10,
            rabbits_born: 2,
            wildlife_born: 1,
            fruit_eaten: 3,
            fruit_grown: 5,
            food_produced: 20,
            apples_reserved: 15,
        };
        assert!((r.rain_fell - 5.0).abs() < 0.001);
        assert_eq!(r.plants_grown, 10);
        assert_eq!(r.rabbits_born, 2);
        assert_eq!(r.wildlife_born, 1);
        assert_eq!(r.fruit_eaten, 3);
        assert_eq!(r.fruit_grown, 5);
        assert_eq!(r.food_produced, 20);
        assert_eq!(r.apples_reserved, 15);
    }

    #[test]
    fn eco_constants_are_reasonable() {
        assert_eq!(RABBIT_CAP, 5);
        assert_eq!(BERRY_BUSH_CAP, 10);
        assert_eq!(WILDLIFE_CAP, 12);
        assert_eq!(PLANT_NODE_CAP, 18);
        assert_eq!(CLOUD_CAP, 3);
        assert!(RABBIT_SPEED_CELLS_PER_TICK > 0.0);
        assert!(BERRY_MAX_FRUIT > 0);
    }
}

impl EcoCycle {
    pub fn demo_at(center: Vec2) -> Self {
        let clouds = (0..CLOUD_CAP)
            .map(|i| {
                let angle = i as f32 / CLOUD_CAP as f32 * std::f32::consts::TAU + 0.4;
                EcoCloud {
                    id: i as u32,
                    pos: center + Vec2::new(angle.cos() * 12.0, angle.sin() * 8.0),
                    rain: 0.55,
                    phase: angle,
                }
            })
            .collect();

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

        Self {
            clouds,
            rabbits,
            berries,
            wildlife,
            plants,
            co2: 0.0,
            rain: 0.0,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
        }
    }

    pub fn seeded_weather_at(center: Vec2) -> Self {
        Self {
            clouds: vec![EcoCloud {
                id: 0,
                pos: center,
                rain: 1.0,
                phase: 0.0,
            }],
            rabbits: Vec::new(),
            berries: Vec::new(),
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: 0.0,
            rain: 0.0,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
        }
    }

    pub fn tick(&mut self, pool: &mut GlobalResourcePool) -> EcoTickReport {
        self.keep_caps();
        let mut report = EcoTickReport::default();
        self.advance_clouds(&mut report);
        self.grow_plants_from_rain(&mut report);
        self.spawn_rabbits_from_berries(&mut report);
        self.spawn_wildlife_from_rabbits(&mut report);
        self.wander_wildlife();
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
        kinds.extend(
            self.rabbits
                .iter()
                .map(|_| EcologyKind::Wildlife(WildlifeKind::Rabbit)),
        );
        kinds.extend(
            self.wildlife
                .iter()
                .map(|animal| EcologyKind::Wildlife(animal.kind)),
        );
        kinds.extend(
            self.berries
                .iter()
                .map(|_| EcologyKind::ResourceNode(ResourceNodeKind::BerryBush)),
        );
        kinds.extend(
            self.plants
                .iter()
                .map(|plant| EcologyKind::ResourceNode(plant.kind)),
        );
        kinds
    }

    pub fn to_snapshot(&self, tick: u64) -> EcoSnapshot {
        EcoSnapshot {
            tick,
            co2: self.co2,
            rain: self.rain,
            rainfall: self.rainfall,
            fruit_eaten: self.fruit_eaten,
            fruit_grown: self.fruit_grown,
            plants_grown: self.plants_grown,
            rabbits_born: self.rabbits_born,
            wildlife_born: self.wildlife_born,
            clouds: self
                .clouds
                .iter()
                .take(ECO_SNAPSHOT_MAX_CLOUDS)
                .map(|cloud| EcoCloudNet {
                    id: cloud.id,
                    x: cloud.pos.x,
                    z: cloud.pos.y,
                    rain: cloud.rain,
                    phase: cloud.phase,
                })
                .collect(),
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
        self.rain = snapshot.rain;
        self.rainfall = snapshot.rainfall;
        self.fruit_eaten = snapshot.fruit_eaten;
        self.fruit_grown = snapshot.fruit_grown;
        self.plants_grown = snapshot.plants_grown;
        self.rabbits_born = snapshot.rabbits_born;
        self.wildlife_born = snapshot.wildlife_born;
        self.clouds = snapshot
            .clouds
            .iter()
            .map(|cloud| EcoCloud {
                id: cloud.id,
                pos: Vec2::new(cloud.x, cloud.z),
                rain: cloud.rain,
                phase: cloud.phase,
            })
            .collect();
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
        self.clouds.truncate(CLOUD_CAP);
        self.rabbits.truncate(RABBIT_CAP);
        self.berries.truncate(BERRY_BUSH_CAP);
        self.wildlife.truncate(WILDLIFE_CAP);
        self.plants.truncate(PLANT_NODE_CAP);
    }

    fn advance_clouds(&mut self, report: &mut EcoTickReport) {
        for cloud in &mut self.clouds {
            cloud.phase += 0.23;
            cloud.pos += Vec2::new(0.18 + cloud.phase.sin() * 0.04, cloud.phase.cos() * 0.06);
            let rain = cloud.rain.max(0.0) * RAIN_PER_CLOUD_TICK;
            self.rain += rain;
            self.rainfall += rain;
            report.rain_fell += rain;
            cloud.rain = (cloud.rain + 0.03).min(1.0);
        }
    }

    fn grow_plants_from_rain(&mut self, report: &mut EcoTickReport) {
        // Establish the first two plant nodes before allocating rain to the
        // berry stage. Once berries are full, remaining rain can grow plants.
        while self.plants.len() < PLANT_NODE_CAP
            && (self.plants.len() < 2 || self.berries.len() >= BERRY_BUSH_CAP)
            && self.rain + f32::EPSILON >= RAIN_PER_PLANT
        {
            let id = next_id_for(self.plants.iter().map(|plant| plant.id));
            let kind = if id % 3 == 0 {
                ResourceNodeKind::Flower
            } else if id % 2 == 0 {
                ResourceNodeKind::MushroomBrown
            } else {
                ResourceNodeKind::MushroomRed
            };
            let origin = self.cloud_anchor();
            self.plants.push(EcoPlantNode {
                id,
                kind,
                pos: origin + deterministic_ring_offset(id, 3.2, 0.53),
                stock: 1,
            });
            self.rain -= RAIN_PER_PLANT;
            self.plants_grown += 1;
            report.plants_grown += 1;
        }

        while self.berries.len() < BERRY_BUSH_CAP
            && self.plants.len() >= 2
            && self.rain + f32::EPSILON >= RAIN_PER_BERRY_BUSH
        {
            let id = next_id_for(self.berries.iter().map(|berry| berry.id));
            let origin = self.cloud_anchor();
            self.berries.push(EcoBerryBush {
                id,
                pos: origin + deterministic_ring_offset(id, 4.4, 1.1),
                fruit: 1,
            });
            self.rain -= RAIN_PER_BERRY_BUSH;
            self.fruit_grown += 1;
            report.fruit_grown += 1;
        }
    }

    fn spawn_rabbits_from_berries(&mut self, report: &mut EcoTickReport) {
        while self.rabbits.len() < RABBIT_CAP
            && self.berries.len() / BERRY_BUSHES_PER_RABBIT > self.rabbits.len()
        {
            let id = next_id_for(self.rabbits.iter().map(|rabbit| rabbit.id));
            let origin = self.berries[id as usize % self.berries.len()].pos;
            self.rabbits.push(EcoRabbit {
                id,
                pos: origin + deterministic_ring_offset(id, 1.1, 1.7),
                energy: 5.0,
            });
            self.rabbits_born += 1;
            report.rabbits_born += 1;
        }
    }

    fn spawn_wildlife_from_rabbits(&mut self, report: &mut EcoTickReport) {
        while self.wildlife.len() < WILDLIFE_CAP
            && self.rabbits.len() / RABBITS_PER_WILDLIFE > self.wildlife.len()
        {
            let id = next_id_for(self.wildlife.iter().map(|animal| animal.id));
            let kind = match id % 4 {
                0 => WildlifeKind::Wolf,
                1 => WildlifeKind::Deer,
                2 => WildlifeKind::Fox,
                _ => WildlifeKind::Bear,
            };
            let origin = self.rabbits[id as usize % self.rabbits.len()].pos;
            self.wildlife.push(EcoWildlife {
                id,
                kind,
                pos: origin + deterministic_ring_offset(id, 2.6, 2.3),
                energy: 7.0,
            });
            self.wildlife_born += 1;
            report.wildlife_born += 1;
        }
    }

    fn cloud_anchor(&self) -> Vec2 {
        if self.clouds.is_empty() {
            DEFAULT_ECOLOGY_CENTER
        } else {
            self.clouds.iter().map(|cloud| cloud.pos).sum::<Vec2>() / self.clouds.len() as f32
        }
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
    targets.iter().copied().min_by(|(_, a), (_, b)| {
        from.distance_squared(*a)
            .total_cmp(&from.distance_squared(*b))
    })
}

fn next_id_for(ids: impl Iterator<Item = u32>) -> u32 {
    ids.max().map(|id| id + 1).unwrap_or(0)
}

fn deterministic_ring_offset(id: u32, radius: f32, phase: f32) -> Vec2 {
    let angle = id as f32 * 1.618_034 + phase;
    Vec2::new(angle.cos() * radius, angle.sin() * radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_starts_with_fixed_population() {
        let eco = EcoCycle::default();

        assert_eq!(eco.clouds.len(), CLOUD_CAP);
        assert_eq!(eco.rabbit_count(), RABBIT_CAP);
        assert_eq!(eco.berry_count(), BERRY_BUSH_CAP);
        assert_eq!(eco.total_fruit(), BERRY_BUSH_CAP as u32);
        assert!(eco.wildlife_count() >= 4);
        assert!(eco.plant_count() >= 7);
    }

    #[test]
    fn clouds_rain_then_plants_then_small_and_large_animals() {
        let mut eco = EcoCycle::seeded_weather_at(Vec2::new(20.0, 20.0));
        let mut pool = GlobalResourcePool::new();

        assert_eq!(eco.plant_count(), 0);
        assert_eq!(eco.rabbit_count(), 0);
        assert_eq!(eco.wildlife_count(), 0);

        let mut saw_rain = false;
        let mut saw_plants = false;
        let mut saw_rabbits = false;
        let mut saw_wildlife = false;

        for _ in 0..150 {
            let report = eco.tick(&mut pool);
            saw_rain |= report.rain_fell > 0.0;
            saw_plants |= report.plants_grown > 0;
            saw_rabbits |= report.rabbits_born > 0;
            saw_wildlife |= report.wildlife_born > 0;
        }

        assert!(saw_rain, "clouds should produce rain");
        assert!(saw_plants, "rain should grow grass/flower resource nodes");
        assert!(saw_rabbits, "berry bushes should support small animals");
        assert!(saw_wildlife, "small animals should support larger wildlife");
        assert!(eco.rainfall > 0.0);
        assert!(eco.berry_count() >= BERRY_BUSHES_PER_RABBIT);
        assert!(eco.rabbit_count() >= RABBITS_PER_WILDLIFE);
        assert!(eco.wildlife_count() >= 1);
        assert_eq!(
            eco.wildlife.first().map(|animal| animal.kind),
            Some(WildlifeKind::Wolf)
        );
    }

    #[test]
    fn rain_does_not_spawn_animals_before_plants() {
        let mut eco = EcoCycle {
            clouds: Vec::new(),
            rabbits: Vec::new(),
            berries: Vec::new(),
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: 0.0,
            rain: RAIN_PER_PLANT - 0.1,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
        };
        let mut pool = GlobalResourcePool::new();

        let report = eco.tick(&mut pool);

        assert_eq!(report.plants_grown, 0);
        assert_eq!(report.rabbits_born, 0);
        assert_eq!(report.wildlife_born, 0);
        assert_eq!(eco.rabbit_count(), 0);
        assert_eq!(eco.wildlife_count(), 0);
    }

    #[test]
    fn plants_do_not_spawn_rabbits_before_berry_bushes_exist() {
        let mut eco = EcoCycle {
            clouds: Vec::new(),
            rabbits: Vec::new(),
            berries: Vec::new(),
            wildlife: Vec::new(),
            plants: vec![
                EcoPlantNode {
                    id: 0,
                    kind: ResourceNodeKind::Flower,
                    pos: Vec2::ZERO,
                    stock: 1,
                },
                EcoPlantNode {
                    id: 1,
                    kind: ResourceNodeKind::MushroomRed,
                    pos: Vec2::X,
                    stock: 1,
                },
            ],
            co2: 0.0,
            rain: 0.0,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
        };

        let report = eco.tick(&mut GlobalResourcePool::new());

        assert_eq!(report.rabbits_born, 0);
        assert!(eco.rabbits.is_empty());
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
            assert!(
                kinds.contains(&kind),
                "missing visible ecology kind {kind:?}"
            );
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
            .filter(|rabbit| {
                rabbit.pos.x > player_x + 6.0 && (rabbit.pos.y - player_z).abs() <= 4.0
            })
            .count();
        let nearby_wildlife = eco
            .wildlife
            .iter()
            .filter(|animal| {
                animal.pos.x > player_x + 6.0 && (animal.pos.y - player_z).abs() <= 6.0
            })
            .count();
        let nearby_plants = eco
            .plants
            .iter()
            .filter(|plant| plant.pos.x > player_x + 6.0 && (plant.pos.y - player_z).abs() <= 7.0)
            .count();

        assert!(
            nearby_rabbits >= 3,
            "rabbits should be visible in the initial view"
        );
        assert!(
            nearby_wildlife >= 2,
            "wildlife should be visible in the initial view"
        );
        assert!(
            nearby_plants >= 3,
            "plant/resource nodes should be visible in the initial view"
        );
    }

    #[test]
    fn rabbits_move_emit_co2_and_eat_fruit() {
        let mut eco = EcoCycle {
            rabbits: vec![EcoRabbit {
                id: 0,
                pos: Vec2::ZERO,
                energy: 6.0,
            }],
            berries: vec![EcoBerryBush {
                id: 0,
                pos: Vec2::new(0.5, 0.0),
                fruit: 1,
            }],
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: 0.0,
            rain: 0.0,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
            clouds: Vec::new(),
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
            berries: vec![EcoBerryBush {
                id: 0,
                pos: Vec2::ZERO,
                fruit: 0,
            }],
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: BERRY_CO2_PER_FRUIT,
            rain: 0.0,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
            clouds: Vec::new(),
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
            berries: vec![EcoBerryBush {
                id: 0,
                pos: Vec2::ZERO,
                fruit: 0,
            }],
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: BERRY_CO2_PER_FRUIT,
            rain: 0.0,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
            clouds: Vec::new(),
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
            rabbits: vec![EcoRabbit {
                id: 0,
                pos: Vec2::ZERO,
                energy: 6.0,
            }],
            berries: vec![EcoBerryBush {
                id: 0,
                pos: Vec2::new(0.5, 0.0),
                fruit: 1,
            }],
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: 0.0,
            rain: 0.0,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
            clouds: Vec::new(),
        };
        let mut pool = GlobalResourcePool::new();
        pool.try_add(ResourceKind::Food, ResourceKind::Food.max())
            .unwrap();

        let report = eco.tick(&mut pool);

        assert_eq!(eco.total_fruit(), 1);
        assert_eq!(eco.fruit_eaten, 0);
        assert_eq!(report.food_produced, 0);
        assert_eq!(pool.get(ResourceKind::Food), ResourceKind::Food.max());
    }

    #[test]
    fn tick_preserves_world_caps() {
        let mut eco = EcoCycle::default();
        eco.rabbits.push(EcoRabbit {
            id: 99,
            pos: Vec2::ZERO,
            energy: 1.0,
        });
        eco.berries.push(EcoBerryBush {
            id: 99,
            pos: Vec2::ZERO,
            fruit: 1,
        });
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
        source.rain = 1.25;
        source.rainfall = 8.5;
        source.fruit_eaten = 7;
        source.fruit_grown = 4;
        source.plants_grown = 3;
        source.rabbits_born = 2;
        source.wildlife_born = 1;

        let snapshot = source.to_snapshot(99);
        let mut applied = EcoCycle {
            rabbits: Vec::new(),
            berries: Vec::new(),
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: 0.0,
            rain: 0.0,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
            clouds: Vec::new(),
        };

        applied.apply_snapshot(&snapshot);

        assert_eq!(snapshot.tick, 99);
        assert_eq!(applied.rabbit_count(), source.rabbit_count());
        assert_eq!(applied.wildlife_count(), source.wildlife_count());
        assert_eq!(applied.berry_count(), source.berry_count());
        assert_eq!(applied.plant_count(), source.plant_count());
        assert_eq!(applied.clouds, source.clouds);
        assert_eq!(
            applied.visible_ecology_kinds(),
            source.visible_ecology_kinds()
        );
        assert_eq!(applied.co2, source.co2);
        assert_eq!(applied.rain, source.rain);
        assert_eq!(applied.rainfall, source.rainfall);
        assert_eq!(applied.fruit_eaten, source.fruit_eaten);
        assert_eq!(applied.fruit_grown, source.fruit_grown);
        assert_eq!(applied.plants_grown, source.plants_grown);
        assert_eq!(applied.rabbits_born, source.rabbits_born);
        assert_eq!(applied.wildlife_born, source.wildlife_born);
    }
}
