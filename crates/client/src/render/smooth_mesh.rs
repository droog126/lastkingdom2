








use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;

use super::marching_cubes::{McVertex, build_mesh as mc_build_mesh};
use super::scalar_field::{ScalarField, build_density_field};
use lk2_core::world::World as GameWorld;


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


    let colors: Vec<[f32; 4]> = positions.iter().map(|p| terrain_color(*p)).collect();


    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices.clone()));

    Some(SmoothMesh { mesh, collider_trimesh: positions, collider_indices: indices })
}













fn terrain_color(p: [f32; 3]) -> [f32; 4] {

    const GRASS: [f32; 3] = [0.38, 0.74, 0.24];
    const DIRT: [f32; 3] = [0.68, 0.45, 0.20];
    const STONE: [f32; 3] = [0.78, 0.76, 0.70];

    let zone_n = (p[0] * 0.08 + p[2] * 0.10).sin() * 0.5 + 0.5;
    let patch_n = (p[0] * 0.32 + p[2] * 0.28).sin() * 0.5 + 0.5;
    let fine_n = ((p[0] * 0.85).sin() * (p[2] * 0.73).cos()) * 0.5 + 0.5;
    let height_t = ((p[1] - 4.0) / 40.0).clamp(0.0, 1.0);


    let main_zone = zone_n * 0.85 + height_t * 0.15;
    let (main_rgb, main_w) = if main_zone < 0.40 {
        (GRASS, 0.60)
    } else if main_zone < 0.66 {
        (DIRT, 0.60)
    } else {
        (STONE, 0.60)
    };


    let sec_rgb = if main_zone < 0.40 {
        if patch_n > 0.55 { DIRT } else { STONE }
    } else if main_zone < 0.66 {
        if patch_n > 0.50 { GRASS } else { STONE }
    } else {
        if patch_n > 0.55 { DIRT } else { GRASS }
    };
    let sec_w = 0.25 + patch_n * 0.10;

    let total = main_w + sec_w;
    let shade = (fine_n - 0.5) * 0.06;
    [
        (((main_rgb[0] * main_w + sec_rgb[0] * sec_w) / total) * (1.0 + shade) * 1.25)
            .clamp(0.0, 1.0),
        (((main_rgb[1] * main_w + sec_rgb[1] * sec_w) / total) * (1.0 + shade) * 1.25)
            .clamp(0.0, 1.0),
        (((main_rgb[2] * main_w + sec_rgb[2] * sec_w) / total) * (1.0 + shade) * 1.25)
            .clamp(0.0, 1.0),
        1.0,
    ]
}



fn laplacian_smooth(vertices: Vec<McVertex>, indices: &[u32], passes: u32) -> Vec<McVertex> {
    let mut verts = vertices;
    for _ in 0..passes {
        let mut new_positions: Vec<[f32; 3]> = Vec::with_capacity(verts.len());
        for i in 0..verts.len() {

            let mut neighbors: Vec<[f32; 3]> = Vec::new();
            for tri in indices.chunks(3) {
                if tri.contains(&(i as u32)) {
                    for &vi in tri {
                        if vi != i as u32 {
                            neighbors.push(verts[vi as usize].position);
                        }
                    }
                }
            }
            if neighbors.is_empty() {
                new_positions.push(verts[i].position);
                continue;
            }
            let sum: [f32; 3] = neighbors.iter().fold([0.0; 3], |acc, n| {
                [acc[0] + n[0], acc[1] + n[1], acc[2] + n[2]]
            });
            let n = neighbors.len() as f32;
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
    use lk2_core::world::World as GameWorld;

    #[test]
    fn empty_world_no_mesh() {
        let world = GameWorld::new(128);

        let result = build_smooth_mesh(&world, [40, 0, 40], [60, 30, 60], 0.5, 0);

        assert!(result.is_none());
    }
}
