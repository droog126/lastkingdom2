use bevy::{prelude::*, utils::hashbrown::HashMap};
use broccoli::{aabb::ManySwappable, axgeom::Rect, rect};
use sepax2d::{
    prelude::{Capsule, Circle, Polygon, AABB},
    sat_overlap, Shape,
};

use crate::instance::InstanceUnitType;

use super::props::InstanceProps;

// 三种形状  圆形 矩形 多边形(凹凸多边形)
#[derive(Clone)]
pub enum CollisionShapeType {
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    Circle {
        x: f32,
        y: f32,
        radius: f32,
    },
    Capsule {
        x: f32,
        y: f32,
        radius: f32,
        arm: Vec2,
    },
    Polygon(CollisionPolygon),
}
impl CollisionShapeType {
    fn transform(&mut self) -> Box<dyn Shape> {
        let target: Box<dyn Shape> = match self {
            CollisionShapeType::Rect { x, y, width, height } => {
                Box::new(AABB::new((*x, *y), *width, *height))
            }
            CollisionShapeType::Circle { x, y, radius } => Box::new(Circle::new((*x, *y), *radius)),
            CollisionShapeType::Capsule { x, y, radius, arm } => {
                Box::new(Capsule::new((*x, *y), (arm.x, arm.y), *radius))
            }
            CollisionShapeType::Polygon(target) => Box::new(Polygon::from_vertices(
                (target.x, target.y),
                target.vertices.clone(),
            )),
        };
        target
    }
}
#[derive(Clone)]
pub struct CollisionAABB {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
impl CollisionAABB {
    fn toRect(&self) -> Rect<f32> {
        rect(self.x, self.x + self.width, self.y, self.y + self.height)
    }
}
#[derive(Clone)]
pub struct CollisionPolygon {
    x: f32,
    y: f32,
    vertices: Vec<(f32, f32)>,
}

#[derive(Clone)]
pub struct CollisionShape {
    rect: CollisionAABB,
    ext: CollisionShapeType,
}
impl CollisionShape {
    fn new_rect(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            rect: CollisionAABB { x, y, width, height },
            ext: CollisionShapeType::Rect { x, y, width, height },
        }
    }
}

pub struct CollisionOut {
    unitType: InstanceUnitType,
    entity: Entity,
    pos: Vec3,
    shape: Option<CollisionShape>,
    props: Option<InstanceProps>,
    serde: Option<String>,
}
#[derive(Clone, Copy)]
pub struct CollisionNeedConfig {
    shape: bool,
    props: bool,
    serde: bool,
}
impl Default for CollisionNeedConfig {
    fn default() -> Self {
        Self { shape: false, props: false, serde: false }
    }
}

pub type CollisionFilterConfig = fn(insType: InstanceUnitType) -> bool;

pub struct CollisionInputUnit {
    shape: CollisionShape,
    need: CollisionNeedConfig,
    filterFunc: CollisionFilterConfig,
    insType: InstanceUnitType,
}

pub struct CollisionInput(HashMap<(Entity, String), CollisionInputUnit>);
impl CollisionInput {
    pub fn new() -> Self {
        let mut hashmap: HashMap<(Entity, String), CollisionInputUnit> = HashMap::new();
        Self(hashmap)
    }
    pub fn add(&mut self, entity: Entity, name: String, inputUnit: CollisionInputUnit) {
        self.0.insert((entity, name), inputUnit);
    }
}

impl CollisionShape {
    pub fn update(&mut self, newX: f32, newY: f32) {
        self.rect.x = newX;
        self.rect.y = newY;
        match &mut self.ext {
            CollisionShapeType::Circle { x, y, radius } => {
                *x = newX;
                *y = newY;
            }
            CollisionShapeType::Capsule { x, y, radius, arm } => {
                *x = newX;
                *y = newY;
            }
            CollisionShapeType::Polygon(polygon) => {
                polygon.x = newX;
                polygon.y = newY;
            }
            CollisionShapeType::Rect { x, y, width, height } => {
                *x = newX;
                *y = newY;
            }
        };
    }
}

pub struct CollisionInter {
    entity: Entity,
    name: String,
    filterFunc: CollisionFilterConfig,
    instanceUnitType: InstanceUnitType,
    ext: CollisionShapeType,
    need: CollisionNeedConfig,
}

pub struct CollisionTransferHashMap(
    HashMap<(Entity, String), Vec<(Entity, String, CollisionNeedConfig)>>,
);
impl CollisionTransferHashMap {
    fn add(&mut self, aEntity: Entity, aName: String, bEntity: Entity, bName: String) {}
    fn clear() {}
}

pub fn init_ins_collision_dependence(app: &mut App) {
    app.add_system_to_stage(CoreStage::PostUpdate, collision_handle.exclusive_system());
    app.insert_resource(CollisionInput::new());
}

// 碰撞结果加工厂
pub fn collision_handle(world: &mut World) {
    let mut aabbs: Vec<ManySwappable<(Rect<f32>, CollisionInter)>> = Vec::new();
    let collisionInput = world.get_resource_mut::<CollisionInput>().unwrap();
    // 获取是否碰撞，再去查找
    let mut aabbs = Vec::new();
    for ((entity, name), collisionUnit) in collisionInput.0.iter() {
        let mut rect = collisionUnit.shape.rect.toRect();
        let mut inner = CollisionInter {
            entity: entity.clone(),
            name: name.clone(),
            filterFunc: collisionUnit.filterFunc,
            instanceUnitType: collisionUnit.insType,
            ext: collisionUnit.shape.ext.clone(),
            need: collisionUnit.need.clone(),
        };
        aabbs.push(ManySwappable((rect, inner)));
    }

    let mut tree = broccoli::Tree::new(&mut aabbs);
    tree.find_colliding_pairs(|a, b| {
        let mut newA = &mut *a.unpack_inner();
        let mut newB = &mut *b.unpack_inner();
        // 处理A
        let mut aShape = newA.ext.transform();
        let mut bShape = newB.ext.transform();
        if sat_overlap(&*aShape, &*bShape) {
            if (newA.filterFunc)(newB.instanceUnitType) {}

            if (newB.filterFunc)(newA.instanceUnitType) {}
        }
    })
}

// 碰撞查询实际加工厂
