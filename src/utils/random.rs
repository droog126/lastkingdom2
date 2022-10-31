use std::ops::Add;

use bevy::math::Vec2;
use rand::{distributions::uniform::SampleUniform, thread_rng, Rng};

use crate::constant::FPS;

pub fn random_arr2(x: u32, y: u32) -> impl Iterator<Item = [f32; 2]> + Clone {
    std::iter::repeat_with(move || {
        let mut rng = thread_rng();
        let randx = rng.gen::<f32>() * x as f32 - x as f32 / 2.0;
        let randy = rng.gen::<f32>() * y as f32 - y as f32 / 2.0;
        [randx, randy]
    })
}

pub fn random_arr4(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> impl Iterator<Item = [i32; 4]> + Clone {
    std::iter::repeat_with(move || {
        let mut rng = thread_rng();
        let randx = rng.gen_range(-x / 2..x / 2);
        let randy = rng.gen_range(-y / 2..y / 2);
        let randWidth = rng.gen_range(0..width);
        let randHeight = rng.gen_range(0..height);
        [randx, randy, randWidth, randHeight]
    })
}

// num=1 表示10秒有一次机会
pub fn random_in_unlimited(num: f32) -> bool {
    let mut rng = thread_rng();
    rng.gen_range(0.0..*FPS) < num / 10.0
}

pub fn random_range(min: f32, max: f32) -> f32 {
    let mut rng = thread_rng();
    rng.gen_range(min * FPS..max * FPS)
}

pub fn random_Vec2() -> Vec2 {
    let mut rng = thread_rng();
    let mut randx = rng.gen_range(-1.0..1.0);
    let mut randy = rng.gen_range(-1.0..1.0);
    Vec2::new(randx, randy).normalize()
}

// pub fn distance_2d(a: Vec2, b: Vec2) -> f32 {
//     let c = a - b;
//     c.distance(rhs)
// }
