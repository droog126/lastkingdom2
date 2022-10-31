use bevy::{prelude::*, time::FixedTimestep};
use bevy_inspector_egui::{bevy_egui::EguiContext, egui};

use crate::constant::{FPS, VIEW_PORT_RADIUS};

use super::{camera::CursorPosition, instanceInput::InstanceInput};

pub struct InViewPortInstanceList(pub Vec<Entity>);

#[derive(Default, Debug)]
pub struct CurEntity(pub Option<Entity>);

#[derive(Default, Debug)]
pub struct CurHoverEntity(pub Option<Entity>);

pub fn init_inViewPortInstanceList_system(app: &mut App) {
    app.insert_resource(CurEntity::default())
        .insert_resource(CurHoverEntity::default())
        .insert_resource(InViewPortInstanceList(Vec::new()));
    app.add_system_set(
        SystemSet::new()
            .with_run_criteria(FixedTimestep::step((2.0 / FPS).into()))
            .with_system(step),
    );
    #[cfg(debug_assertions)]
    {
        app.add_system(debug);
    }
}

// 像这种 .0 的一般都要带上 mut
pub fn step(
    mut query: Query<(&Transform, Entity), With<InstanceInput>>,
    curPos: Res<CursorPosition>,
    mut curHoverEntity: ResMut<CurHoverEntity>,
    mut curEntity: ResMut<CurEntity>,
    mut list: ResMut<InViewPortInstanceList>,
    input: Res<Input<MouseButton>>,
) {
    let mut list = &mut list.0;
    list.clear();
    let a = Vec2::new(curPos.x, curPos.y);
    curHoverEntity.0 = None;

    for (transform, entity) in &query {
        let b = transform.translation.truncate();
        let distance = a.distance(b);
        if distance <= *VIEW_PORT_RADIUS {
            list.push(entity);
        }

        #[cfg(debug_assertions)]
        {
            if distance <= 10.0 {
                if input.pressed(MouseButton::Left) {
                    curEntity.0 = Some(entity);
                }
            }
        }
        if distance <= 10.0 {
            if curHoverEntity.0 == None {
                curHoverEntity.0 = Some(entity);
            }
        }
    }
}

pub fn debug(
    mut egui_context: ResMut<EguiContext>,
    curEntity: Res<CurEntity>,
    curHoverEntity: Res<CurHoverEntity>,
    curEntityList: Res<InViewPortInstanceList>,
) {
    let curEntityList = &curEntityList.0;
    egui::Window::new("viewEntity").show(egui_context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("curEntity:{:?}", curEntity.0));
        });
        ui.horizontal(|ui| {
            ui.label(format!("hoverEntity:{:?}", curHoverEntity.0));
        });
        ui.horizontal(|ui| {
            ui.label(format!("viewEntityLength:{:?}", curEntityList.len()));
        });
    });
}
