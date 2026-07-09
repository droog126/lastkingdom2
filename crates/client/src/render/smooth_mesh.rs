use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;

use super::marching_cubes::{McVertex, build_mesh as mc_build_mesh};
use super::scalar_field::build_density_field;
use lk2_core::world::{Biome, BlockType, World as GameWorld};

pub struct SmoothMesh {
    pub mesh: Mesh,
    pub collider_trimesh: Vec<[f32; 3]>,
    pub collider_indices: Vec<u32>,
}

pub fn build_smooth_mesh(
    world: &GameWorld,
    min: [i32; 3],
    max: [i32; 3],
    iso: f32,
    smooth_passes: u32,
) -> Option<SmoothMesh> {
    let field = build_density_field(world, min, max);
    if field.shape[0] < 2 || field.shape[1] < 2 || field.shape[2] < 2 {
        return None;
    }

    let corner_origin = [min[0] as f32, min[1] as f32, min[2] as f32];
    let cell_size = [1.0_f32, 1.0, 1.0];
    let (vertices, indices) = mc_build_mesh(&field, iso, corner_origin, cell_size);
    if vertices.is_empty() {
        return None;
    }

    let vertices = if smooth_passes > 0 {
        laplacian_smooth(vertices, &indices, smooth_passes)
    } else {
        vertices
    };

    let (positions, normals) = smooth_normals(&vertices, &indices);

    let colors: Vec<[f32; 4]> = positions.iter().map(|p| terrain_color(world, *p)).collect();

    let uvs: Vec<[f32; 2]> = positions.iter().map(|p| [p[0] * 0.10, p[2] * 0.10]).collect();

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices.clone()));

    Some(SmoothMesh { mesh, collider_trimesh: positions, collider_indices: indices })
}

fn terrain_color(world: &GameWorld, p: [f32; 3]) -> [f32; 4] {
    const GRASS_DARK: [f32; 3] = [0.28, 0.52, 0.18];
    const GRASS: [f32; 3] = [0.36, 0.66, 0.22];
    const GRASS_LIGHT: [f32; 3] = [0.50, 0.78, 0.30];
    const DIRT: [f32; 3] = [0.55, 0.38, 0.20];
    const DIRT_DARK: [f32; 3] = [0.40, 0.26, 0.14];
    const STONE: [f32; 3] = [0.62, 0.60, 0.55];
    const STONE_DARK: [f32; 3] = [0.45, 0.43, 0.40];
    const SAND: [f32; 3] = [0.80, 0.72, 0.50];
    const MOSS: [f32; 3] = [0.28, 0.52, 0.22];
    const SNOW: [f32; 3] = [0.88, 0.93, 0.96];
    const SNOW_BLUE: [f32; 3] = [0.68, 0.82, 0.90];
    const WATER_EDGE: [f32; 3] = [0.30, 0.55, 0.72];

    let block = surface_block_near(world, p);
    let biome = Biome::from_xz_infinite(p[0].floor() as i32, p[2].floor() as i32);
    let zone_n = (p[0] * 0.06 + p[2] * 0.08).sin() * 0.5 + 0.5;
    let macro_n = (p[0] * 0.015 + p[2] * 0.018).sin() * 0.5 + 0.5;
    let patch_n = (p[0] * 0.32 + p[2] * 0.28).sin() * 0.5 + 0.5;
    let fine_n = ((p[0] * 0.85).sin() * (p[2] * 0.73).cos()) * 0.5 + 0.5;
    let micro_n = (p[0] * 1.7 + p[2] * 1.3).sin() * 0.5 + 0.5;
    let height_t = ((p[1] - 2.0) / 35.0).clamp(0.0, 1.0);

    let (base_rgb, base_w, sec_rgb, sec_w) = match block {
        BlockType::Grass | BlockType::Leaves | BlockType::BerryThicket => {
            let grass_base = if height_t > 0.58 { GRASS_DARK } else { GRASS };
            let detail_rgb = if patch_n > 0.78 {
                DIRT_DARK
            } else if patch_n > 0.48 {
                GRASS_LIGHT
            } else {
                MOSS
            };
            (grass_base, 0.78, detail_rgb, 0.26)
        }
        BlockType::Dirt => {
            if biome == Biome::Jungle {
                (
                    MOSS,
                    0.64,
                    if patch_n > 0.45 { GRASS_DARK } else { DIRT },
                    0.28,
                )
            } else {
                (
                    DIRT,
                    0.70,
                    if patch_n > 0.55 {
                        GRASS_DARK
                    } else {
                        DIRT_DARK
                    },
                    0.25,
                )
            }
        }
        BlockType::Sand => (
            SAND,
            0.76,
            if patch_n > 0.62 { GRASS_LIGHT } else { DIRT },
            0.18,
        ),
        BlockType::Snow => (
            SNOW,
            0.78,
            if patch_n > 0.50 { SNOW_BLUE } else { STONE },
            0.20,
        ),
        BlockType::Stone
        | BlockType::IronOre
        | BlockType::SunstoneOre
        | BlockType::FrostcoreOre
        | BlockType::LivingRoot => (
            STONE,
            0.72,
            if patch_n > 0.58 {
                STONE_DARK
            } else {
                DIRT_DARK
            },
            0.22,
        ),
        BlockType::Water => (WATER_EDGE, 0.70, SAND, 0.16),
        BlockType::Wood => (DIRT_DARK, 0.70, GRASS_DARK, 0.18),
        BlockType::Air => {
            if macro_n > 0.90 {
                (SAND, 0.60, GRASS_LIGHT, 0.22)
            } else {
                (
                    GRASS,
                    0.70,
                    if patch_n > 0.55 { GRASS_LIGHT } else { MOSS },
                    0.24,
                )
            }
        }
    };

    let detail = micro_n * 0.10 + fine_n * 0.08;
    let total = base_w + sec_w;
    let mut r = (base_rgb[0] * base_w + sec_rgb[0] * sec_w) / total;
    let mut g = (base_rgb[1] * base_w + sec_rgb[1] * sec_w) / total;
    let mut b = (base_rgb[2] * base_w + sec_rgb[2] * sec_w) / total;

    r = (r * (1.0 + (detail - 0.5) * 0.22)).clamp(0.0, 1.0);
    g = (g * (1.0 + (detail - 0.5) * 0.22)).clamp(0.0, 1.0);
    b = (b * (1.0 + (detail - 0.5) * 0.22)).clamp(0.0, 1.0);

    let mut zone_tint = 1.0;
    if zone_n > 0.7 {
        zone_tint = 1.04;
    } else if zone_n < 0.3 {
        zone_tint = 0.92;
    }

    [r * zone_tint, g * zone_tint, b * zone_tint, 1.0]
}

