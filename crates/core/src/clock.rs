//! SimClock — 全局 sim 节拍 / 截图节奏资源
//!
//! 原位置: `src/main.rs::SimClock` (line 49-82)
//! 迁出原因: scenario 的 `scenario_runner` / `scenario_tick_recorder` 用它做
//! 节奏控制 + 录制。sim 系统在 core 里跑，main.rs 只是 wiring。
//!
//! 字段：
//! - `tick`                 — 当前 sim tick 编号
//! - `last_tick_wall`       — 上次 tick 的真实时间（防 1 帧多次 tick）
//! - `last_hud_wall`        — HUD 节流
//! - `last_screenshot_wall` — 截图节流
//! - `screenshot_count`     — 已截图数（自增命名 `iter_NN.png`）

use bevy::prelude::*;

#[derive(Resource)]
pub struct SimClock {
    pub tick: u64,
    pub last_tick_wall: f32,
    pub last_hud_wall: f32,
    pub last_screenshot_wall: f32,
    pub screenshot_count: u32,
}

impl Default for SimClock {
    fn default() -> Self {
        // 扫 screenshots/iter_NN/ 找最大编号, 让 screenshot_count 接着涨 (避免覆盖老 iter)
        // loop.ps1 期望每轮一个独立目录, 但 SimClock 是 in-process 重置, 所以这里手动接力
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
            tick: 0,
            last_tick_wall: 0.0,
            last_hud_wall: 0.0,
            last_screenshot_wall: 0.0,
            screenshot_count: max_iter,
        }
    }
}
