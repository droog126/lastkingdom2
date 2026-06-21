use bevy::prelude::*;

pub const RABBIT_CAP: usize = 5;
pub const BERRY_BUSH_CAP: usize = 10;

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

#[derive(Resource, Debug, Clone)]
pub struct EcoCycle {
    pub rabbits: Vec<EcoRabbit>,
    pub berries: Vec<EcoBerryBush>,
    pub co2: f32,
    pub fruit_eaten: u64,
    pub fruit_grown: u64,
}

impl Default for EcoCycle {
    fn default() -> Self {
        Self::demo_at(Vec2::new(
            crate::constant::WORLD_SIZE as f32 * 0.5,
            crate::constant::WORLD_SIZE as f32 * 0.5,
        ))
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

        Self { rabbits, berries, co2: 0.0, fruit_eaten: 0, fruit_grown: 0 }
    }

    pub fn tick(&mut self) {
        self.keep_caps();
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
            {
                self.berries[target_idx].fruit -= 1;
                rabbit.energy = (rabbit.energy + 2.0).min(12.0);
                self.fruit_eaten += 1;
            }
        }

        for berry in &mut self.berries {
            while berry.fruit < BERRY_MAX_FRUIT && self.co2 + f32::EPSILON >= BERRY_CO2_PER_FRUIT {
                berry.fruit += 1;
                self.co2 -= BERRY_CO2_PER_FRUIT;
                self.fruit_grown += 1;
            }
        }
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

    fn keep_caps(&mut self) {
        self.rabbits.truncate(RABBIT_CAP);
        self.berries.truncate(BERRY_BUSH_CAP);
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
    }

    #[test]
    fn rabbits_move_emit_co2_and_eat_fruit() {
        let mut eco = EcoCycle {
            rabbits: vec![EcoRabbit { id: 0, pos: Vec2::ZERO, energy: 6.0 }],
            berries: vec![EcoBerryBush { id: 0, pos: Vec2::new(0.5, 0.0), fruit: 1 }],
            co2: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
        };

        eco.tick();

        assert_eq!(eco.rabbit_count(), 1);
        assert_eq!(eco.berry_count(), 1);
        assert!(eco.fruit_eaten >= 1);
        assert!(eco.co2 > 0.0 || eco.fruit_grown > 0);
    }

    #[test]
    fn berries_consume_co2_to_regrow_fruit() {
        let mut eco = EcoCycle {
            rabbits: Vec::new(),
            berries: vec![EcoBerryBush { id: 0, pos: Vec2::ZERO, fruit: 0 }],
            co2: BERRY_CO2_PER_FRUIT,
            fruit_eaten: 0,
            fruit_grown: 0,
        };

        eco.tick();

        assert_eq!(eco.total_fruit(), 1);
        assert_eq!(eco.fruit_grown, 1);
        assert!(eco.co2.abs() <= f32::EPSILON);
    }

    #[test]
    fn tick_preserves_world_caps() {
        let mut eco = EcoCycle::default();
        eco.rabbits.push(EcoRabbit { id: 99, pos: Vec2::ZERO, energy: 1.0 });
        eco.berries.push(EcoBerryBush { id: 99, pos: Vec2::ZERO, fruit: 1 });

        eco.tick();

        assert_eq!(eco.rabbit_count(), RABBIT_CAP);
        assert_eq!(eco.berry_count(), BERRY_BUSH_CAP);
    }
}
