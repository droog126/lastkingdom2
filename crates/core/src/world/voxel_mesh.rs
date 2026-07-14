#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceNetsMesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

pub fn build_surface_nets<F>(cells: i32, sample: F) -> SurfaceNetsMesh
where
    F: Fn(i32, i32, i32) -> f32,
{
    build_surface_nets_rect([cells, cells, cells], sample)
}

pub fn build_surface_nets_rect<F>(cells: [i32; 3], sample: F) -> SurfaceNetsMesh
where
    F: Fn(i32, i32, i32) -> f32,
{
    let [cells_x, cells_y, cells_z] = cells;
    if cells_x <= 0 || cells_y <= 0 || cells_z <= 0 {
        return SurfaceNetsMesh::default();
    }

    let vertex_count = (cells_x * cells_y * cells_z) as usize;
    let mut vertices = vec![None; vertex_count];
    let mut mesh = SurfaceNetsMesh::default();

    for z in 0..cells_z {
        for y in 0..cells_y {
            for x in 0..cells_x {
                let corners = [
                    sample(x, y, z),
                    sample(x + 1, y, z),
                    sample(x + 1, y + 1, z),
                    sample(x, y + 1, z),
                    sample(x, y, z + 1),
                    sample(x + 1, y, z + 1),
                    sample(x + 1, y + 1, z + 1),
                    sample(x, y + 1, z + 1),
                ];
                let has_positive = corners.iter().any(|value| *value >= 0.0);
                let has_negative = corners.iter().any(|value| *value < 0.0);
                if !(has_positive && has_negative) {
                    continue;
                }

                let corner_positions = [
                    [x as f32, y as f32, z as f32],
                    [(x + 1) as f32, y as f32, z as f32],
                    [(x + 1) as f32, (y + 1) as f32, z as f32],
                    [x as f32, (y + 1) as f32, z as f32],
                    [x as f32, y as f32, (z + 1) as f32],
                    [(x + 1) as f32, y as f32, (z + 1) as f32],
                    [(x + 1) as f32, (y + 1) as f32, (z + 1) as f32],
                    [x as f32, (y + 1) as f32, (z + 1) as f32],
                ];
                const EDGES: [(usize, usize); 12] = [
                    (0, 1),
                    (1, 2),
                    (2, 3),
                    (3, 0),
                    (4, 5),
                    (5, 6),
                    (6, 7),
                    (7, 4),
                    (0, 4),
                    (1, 5),
                    (2, 6),
                    (3, 7),
                ];

                let mut position = [0.0; 3];
                let mut intersections = 0.0;
                for (a, b) in EDGES {
                    let da = corners[a];
                    let db = corners[b];
                    if (da >= 0.0) == (db >= 0.0) {
                        continue;
                    }
                    let t = (da / (da - db)).clamp(0.0, 1.0);
                    for axis in 0..3 {
                        position[axis] +=
                            corner_positions[a][axis]
                                + (corner_positions[b][axis] - corner_positions[a][axis]) * t;
                    }
                    intersections += 1.0;
                }
                if intersections == 0.0 {
                    continue;
                }
                for axis in 0..3 {
                    position[axis] /= intersections;
                }
                let index = mesh.positions.len() as u32;
                mesh.positions.push(position);
                vertices[cell_index(cells, x, y, z)] = Some(index);
            }
        }
    }

    for x in 0..cells_x {
        for y in 1..cells_y {
            for z in 1..cells_z {
                if sign_change(sample(x, y, z), sample(x + 1, y, z)) {
                    add_quad(
                        &mut mesh.indices,
                        adjacent_vertices(
                            &vertices,
                            cells,
                            [(x, y - 1, z - 1), (x, y, z - 1), (x, y, z), (x, y - 1, z)],
                        ),
                    );
                }
            }
        }
    }
    for x in 1..cells_x {
        for y in 0..cells_y {
            for z in 1..cells_z {
                if sign_change(sample(x, y, z), sample(x, y + 1, z)) {
                    add_quad(
                        &mut mesh.indices,
                        adjacent_vertices(
                            &vertices,
                            cells,
                            [(x - 1, y, z - 1), (x, y, z - 1), (x, y, z), (x - 1, y, z)],
                        ),
                    );
                }
            }
        }
    }
    for x in 1..cells_x {
        for y in 1..cells_y {
            for z in 0..cells_z {
                if sign_change(sample(x, y, z), sample(x, y, z + 1)) {
                    add_quad(
                        &mut mesh.indices,
                        adjacent_vertices(
                            &vertices,
                            cells,
                            [(x - 1, y - 1, z), (x, y - 1, z), (x, y, z), (x - 1, y, z)],
                        ),
                    );
                }
            }
        }
    }
    mesh
}

fn cell_index(cells: [i32; 3], x: i32, y: i32, z: i32) -> usize {
    let [cells_x, _, cells_z] = cells;
    ((y * cells_z + z) * cells_x + x) as usize
}

fn sign_change(a: f32, b: f32) -> bool {
    (a >= 0.0) != (b >= 0.0)
}

fn adjacent_vertices(
    vertices: &[Option<u32>],
    cells: [i32; 3],
    coordinates: [(i32, i32, i32); 4],
) -> Option<[u32; 4]> {
    let [cells_x, cells_y, cells_z] = cells;
    let values = coordinates.map(|(x, y, z)| {
        (x >= 0
            && y >= 0
            && z >= 0
            && x < cells_x
            && y < cells_y
            && z < cells_z)
            .then(|| vertices[cell_index(cells, x, y, z)])
            .flatten()
    });
    Some([values[0]?, values[1]?, values[2]?, values[3]?])
}

fn add_quad(indices: &mut Vec<u32>, quad: Option<[u32; 4]>) {
    let Some([a, b, c, d]) = quad else {
        return;
    };
    indices.extend([a, b, c, a, c, d]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fully_solid_volume_has_no_surface() {
        let mesh = build_surface_nets(4, |_, _, _| 1.0);
        assert!(mesh.positions.is_empty());
        assert!(mesh.indices.is_empty());
    }

    #[test]
    fn rounded_spherical_boundary_produces_connected_surface() {
        let mesh = build_surface_nets(8, |x, y, z| {
            let center = 4.0;
            let distance = ((x as f32 - center).powi(2)
                + (y as f32 - center).powi(2)
                + (z as f32 - center).powi(2))
            .sqrt();
            3.2 - distance
        });
        assert!(!mesh.positions.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);
    }

    #[test]
    fn rectangular_volume_supports_tall_chunk_sampling() {
        let mesh = build_surface_nets_rect([4, 12, 4], |x, y, z| {
            if x == 0 || x == 4 || z == 0 || z == 4 || y == 0 || y == 12 {
                -1.0
            } else {
                1.0
            }
        });
        assert!(!mesh.positions.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);
    }
}
