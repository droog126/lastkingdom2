use bevy::prelude::*;

#[derive(Resource)]
pub struct SimClock {
    pub frame_tick: u64,
    pub tick: u64,
    pub last_sim_step_ran: bool,
    pub last_tick_wall: f32,
    pub slow_tick_accum: f32,
    pub last_hud_wall: f32,
    pub last_screenshot_wall: f32,
    pub screenshot_count: u32,
}

impl Default for SimClock {
    fn default() -> Self {
        let mut max_iter: u32 = 0;
        if let Ok(entries) = std::fs::read_dir("screenshots") {
            for e in entries.flatten() {
                if let Some(name) = e.file_name().to_str() {
                    if let Some(rest) = name.strip_prefix("iter_") {
                        if let Ok(n) = rest.parse::<u32>() {
                            if n > max_iter {
                                max_iter = n;
                            }
                        }
                    }
                }
            }
        }
        Self {
            frame_tick: 0,
            tick: 0,
            last_sim_step_ran: false,
            last_tick_wall: 0.0,
            slow_tick_accum: 0.0,
            last_hud_wall: 0.0,
            last_screenshot_wall: 0.0,
            screenshot_count: max_iter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_clock_default_initializes_zero() {
        let c = SimClock::default();
        assert_eq!(c.frame_tick, 0);
        assert_eq!(c.tick, 0);
        assert_eq!(c.last_sim_step_ran, false);
        assert_eq!(c.last_tick_wall, 0.0);
        assert_eq!(c.slow_tick_accum, 0.0);
        assert_eq!(c.last_hud_wall, 0.0);
        assert_eq!(c.last_screenshot_wall, 0.0);
    }

    #[test]
    fn sim_clock_tick_counter() {
        let mut c = SimClock::default();
        c.tick = 100;
        c.frame_tick = 1000;

        assert_eq!(c.tick, 100);
        assert_eq!(c.frame_tick, 1000);
    }

    #[test]
    fn sim_clock_accumulator() {
        let mut c = SimClock::default();
        c.slow_tick_accum = 0.5;
        c.slow_tick_accum += 0.3;

        assert!((c.slow_tick_accum - 0.8).abs() < 0.001);
    }

    #[test]
    fn sim_clock_wall_time() {
        let mut c = SimClock::default();
        c.last_tick_wall = 10.0;
        c.last_hud_wall = 5.0;
        c.last_screenshot_wall = 2.0;

        assert_eq!(c.last_tick_wall, 10.0);
        assert_eq!(c.last_hud_wall, 5.0);
        assert_eq!(c.last_screenshot_wall, 2.0);
    }

    #[test]
    fn sim_clock_sim_step_flag() {
        let mut c = SimClock::default();
        assert_eq!(c.last_sim_step_ran, false);

        c.last_sim_step_ran = true;
        assert_eq!(c.last_sim_step_ran, true);

        c.last_sim_step_ran = false;
        assert_eq!(c.last_sim_step_ran, false);
    }

    #[test]
    fn sim_clock_screenshot_count() {
        let c = SimClock::default();
        assert_eq!(c.screenshot_count, 0);
    }
}
