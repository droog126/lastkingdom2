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
