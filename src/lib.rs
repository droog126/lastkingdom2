#![allow(
    dead_code,
    unused_variables,
    unused_mut,
    unused_imports,
    non_snake_case,
    unused_assignments,
    non_camel_case_types,
    unused_must_use
)]

use bevy::{prelude::*, window::PresentMode};
pub mod constant;
pub mod index;
pub mod instance;
pub mod system;
pub mod utils;

pub const LAUNCHER_TITLE: &str = "LastKingdom";

pub fn app() -> App {
    let mut app = App::new();
    app.insert_resource(WindowDescriptor {
        title: LAUNCHER_TITLE.to_string(),
        canvas: Some("#bevy".to_string()),
        fit_canvas_to_parent: true,
        present_mode: PresentMode::Immediate,
        resizable: true,
        ..Default::default()
    });
    index::entry(&mut app);
    app
}
