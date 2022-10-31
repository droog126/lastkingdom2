use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::{
    system::{
        animation::{AnimationInfo, AnimationMachine, AnimationValue, ChangeAnimation},
        assets::{FontCenter, TextureAtlasCenter},
        collision::{CollisionInput, CollisionInputUnit},
        inViewPort::CurEntity,
        instanceInput::{self, CurInput, InstanceInput},
        timeLine::TimeLine,
    },
    utils::random::{random_Vec2, random_in_unlimited, random_range},
};
use bevy::{ecs::entity, prelude::*};

//  不定的类型返回，如何比较优雅？
pub fn PlayerAnimationConfig(value: &AnimationValue) -> AnimationInfo {
    match value {
        AnimationValue::Idle => {
            AnimationInfo { startIndex: 0, endIndex: 0, spriteName: "player".to_string() }
        }
        AnimationValue::Walk => {
            AnimationInfo { startIndex: 8, endIndex: 15, spriteName: "player".to_string() }
        }
        _ => AnimationInfo { startIndex: 0, endIndex: 0, spriteName: "player".to_string() },
    }
}

#[derive(Component)]
pub struct Player {}

#[derive(Component)]
pub struct PlayerData {
    childrenTable: PlayerChildrenTable,
    ai: PlayerAiState,
}
pub struct PlayerChildrenTable {
    animationNode: Option<Entity>,
}

pub enum PlayerAiState {
    HangOut { timeLine: i64, dir: Vec2 },
    None,
}

pub fn on_player_add(
    mut commands: Commands,
    query: Query<Entity, Added<Player>>,
    textureCenter: Res<TextureAtlasCenter>,
    collisionInput: ResMut<CollisionInput>,
    mut curEntity: ResMut<CurEntity>,
    timeLine: Res<TimeLine>,
    fontCenter: Res<FontCenter>,
) {
    let mut timeLine = timeLine.0;
    let mut curEntity = curEntity.into_inner();
    let fontAssets = fontCenter.0.get("default");
    for entity in &query {
        if curEntity.0 == None {
            curEntity.0 = Some(entity.clone());
        }

        let mut childrenTable = PlayerChildrenTable { animationNode: None };

        commands
            .entity(entity)
            .insert(Player {})
            .insert(InstanceInput { ..Default::default() })
            .with_children(|parent| {
                let mut id = parent
                    .spawn_bundle(SpriteSheetBundle {
                        transform: Transform { scale: Vec3::new(1.0, 1.0, 0.0), ..default() },
                        texture_atlas: textureCenter.0.get("player").unwrap().clone(),
                        ..Default::default()
                    })
                    .insert(AnimationMachine {
                        value: AnimationValue::Idle,
                        progress: 0.0,
                        config: PlayerAnimationConfig,
                    })
                    .id();
                childrenTable.animationNode = Some(id);
            })
            .insert(PlayerData { childrenTable, ai: PlayerAiState::None });

        #[cfg(debug_assertions)]
        {
            if let Some(font) = fontAssets {
                commands.entity(entity).with_children(|parent| {
                    parent.spawn_bundle(Text2dBundle {
                        text: Text {
                            sections: vec![TextSection {
                                value: format!("{:#?}", entity),
                                style: TextStyle {
                                    font: font.clone(),
                                    font_size: 8.0,
                                    color: Color::WHITE,
                                },
                            }],
                            alignment: TextAlignment::CENTER,
                        },
                        transform: Transform::from_xyz(0.0, 10.0, 10.0),
                        ..Default::default()
                    });
                });
            }
        }
    }
}

pub fn on_player_step(
    mut query: Query<(&mut InstanceInput, &mut PlayerData, &mut Transform, Entity)>,
    curEntity: Res<CurEntity>,
    curInput: Res<CurInput>,
    changeAnimation: Res<Sender<ChangeAnimation>>,
    timeLine: Res<TimeLine>,
    time: Res<Time>,
) {
    let globalTimeLine = timeLine.0;
    let curInput = &curInput.0;
    for (instanceInput, mut playerData, mut transform, entity) in &mut query {
        let instanceInput = instanceInput.into_inner();
        if let Some(curEntity) = curEntity.0 {
            if curEntity == entity {
                instanceInput.setNew(curInput.clone());
            } else {
                let mut ai = &mut playerData.ai;

                match ai {
                    PlayerAiState::HangOut { timeLine, dir } => {
                        instanceInput.dir = Some(dir.clone());
                        if globalTimeLine > *timeLine {
                            *ai = PlayerAiState::None;
                        }
                    }
                    PlayerAiState::None => {
                        if random_in_unlimited(1.0) {
                            let timeLine = globalTimeLine + random_range(0.5, 2.0) as i64;
                            let dir = random_Vec2();
                            *ai = PlayerAiState::HangOut { timeLine, dir };
                        }
                    }
                }
                // ai logic
            }
        }

        if instanceInput.dir != None {
            let dir = instanceInput.dir.unwrap();

            transform.translation.x += dir.x;
            transform.translation.y += dir.y;
        }

        if let Some(dir) = &instanceInput.dir {
            changeAnimation.send(ChangeAnimation {
                ins: playerData.childrenTable.animationNode.unwrap(),
                newValue: AnimationValue::Walk,
                xDir: dir.x,
            });
        } else {
            changeAnimation.send(ChangeAnimation {
                ins: playerData.childrenTable.animationNode.unwrap(),
                newValue: AnimationValue::Idle,
                xDir: 0.0,
            });
        }
    }
}
// 要训练记忆
