use std::fmt::{self, Debug};

use bevy::prelude::*;
use bevy::time::FixedTimestep;
use bevy::utils::hashbrown::HashMap;
use crossbeam_channel::{unbounded, Receiver};

use super::assets::TextureAtlasCenter;

pub type SpriteConfigFn = fn(&AnimationValue) -> AnimationInfo;
// impl fmt::Debug for SpriteConfigFn {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "PolFn")
//     }
// }

#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Hash, Reflect)]
#[reflect(Component)]
pub enum AnimationValue {
    Idle,
    Walk,
    Attack,
}
impl Default for AnimationValue {
    fn default() -> Self {
        AnimationValue::Idle
    }
}

#[derive(Debug, Default, Clone)]
pub struct AnimationInfo {
    pub startIndex: usize,
    pub endIndex: usize,
    pub spriteName: String,
}
#[derive(Component, Clone)]
pub struct AnimationMachine {
    pub value: AnimationValue,
    pub progress: f32,
    pub config: fn(&AnimationValue) -> AnimationInfo,
}
impl Debug for AnimationMachine{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimationMachine").field("value", &self.value).field("progress", &self.progress).finish()
    }
}

impl Default for AnimationMachine {
    fn default() -> Self {
        Self {
            value: AnimationValue::Idle,
            progress: 0.0,
            config: |_| AnimationInfo { ..Default::default() },
        }
    }
}
impl AnimationMachine {
    fn get(&self) -> AnimationInfo {
        (self.config)(&self.value)
    }
}

#[derive(Debug)]
pub struct ChangeAnimation {
    pub ins: Entity,
    pub newValue: AnimationValue,
    pub xDir: f32,
}

pub fn init_animation(app: &mut App) {
    let (s, r) = unbounded::<ChangeAnimation>();
    app.add_system_set(
        SystemSet::new().with_run_criteria(FixedTimestep::step(0.1)).with_system(sprite_update),
    );
    app.insert_resource(s);
    app.insert_resource(r);
    app.add_system(state_trigger);
}

fn state_trigger(
    getAnimationEvt: ResMut<Receiver<ChangeAnimation>>,
    textureAtlasCenter: Res<TextureAtlasCenter>,
    mut query: Query<(
        &mut AnimationMachine,
        &mut TextureAtlasSprite,
        &mut Handle<TextureAtlas>,
        &mut Transform,
    )>,
) {
    loop {
        if getAnimationEvt.is_empty() {
            break;
        }
        let ev = getAnimationEvt.recv().unwrap();
        if let Ok((mut animationMachine, mut sprite, mut sprite_handle, mut transform)) =
            query.get_mut(ev.ins)
        {
            if animationMachine.value != ev.newValue {
                animationMachine.value = ev.newValue;
                sprite.index = 0;
                let AnimationInfo { spriteName, startIndex, endIndex } = animationMachine.get();
                sprite.index = startIndex;

                // 为什么需要替换呢 他们不是相等吗？
                let newTextureAtlasHandle = textureAtlasCenter.0.get(&spriteName).unwrap().clone();
                *sprite_handle = newTextureAtlasHandle;
            }

            if ev.xDir > 0.0 {
                sprite.flip_x = false;
                transform.translation.x = transform.translation.x.abs();
            }
            if ev.xDir < 0.0 {
                sprite.flip_x = true;
                transform.translation.x = -transform.translation.x.abs();
            }
        }
    }
}

fn sprite_update(mut query: Query<(&mut AnimationMachine, &mut TextureAtlasSprite)>) {
    for (mut animationMachine, mut sprite) in query.iter_mut() {
        let AnimationInfo { startIndex, endIndex, spriteName } = animationMachine.get();
        // println!("here {:?} {:?} {:?}", startIndex, endIndex,animationMachine);
        if sprite.index >= endIndex {
            sprite.index = startIndex;
        } else {
            sprite.index += 1;
        }
        // animationMachine.progress =
        //     (sprite.index - startIndex) as f32 / (endIndex - startIndex) as f32;
    }
}
