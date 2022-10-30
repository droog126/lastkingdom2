use std::{time::Instant, f32::consts::E};

use bevy::prelude::*;
use bevy_inspector_egui::{egui::Key, bevy_egui::EguiContext,egui};
use crossbeam_channel::Sender;

use crate::utils::num::bool_to_f32;

pub enum NormalInputClickType {
    SomeTime(f32),
    Temp,
}
// 按住 一次  频率

#[derive(Component, Clone,Debug)]
pub struct InstanceInput {
    pub dir: Option<Vec2>,
    pub attack: (bool, bool),
    pub action: (bool, bool),
    pub jump: (bool, bool),
    pub defense: (bool, bool),
    pub slide: bool,
    pub squat: bool,
    pub sprint: bool,
    pub dance: bool,
    pub laugh: bool,
}


impl Default for InstanceInput {
    fn default() -> Self {
        Self {
            dir: Default::default(),
            attack: Default::default(),
            action: Default::default(),
            jump: Default::default(),
            defense: Default::default(),
            slide: Default::default(),
            squat: Default::default(),
            sprint: Default::default(),
            dance: Default::default(),
            laugh: Default::default(),
        }
    }
}
impl InstanceInput {
    pub fn setNew(&mut self, newValue: InstanceInput) {
        self.dir = newValue.dir;
        self.attack = newValue.attack;
        self.action = newValue.action;
        self.jump = newValue.jump;
        self.defense = newValue.defense;
        self.slide = newValue.slide;
        self.squat = newValue.squat;
        self.sprint = newValue.sprint;
        self.dance = newValue.dance;
        self.laugh = newValue.laugh;
    }
}

// p0 找到那个可以解构的办法
// 自己和自己打
#[derive(Debug)]
pub struct CurInput(pub InstanceInput);
impl Default for CurInput {
    fn default() -> Self {
        Self(InstanceInput::default())
    }
}

#[derive(Default,Debug)]
pub struct CurEntity(pub Option<Entity>);


#[derive(Default,Debug)]
pub struct CurHoverEntity(pub Option<Entity>);

pub fn init_instanceInput(app: &mut App) {
    app.insert_resource(CurEntity::default())
        .insert_resource(CurHoverEntity::default())
        .insert_resource(CurInput::default());
    app.add_system(index);
    #[cfg(debug_assertions)]
    {
        app.add_system(debug);
    }
}

pub fn debug(mut egui_context:ResMut<EguiContext>,curEntity:Res<CurEntity>,curHoverEntity:Res<CurHoverEntity>,curInput:Res<CurInput>){
    egui::Window::new("inputSystem").show(egui_context.ctx_mut(),|ui|{
        ui.horizontal(|ui|{
            ui.label("input");
            ui.set_width(200.0);
            ui.wrap_text();
            ui.set_height(400.0);
            ui.label(format!("{:#?}",curInput.0));
        });
        ui.horizontal(|ui|{
            ui.label("curEntity:");
            ui.label(format!("{:?}",curEntity.0));
        });
        ui.horizontal(|ui|{
            // ui.label("curHoverEntity");
            ui.text_edit_singleline(&mut format!("{:?}",curHoverEntity.0));
        });
    });

}

pub fn index(
    // mut query: Query<&mut InstanceInput>,
    curEntity: Res<CurEntity>,
    mut curInput: ResMut<CurInput>,
    keyboardInput: Res<Input<KeyCode>>,
    mouseInput: Res<Input<MouseButton>>,
) {
    let mut curInput = &mut curInput.0;
    if keyboardInput.pressed(KeyCode::D)
        || keyboardInput.pressed(KeyCode::A)
        || keyboardInput.pressed(KeyCode::W)
        || keyboardInput.pressed(KeyCode::S)
    {
        let mut x = bool_to_f32(keyboardInput.pressed(KeyCode::D))
            - bool_to_f32(keyboardInput.pressed(KeyCode::A));
        let mut y = bool_to_f32(keyboardInput.pressed(KeyCode::W))
            - bool_to_f32(keyboardInput.pressed(KeyCode::S));
        if x==0.0 && y==0.0{
            curInput.dir = None;
        }else{
            curInput.dir = Some(Vec2::new(x, y).normalize())
        }
    } else {
        curInput.dir = None;
    }

    if keyboardInput.just_pressed(KeyCode::Space) {
        curInput.jump.0 = true;
    } else {
        curInput.jump.0 = false;
    }
    if keyboardInput.pressed(KeyCode::Space) {
        curInput.jump.1 = true;
    } else {
        curInput.jump.1 = false;
    }

    if keyboardInput.just_pressed(KeyCode::E) {
        curInput.action.0 = true;
    } else {
        curInput.action.0 = false;
    }
    if keyboardInput.pressed(KeyCode::E) {
        curInput.action.1 = true;
    } else {
        curInput.action.1 = false;
    }

    if mouseInput.just_pressed(MouseButton::Left) {
        curInput.attack.0 = true;
    } else {
        curInput.attack.0 = false;
    }
    if mouseInput.pressed(MouseButton::Left) {
        curInput.attack.1 = true;
    } else {
        curInput.attack.1 = false;
    }

    if mouseInput.just_pressed(MouseButton::Right) {
        curInput.defense.0 = true;
    } else {
        curInput.defense.0 = false;
    }
    if mouseInput.pressed(MouseButton::Right) {
        curInput.defense.1 = true;
    } else {
        curInput.defense.1 = false;
    }

    // if let Some(entity) = curEntity.0 {
    //     if let Ok(mut instanceInput) = query.get_mut(entity) {
    //         instanceInput.setNew(curInput.clone());
    //     }
    // }
    // query.get_mut(*curEntity);
}

// 目标是做所有游戏的基础
