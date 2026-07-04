#![allow(dead_code)]

pub const SLOW_TICK_SECS: f32 = 1.0;

pub const FAST_TICK_SECS: f32 = 0.2;

pub const VERTICAL_SIZE: i32 = 96;

pub const SEA_LEVEL: i32 = 12;

pub const WATER_Y: f32 = SEA_LEVEL as f32;

pub const SUPERFLAT_GROUND_Y: f32 = SEA_LEVEL as f32 + 1.0;

pub const WORLD_SIZE: i32 = VERTICAL_SIZE;

pub const WORLD_CENTER: [i32; 2] = [WORLD_SIZE / 2, WORLD_SIZE / 2];

pub const MAX_NATIONAL_FLAGS: u32 = 8;

pub const FLAG_COSTS_SOULS: [u64; 8] = [10, 15, 20, 25, 30, 40, 50, 60];

pub const INITIAL_POP_CAP: u32 = 5;

pub const POP_UPGRADE_10_COST: (u64, u64, u64) = (500, 200, 0);

pub const POP_UPGRADE_15_COST: (u64, u64, u64) = (1_000, 500, 10);

pub const POP_UPGRADE_20_COST: (u64, u64, u64) = (2_000, 1_000, 25);

pub const FLAG_HP: u32 = 100;

pub const PLAYER_VISION_RADIUS: i32 = 24;

pub const FOG_FALLOFF_PER_BLOCK: f32 = 0.04;

pub const MAX_MONSTER_KINGDOMS: u32 = 5;

pub const MAX_MONSTER_NESTS: u32 = 80;

pub const MAX_MONSTER_INDIVIDUALS: u32 = 1_500;

pub const NEST_INITIAL_INDIVIDUALS: (u32, u32) = (15, 25);

pub const KINGDOM_MAINTAIN_INDIVIDUALS: (u32, u32) = (80, 120);

pub const NEST_DORMANCY_SECS: u32 = 5 * 60;

pub const STRICT_CONSERVATION_CHECK: bool = true;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_below_ground_invariant() {
        assert!(
            WATER_Y < SUPERFLAT_GROUND_Y,
            "WATER_Y ({}) 必须 < SUPERFLAT_GROUND_Y ({}), 否则玩家'站在水里'",
            WATER_Y,
            SUPERFLAT_GROUND_Y
        );

        assert!(
            SUPERFLAT_GROUND_Y - WATER_Y >= 1.0,
            "WATER_Y ({}) 到 SUPERFLAT_GROUND_Y ({}) 至少差 1m, got {}",
            WATER_Y,
            SUPERFLAT_GROUND_Y,
            SUPERFLAT_GROUND_Y - WATER_Y
        );
    }

    #[test]
    fn water_y_equals_sea_level() {
        assert_eq!(WATER_Y as i32, SEA_LEVEL);
    }
}
