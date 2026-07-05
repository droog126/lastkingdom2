use lk2_core::world::BlockType;
use lk2_core::world::World as GameWorld;

#[derive(Debug, Clone)]
pub struct ScalarField {
    pub data: Vec<f32>,
    pub shape: [usize; 3],
}

impl ScalarField {
    #[inline]
    pub fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        (y * self.shape[2] + z) * self.shape[0] + x
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> f32 {
        self.data[self.idx(x, y, z)]
    }
}

pub fn build_density_field(world: &GameWorld, min: [i32; 3], max: [i32; 3]) -> ScalarField {
    let cell_size = [
        (max[0] - min[0]).max(1) as usize,
        (max[1] - min[1]).max(1) as usize,
        (max[2] - min[2]).max(1) as usize,
    ];
    let corner_shape = [cell_size[0] + 1, cell_size[1] + 1, cell_size[2] + 1];
    let n = corner_shape[0] * corner_shape[1] * corner_shape[2];
    let mut data = vec![0.0_f32; n];

    for cz in 0..corner_shape[2] {
        for cy in 0..corner_shape[1] {
            for cx in 0..corner_shape[0] {
                let mut solid = 0u32;
                let mut total = 0u32;
                for dz in -1i32..=0 {
                    for dy in -1i32..=0 {
                        for dx in -1i32..=0 {
                            let wx = min[0] + cx as i32 + dx;
                            let wy = min[1] + cy as i32 + dy;
                            let wz = min[2] + cz as i32 + dz;
                            if !world.in_bounds(wx, wy, wz) {
                                continue;
                            }
                            total += 1;

                            let b = world.get(wx, wy, wz);
                            if b.is_solid() && !matches!(b, BlockType::Water) {
                                solid += 1;
                            }
                        }
                    }
                }
                let density = if total == 0 {
                    0.0
                } else {
                    solid as f32 / total as f32
                };
                let i = (cy * corner_shape[2] + cz) * corner_shape[0] + cx;
                data[i] = density;
            }
        }
    }

    ScalarField { data, shape: corner_shape }
}

pub fn effective_ground_height(world: &GameWorld, x: i32, z: i32) -> f32 {
    match find_highest_standable_top_y(world, x, z) {
        Some(h) => h,
        None => world.pipeline.surface_f32(x, z).unwrap_or(world.size as f32),
    }
}

fn find_highest_standable_top_y(world: &GameWorld, x: i32, z: i32) -> Option<f32> {
    for y in (1..(world.size - 2)).rev() {
        if world.get(x, y, z).is_solid()
            && world.get(x, y - 1, z).is_solid()
            && !world.get(x, y + 1, z).is_solid()
            && !world.get(x, y + 2, z).is_solid()
        {
            return Some(y as f32 + 1.0);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use lk2_core::world::install_huge_spawn_platform;
    use lk2_core::world::terrain::presets;

    #[test]
    fn density_field_41x41x41() {
        let world = GameWorld::new(128);
        let field = build_density_field(&world, [0, 0, 0], [40, 40, 40]);
        assert_eq!(field.shape, [41, 41, 41]);

        let max = field.data.iter().cloned().fold(0.0_f32, f32::max);
        assert!(
            max < 0.5,
            "鍏?air 鏃惰鐐规渶澶?density 搴?< 0.5, got {}",
            max
        );
    }

    #[test]
    fn effective_ground_height_superflat_at_spawn_is_standable_top() {
        let pipeline = presets::superflat_preset();
        let world = GameWorld::with_pipeline(96, pipeline);
        let h = effective_ground_height(&world, 48, 48);
        assert!(
            (h - 13.0).abs() < 0.01,
            "effective_ground_height should return the top of the standable floor, got {}",
            h
        );
    }

    #[test]
    fn effective_ground_height_ignores_overhead_tree_canopy() {
        let mut world = GameWorld::new(32);
        world.set(8, 3, 8, BlockType::Stone);
        world.set(8, 4, 8, BlockType::Dirt);
        world.set(8, 12, 8, BlockType::Leaves);

        let h = effective_ground_height(&world, 8, 8);

        assert!(
            (h - 5.0).abs() < 0.01,
            "ground height should use the standable floor, not overhead canopy: got {}",
            h
        );
    }

    #[test]
    fn effective_ground_height_prefers_spawn_platform_over_pipeline_surface() {
        let pipeline = presets::by_name("default");
        let mut world = GameWorld::with_pipeline(96, pipeline);
        install_huge_spawn_platform(&mut world);

        let h = effective_ground_height(&world, 48, 48);

        assert!(
            (h - 15.0).abs() < 0.01,
            "decor and props should sit on the spawn platform top, got {}",
            h
        );
    }
}
