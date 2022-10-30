use bevy::{prelude::*, time::FixedTimestep};
use bevy_inspector_egui::{bevy_egui::EguiContext, egui};
use crossbeam_channel::Sender;

#[derive(Debug)]
pub struct TimeLine(pub i64);

pub fn init_timeLine_system(app: &mut App) {
    app.insert_resource(TimeLine(0));
    app.add_system_set(
        SystemSet::new().with_run_criteria(FixedTimestep::step(1.0 / 120.0)).with_system(index),
    );
    app.add_system(index);

    // #[cfg(debug_assertions)]
    {
        app.add_system(debug_system);
    }
}

pub fn index(mut timeLine: ResMut<TimeLine>) {
    timeLine.0 += 1;
}

// #[cfg(debug_assertions)]
pub fn debug_system(mut context: ResMut<EguiContext>, timeLine: Res<TimeLine>, time: Res<Time>) {
    let mut timeLine = timeLine.0;
    egui::Window::new("timeLine").show(context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("timeLine:{:#?}", timeLine));
        });
        ui.horizontal(|ui| {
            ui.label(format!("fps:{:#?}", 1.0 / time.delta_seconds()));
        });
    });
}