fn surface_block_near(world: &GameWorld, p: [f32; 3]) -> BlockType {
    let x = p[0].floor() as i32;
    let z = p[2].floor() as i32;
    let y = p[1].floor() as i32;
    let mut fallback = None;

    for (dx, dz) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
        for sy in ((y - 4)..=(y + 4)).rev() {
            let block = world.get(x + dx, sy, z + dz);
            if !block.is_solid() {
                continue;
            }
            fallback.get_or_insert(block);
            if !world.get(x + dx, sy + 1, z + dz).is_solid() {
                return block;
            }
        }
    }

    fallback.unwrap_or(BlockType::Air)
}

fn laplacian_smooth(vertices: Vec<McVertex>, indices: &[u32], passes: u32) -> Vec<McVertex> {
    let mut verts = vertices;
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); verts.len()];
    for tri in indices.chunks_exact(3) {
        let a = tri[0] as usize;
        let b = tri[1] as usize;
        let c = tri[2] as usize;
        if a >= verts.len() || b >= verts.len() || c >= verts.len() {
            continue;
        }
        neighbors[a].push(b);
        neighbors[a].push(c);
        neighbors[b].push(a);
        neighbors[b].push(c);
        neighbors[c].push(a);
        neighbors[c].push(b);
    }

    for _ in 0..passes {
        let mut new_positions: Vec<[f32; 3]> = Vec::with_capacity(verts.len());
        for (i, vertex_neighbors) in neighbors.iter().enumerate() {
            if vertex_neighbors.is_empty() {
                new_positions.push(verts[i].position);
                continue;
            }
            let mut sum = [0.0; 3];
            for &neighbor in vertex_neighbors {
                let p = verts[neighbor].position;
                sum[0] += p[0];
                sum[1] += p[1];
                sum[2] += p[2];
            }
            let n = vertex_neighbors.len() as f32;
            let avg = [sum[0] / n, sum[1] / n, sum[2] / n];

            let orig = verts[i].position;
            new_positions.push([
                orig[0] * 0.5 + avg[0] * 0.5,
                orig[1] * 0.5 + avg[1] * 0.5,
                orig[2] * 0.5 + avg[2] * 0.5,
            ]);
        }
        for (i, p) in new_positions.iter().enumerate() {
            verts[i].position = *p;
        }
    }
    verts
}

fn smooth_normals(vertices: &[McVertex], _indices: &[u32]) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let positions: Vec<[f32; 3]> = vertices.iter().map(|v| v.position).collect();
    let normals: Vec<[f32; 3]> = vertices.iter().map(|v| v.normal).collect();
    (positions, normals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lk2_core::world::{World as GameWorld, install_huge_spawn_platform, terrain::presets};

    #[test]
    fn empty_world_no_mesh() {
        let world = GameWorld::new(128);

        let result = build_smooth_mesh(&world, [40, 0, 40], [60, 30, 60], 0.5, 0);

        assert!(result.is_none());
    }

    #[test]
    fn default_spawn_surface_samples_as_green_grass() {
        let pipeline = presets::by_name("default");
        let mut world = GameWorld::with_pipeline(96, pipeline);
        install_huge_spawn_platform(&mut world);

        let block = surface_block_near(&world, [48.5, 14.55, 48.5]);
        let low_smoothed_block = surface_block_near(&world, [48.5, 12.20, 48.5]);
        let color = terrain_color(&world, [48.5, 14.55, 48.5]);

        assert_eq!(block, BlockType::Grass);
        assert_eq!(low_smoothed_block, BlockType::Grass);
        assert!(
            color[1] > color[0] && color[1] > color[2],
            "grass terrain should be green-dominant, got rgba={color:?}"
        );
    }
}
