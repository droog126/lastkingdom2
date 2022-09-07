use bevy::{prelude::*, window::PresentMode};
pub mod index;

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
