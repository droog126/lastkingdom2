













































use serde::{Deserialize, Serialize};

use crate::world::{Biome, BlockType, SEA_LEVEL};






#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FillMode {

    Replace(BlockType),

    Carve,

    Surface(BlockType),

    AdaptiveSurface {
        surface: BlockType,
        subsurface: BlockType,
    },
}

impl FillMode {
    pub fn is_carve(&self) -> bool {
        matches!(self, FillMode::Carve)
    }
}


pub trait Shape: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;



    fn contains(&self, x: i32, y: i32, z: i32) -> Option<bool>;



    fn surface_y_f32(&self, _x: i32, _z: i32) -> Option<f32> {
        None
    }


    fn biome_at(&self, _x: i32, _z: i32) -> Option<Biome> {
        None
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxShape {
    pub name: String,

    pub min: [i32; 3],

    pub max: [i32; 3],
}

impl Shape for BoxShape {
    fn name(&self) -> &str {
        &self.name
    }
    fn contains(&self, x: i32, y: i32, z: i32) -> Option<bool> {
        Some(
            x >= self.min[0]
                && x <= self.max[0]
                && y >= self.min[1]
                && y <= self.max[1]
                && z >= self.min[2]
                && z <= self.max[2],
        )
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SphereShape {
    pub name: String,
    pub center: [i32; 3],
    pub radius: f32,
}

impl Shape for SphereShape {
    fn name(&self) -> &str {
        &self.name
    }
    fn contains(&self, x: i32, y: i32, z: i32) -> Option<bool> {
        let dx = (x - self.center[0]) as f32;
        let dy = (y - self.center[1]) as f32;
        let dz = (z - self.center[2]) as f32;
        let d2 = dx * dx + dy * dy + dz * dz;
        Some(d2 <= self.radius * self.radius)
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EllipsoidShape {
    pub name: String,
    pub center: [i32; 3],

    pub radii: [f32; 3],
}

impl Shape for EllipsoidShape {
    fn name(&self) -> &str {
        &self.name
    }
    fn contains(&self, x: i32, y: i32, z: i32) -> Option<bool> {
        let rx = self.radii[0];
        let ry = self.radii[1];
        let rz = self.radii[2];
        if rx <= 0.0 || ry <= 0.0 || rz <= 0.0 {
            return Some(false);
        }
        let nx = (x - self.center[0]) as f32 / rx;
        let ny = (y - self.center[1]) as f32 / ry;
        let nz = (z - self.center[2]) as f32 / rz;
        Some(nx * nx + ny * ny + nz * nz <= 1.0)
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CylinderShape {
    pub name: String,
    pub center_x: i32,
    pub center_z: i32,
    pub y_min: i32,
    pub y_max: i32,
    pub radius: f32,
}

impl Shape for CylinderShape {
    fn name(&self) -> &str {
        &self.name
    }
    fn contains(&self, x: i32, y: i32, z: i32) -> Option<bool> {
        if y < self.y_min || y > self.y_max {
            return Some(false);
        }
        let dx = (x - self.center_x) as f32;
        let dz = (z - self.center_z) as f32;
        Some(dx * dx + dz * dz <= self.radius * self.radius)
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaneShape {
    pub name: String,

    pub coeffs: [f32; 4],

    pub normalize: bool,
}

impl Shape for PlaneShape {
    fn name(&self) -> &str {
        &self.name
    }
    fn contains(&self, x: i32, y: i32, z: i32) -> Option<bool> {
        let c = if self.normalize {
            let len = (self.coeffs[0] * self.coeffs[0]
                + self.coeffs[1] * self.coeffs[1]
                + self.coeffs[2] * self.coeffs[2])
                .sqrt();
            if len <= 0.0 {
                return Some(false);
            }
            [
                self.coeffs[0] / len,
                self.coeffs[1] / len,
                self.coeffs[2] / len,
                self.coeffs[3] / len,
            ]
        } else {
            self.coeffs
        };
        let v = c[0] * x as f32 + c[1] * y as f32 + c[2] * z as f32 + c[3];

        Some(if c[1] > 0.0 { v <= 0.0 } else { v >= 0.0 })
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HillShape {
    pub name: String,
    pub center_x: i32,
    pub center_z: i32,
    pub radius: f32,
    pub max_height: f32,
}

impl Shape for HillShape {
    fn name(&self) -> &str {
        &self.name
    }
    fn contains(&self, x: i32, y: i32, z: i32) -> Option<bool> {
        let dx = (x - self.center_x) as f32;
        let dz = (z - self.center_z) as f32;
        let dist2 = dx * dx + dz * dz;
        let r = self.radius;
        if dist2 > r * r {
            return Some(false);
        }
        let dist = dist2.sqrt();
        let t = (1.0 - dist / r).clamp(0.0, 1.0);
        let dome_h = (1.0 - (t * std::f32::consts::FRAC_PI_2).cos()) * self.max_height;
        let surface = SEA_LEVEL as f32 + 1.0 + dome_h;
        Some((y as f32) <= surface)
    }

    fn surface_y_f32(&self, x: i32, z: i32) -> Option<f32> {
        let dx = (x - self.center_x) as f32;
        let dz = (z - self.center_z) as f32;
        let dist2 = dx * dx + dz * dz;
        let r = self.radius;
        if dist2 > r * r {
            return None;
        }
        let dist = dist2.sqrt();
        let t = (1.0 - dist / r).clamp(0.0, 1.0);
        let dome_h = (1.0 - (t * std::f32::consts::FRAC_PI_2).cos()) * self.max_height;
        Some(SEA_LEVEL as f32 + 1.0 + dome_h)
    }
}






fn noise3_value(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    use crate::world::terrain::hash01;
    let xi = x as f32;
    let yi = y as f32;
    let zi = z as f32;
    let c000 = hash01(x, y, z, seed);
    let c100 = hash01(x + 1, y, z, seed);
    let c010 = hash01(x, y + 1, z, seed);
    let c110 = hash01(x + 1, y + 1, z, seed);
    let c001 = hash01(x, y, z + 1, seed);
    let c101 = hash01(x + 1, y, z + 1, seed);
    let c011 = hash01(x, y + 1, z + 1, seed);
    let c111 = hash01(x + 1, y + 1, z + 1, seed);
    let xf = xi.fract();
    let yf = yi.fract();
    let zf = zi.fract();
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let w = zf * zf * (3.0 - 2.0 * zf);
    let x00 = c000 * (1.0 - u) + c100 * u;
    let x10 = c010 * (1.0 - u) + c110 * u;
    let x01 = c001 * (1.0 - u) + c101 * u;
    let x11 = c011 * (1.0 - u) + c111 * u;
    let y0 = x00 * (1.0 - v) + x10 * v;
    let y1 = x01 * (1.0 - v) + x11 * v;
    y0 * (1.0 - w) + y1 * w
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseFieldShape {
    pub name: String,
    pub seed: u64,

    pub freq: [f32; 3],

    pub threshold: f32,
}

impl Shape for NoiseFieldShape {
    fn name(&self) -> &str {
        &self.name
    }
    fn contains(&self, x: i32, y: i32, z: i32) -> Option<bool> {
        let fx = (x as f32 * self.freq[0]) as i32;
        let fy = (y as f32 * self.freq[1]) as i32;
        let fz = (z as f32 * self.freq[2]) as i32;
        let n = noise3_value(fx, fy, fz, self.seed as u32);
        Some(n > self.threshold)
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseHillShape {
    pub name: String,
    pub center_x: i32,
    pub center_z: i32,
    pub radius: f32,
    pub seed: u64,
    pub freq: f32,
    pub peak_height: f32,
}

impl NoiseHillShape {

    pub fn height_at(&self, x: i32, z: i32) -> f32 {
        let dx = (x - self.center_x) as f32;
        let dz = (z - self.center_z) as f32;
        let d2 = dx * dx + dz * dz;
        let r = self.radius;
        if d2 > r * r {
            return SEA_LEVEL as f32;
        }
        let t = 1.0 - (d2.sqrt() / r);

        let n = noise3_value(
            (x as f32 * self.freq) as i32,
            0,
            (z as f32 * self.freq) as i32,
            self.seed as u32,
        );

        SEA_LEVEL as f32 + 1.0 + t * self.peak_height * n
    }
}

impl Shape for NoiseHillShape {
    fn name(&self) -> &str {
        &self.name
    }
    fn contains(&self, x: i32, y: i32, z: i32) -> Option<bool> {
        let h = self.height_at(x, z);
        Some((y as f32) <= h)
    }
    fn surface_y_f32(&self, x: i32, z: i32) -> Option<f32> {
        Some(self.height_at(x, z))
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtractShape {
    pub name: String,

    pub base: Box<ShapeSpec>,
    pub subtract: Box<ShapeSpec>,
}

impl Shape for SubtractShape {
    fn name(&self) -> &str {
        &self.name
    }
    fn contains(&self, x: i32, y: i32, z: i32) -> Option<bool> {
        let in_base = eval_spec(&self.base, x, y, z)?;
        let in_sub = eval_spec(&self.subtract, x, y, z)?;
        Some(in_base && !in_sub)
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ShapeSpec {
    #[serde(rename = "box")]
    Box(BoxShape),
    #[serde(rename = "sphere")]
    Sphere(SphereShape),
    #[serde(rename = "ellipsoid")]
    Ellipsoid(EllipsoidShape),
    #[serde(rename = "cylinder")]
    Cylinder(CylinderShape),
    #[serde(rename = "plane")]
    Plane(PlaneShape),
    #[serde(rename = "hill")]
    Hill(HillShape),
    #[serde(rename = "noise_field")]
    NoiseField(NoiseFieldShape),
    #[serde(rename = "noise_hill")]
    NoiseHill(NoiseHillShape),
    #[serde(rename = "subtract")]
    Subtract(SubtractShape),
}

impl ShapeSpec {
    pub fn name(&self) -> &str {
        match self {
            ShapeSpec::Box(s) => s.name(),
            ShapeSpec::Sphere(s) => s.name(),
            ShapeSpec::Ellipsoid(s) => s.name(),
            ShapeSpec::Cylinder(s) => s.name(),
            ShapeSpec::Plane(s) => s.name(),
            ShapeSpec::Hill(s) => s.name(),
            ShapeSpec::NoiseField(s) => s.name(),
            ShapeSpec::NoiseHill(s) => s.name(),
            ShapeSpec::Subtract(s) => s.name(),
        }
    }
}


pub fn eval_spec(spec: &ShapeSpec, x: i32, y: i32, z: i32) -> Option<bool> {
    match spec {
        ShapeSpec::Box(s) => s.contains(x, y, z),
        ShapeSpec::Sphere(s) => s.contains(x, y, z),
        ShapeSpec::Ellipsoid(s) => s.contains(x, y, z),
        ShapeSpec::Cylinder(s) => s.contains(x, y, z),
        ShapeSpec::Plane(s) => s.contains(x, y, z),
        ShapeSpec::Hill(s) => s.contains(x, y, z),
        ShapeSpec::NoiseField(s) => s.contains(x, y, z),
        ShapeSpec::NoiseHill(s) => s.contains(x, y, z),
        ShapeSpec::Subtract(s) => s.contains(x, y, z),
    }
}


pub fn surface_y_f32_spec(spec: &ShapeSpec, x: i32, z: i32) -> Option<f32> {
    match spec {
        ShapeSpec::Box(_)
        | ShapeSpec::Sphere(_)
        | ShapeSpec::Ellipsoid(_)
        | ShapeSpec::Cylinder(_)
        | ShapeSpec::Plane(_)
        | ShapeSpec::NoiseField(_)
        | ShapeSpec::Subtract(_) => None,
        ShapeSpec::Hill(s) => s.surface_y_f32(x, z),
        ShapeSpec::NoiseHill(s) => s.surface_y_f32(x, z),
    }
}







#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeLayer {
    pub name: String,
    pub weight: f32,
    pub fill: FillMode,
    pub shapes: Vec<ShapeSpec>,

    pub biome_override: Option<Biome>,

    pub enabled: bool,
}

impl Default for ShapeLayer {
    fn default() -> Self {
        Self {
            name: "shape_layer".into(),
            weight: 5.0,
            fill: FillMode::Replace(BlockType::Stone),
            shapes: vec![],
            biome_override: None,
            enabled: true,
        }
    }
}

impl ShapeLayer {
    pub fn new(name: impl Into<String>, weight: f32, fill: FillMode) -> Self {
        Self {
            name: name.into(),
            weight,
            fill,
            shapes: vec![],
            biome_override: None,
            enabled: true,
        }
    }

    pub fn with_shape(mut self, spec: ShapeSpec) -> Self {
        self.shapes.push(spec);
        self
    }

    pub fn with_biome(mut self, b: Biome) -> Self {
        self.biome_override = Some(b);
        self
    }


    pub fn evaluate(&self, x: i32, y: i32, z: i32) -> Option<BlockType> {
        if !self.enabled {
            return None;
        }
        let mut any_inside = false;
        let mut any_outside = None;
        let mut any_surface: Option<f32> = None;
        for spec in &self.shapes {
            match eval_spec(spec, x, y, z) {
                Some(true) => {
                    any_inside = true;
                    if any_surface.is_none() {
                        any_surface = surface_y_f32_spec(spec, x, z);
                    }
                }
                Some(false) => {
                    if any_outside.is_none() {
                        any_outside = Some(false);
                    }
                }
                None => {}
            }
        }
        if !any_inside {
            return None;
        }
        match &self.fill {
            FillMode::Replace(b) => Some(*b),
            FillMode::Carve => Some(BlockType::Air),
            FillMode::Surface(b) => {

                if !self.any_inside(x, y + 1, z) {
                    Some(*b)
                } else {
                    None
                }
            }
            FillMode::AdaptiveSurface { surface, subsurface } => {
                if let Some(h) = any_surface {


                    let h_int = h.round() as i32;
                    let dy = h_int - y;
                    if dy == 1 {
                        Some(*surface)
                    } else if dy >= 2 && dy <= 4 {
                        Some(*subsurface)
                    } else if dy > 4 {
                        Some(BlockType::Stone)
                    } else {

                        None
                    }
                } else {

                    Some(*subsurface)
                }
            }
        }
    }

    fn any_inside(&self, x: i32, y: i32, z: i32) -> bool {
        self.shapes.iter().any(|spec| eval_spec(spec, x, y, z) == Some(true))
    }
}





use crate::world::terrain::{TerrainContext, TerrainModule};

impl TerrainModule for ShapeLayer {
    fn name(&self) -> &str {
        &self.name
    }
    fn weight(&self) -> f32 {
        self.weight
    }

    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType> {
        let b = self.evaluate(ctx.x, ctx.y, ctx.z)?;

        if let Some(biome) = self.biome_override {
            ctx.biome = Some(biome);
        }
        Some(b)
    }

    fn surface_f32(&self, x: i32, z: i32) -> Option<f32> {

        let mut best: Option<f32> = None;
        for spec in &self.shapes {
            if let Some(h) = surface_y_f32_spec(spec, x, z) {
                if let Some(b) = best {
                    if h > b {
                        best = Some(h);
                    }
                } else {
                    best = Some(h);
                }
            }
        }
        best
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ModuleSpec {

    #[serde(rename = "builtin")]
    Builtin { name: String },

    #[serde(rename = "shape_layer")]
    ShapeLayer(ShapeLayer),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    pub name: String,
    pub modules: Vec<ModuleSpec>,
    pub vertical_min: i32,
    pub vertical_max: i32,
    pub seed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_contains_interior_and_boundary() {
        let b = BoxShape { name: "b".into(), min: [0, 0, 0], max: [3, 3, 3] };

        assert_eq!(b.contains(0, 0, 0), Some(true));
        assert_eq!(b.contains(3, 3, 3), Some(true));

        assert_eq!(b.contains(1, 2, 3), Some(true));

        assert_eq!(b.contains(-1, 0, 0), Some(false));
        assert_eq!(b.contains(4, 0, 0), Some(false));
        assert_eq!(b.contains(0, 4, 0), Some(false));
    }

    #[test]
    fn sphere_contains_radius() {
        let s = SphereShape { name: "s".into(), center: [0, 0, 0], radius: 5.0 };
        assert_eq!(s.contains(0, 0, 0), Some(true));
        assert_eq!(s.contains(5, 0, 0), Some(true));
        assert_eq!(s.contains(6, 0, 0), Some(false));
        assert_eq!(s.contains(3, 4, 0), Some(true));
        assert_eq!(s.contains(3, 4, 1), Some(false));
    }

    #[test]
    fn ellipsoid_non_uniform() {
        let e = EllipsoidShape { name: "e".into(), center: [0, 0, 0], radii: [10.0, 2.0, 10.0] };
        assert_eq!(e.contains(0, 0, 0), Some(true));
        assert_eq!(e.contains(10, 0, 0), Some(true));
        assert_eq!(e.contains(0, 2, 0), Some(true));
        assert_eq!(e.contains(0, 3, 0), Some(false));

        assert_eq!(e.contains(5, 1, 0), Some(true));

        assert_eq!(e.contains(6, 2, 0), Some(false));
    }

    #[test]
    fn cylinder_vertical_clamp() {
        let c = CylinderShape {
            name: "c".into(),
            center_x: 0,
            center_z: 0,
            y_min: 5,
            y_max: 10,
            radius: 3.0,
        };
        assert_eq!(c.contains(0, 5, 0), Some(true));
        assert_eq!(c.contains(0, 10, 0), Some(true));
        assert_eq!(c.contains(0, 4, 0), Some(false));
        assert_eq!(c.contains(0, 11, 0), Some(false));
        assert_eq!(c.contains(3, 7, 0), Some(true));
        assert_eq!(c.contains(4, 7, 0), Some(false));
    }

    #[test]
    fn plane_lower_half() {
        let p = PlaneShape {
            name: "p".into(),
            coeffs: [0.0, 1.0, 0.0, -5.0],
            normalize: false,
        };
        assert_eq!(p.contains(0, 5, 0), Some(true));
        assert_eq!(p.contains(0, 0, 0), Some(true));
        assert_eq!(p.contains(0, 6, 0), Some(false));
    }

    #[test]
    fn hill_surface_geometry() {
        let h = HillShape {
            name: "h".into(),
            center_x: 0,
            center_z: 0,
            radius: 10.0,
            max_height: 20.0,
        };

        let surf_center = h.surface_y_f32(0, 0).unwrap();
        assert!(surf_center > (SEA_LEVEL as f32 + 15.0));

        let surf_edge = h.surface_y_f32(10, 0).unwrap();
        assert!((surf_edge - (SEA_LEVEL as f32 + 1.0)).abs() < 0.01);

        assert_eq!(h.surface_y_f32(11, 0), None);

        let inside_y = surf_center as i32 - 5;
        assert_eq!(h.contains(0, inside_y, 0), Some(true));

        let above_y = surf_center as i32 + 5;
        assert_eq!(h.contains(0, above_y, 0), Some(false));
    }

    #[test]
    fn noise_field_is_deterministic() {
        let n1 =
            NoiseFieldShape { name: "n".into(), seed: 42, freq: [0.1, 0.1, 0.1], threshold: 0.5 };
        let n2 = n1.clone();
        for x in -10..10 {
            for y in -10..10 {
                for z in -10..10 {
                    assert_eq!(
                        n1.contains(x, y, z),
                        n2.contains(x, y, z),
                        "noise mismatch at ({},{},{})",
                        x,
                        y,
                        z
                    );
                }
            }
        }
    }

    #[test]
    fn noise_hill_height_falls_off() {
        let h = NoiseHillShape {
            name: "nh".into(),
            center_x: 0,
            center_z: 0,
            radius: 20.0,
            seed: 0xCAFE,
            freq: 0.05,
            peak_height: 30.0,
        };

        let c = h.height_at(0, 0);
        let e = h.height_at(15, 0);
        assert!(c >= e);

        assert_eq!(h.height_at(30, 0), SEA_LEVEL as f32);
    }

    #[test]
    fn subtract_carves_out_interior() {

        let sub = SubtractShape {
            name: "donut".into(),
            base: Box::new(ShapeSpec::Sphere(SphereShape {
                name: "big".into(),
                center: [0, 0, 0],
                radius: 10.0,
            })),
            subtract: Box::new(ShapeSpec::Sphere(SphereShape {
                name: "small".into(),
                center: [0, 0, 0],
                radius: 5.0,
            })),
        };

        assert_eq!(sub.contains(0, 0, 0), Some(false));

        assert_eq!(sub.contains(7, 0, 0), Some(true));

        assert_eq!(sub.contains(11, 0, 0), Some(false));
    }

    #[test]
    fn shape_layer_replace_fills_interior() {
        let layer =
            ShapeLayer::new("stone_box", 5.0, FillMode::Replace(BlockType::Stone)).with_shape(
                ShapeSpec::Box(BoxShape { name: "b".into(), min: [0, 0, 0], max: [4, 4, 4] }),
            );
        assert_eq!(layer.evaluate(2, 2, 2), Some(BlockType::Stone));

        assert_eq!(layer.evaluate(10, 2, 2), None);
    }

    #[test]
    fn shape_layer_carve_only_hits_solid() {

        let layer = ShapeLayer {
            name: "carve_test".into(),
            weight: 5.0,
            fill: FillMode::Carve,
            shapes: vec![ShapeSpec::Sphere(SphereShape {
                name: "s".into(),
                center: [0, 0, 0],
                radius: 5.0,
            })],
            biome_override: None,
            enabled: true,
        };
        assert_eq!(layer.evaluate(0, 0, 0), Some(BlockType::Air));
        assert_eq!(layer.evaluate(10, 0, 0), None);
    }

    #[test]
    fn shape_layer_multiple_shapes_union() {

        let layer = ShapeLayer {
            name: "two".into(),
            weight: 1.0,
            fill: FillMode::Replace(BlockType::Wood),
            shapes: vec![
                ShapeSpec::Box(BoxShape { name: "a".into(), min: [0, 0, 0], max: [2, 2, 2] }),
                ShapeSpec::Box(BoxShape { name: "b".into(), min: [5, 0, 0], max: [7, 2, 2] }),
            ],
            biome_override: None,
            enabled: true,
        };
        assert_eq!(layer.evaluate(1, 0, 0), Some(BlockType::Wood));
        assert_eq!(layer.evaluate(6, 0, 0), Some(BlockType::Wood));
        assert_eq!(layer.evaluate(3, 0, 0), None);
    }

    #[test]
    fn shape_layer_surface_only_top() {

        let layer = ShapeLayer {
            name: "surf".into(),
            weight: 1.0,
            fill: FillMode::Surface(BlockType::Dirt),
            shapes: vec![ShapeSpec::Sphere(SphereShape {
                name: "s".into(),
                center: [0, 0, 0],
                radius: 5.0,
            })],
            biome_override: None,
            enabled: true,
        };

        assert_eq!(layer.evaluate(0, 5, 0), Some(BlockType::Dirt));

        assert_eq!(layer.evaluate(0, 4, 0), None);

        assert_eq!(layer.evaluate(10, 0, 0), None);
    }

    #[test]
    fn shape_layer_adaptive_surface() {

        let nh = NoiseHillShape {
            name: "nh".into(),
            center_x: 0,
            center_z: 0,
            radius: 20.0,
            seed: 0xCAFE,
            freq: 0.05,
            peak_height: 30.0,
        };
        let layer = ShapeLayer {
            name: "hill".into(),
            weight: 5.0,
            fill: FillMode::AdaptiveSurface {
                surface: BlockType::Dirt,
                subsurface: BlockType::Stone,
            },
            shapes: vec![ShapeSpec::NoiseHill(nh.clone())],
            biome_override: None,
            enabled: true,
        };

        let h = nh.height_at(0, 0).round() as i32;

        assert_eq!(layer.evaluate(0, h, 0), None);

        assert_eq!(layer.evaluate(0, h - 1, 0), Some(BlockType::Dirt));

        assert_eq!(layer.evaluate(0, h - 2, 0), Some(BlockType::Stone));

        assert_eq!(layer.evaluate(0, h - 4, 0), Some(BlockType::Stone));

        assert_eq!(layer.evaluate(0, h - 6, 0), Some(BlockType::Stone));

        assert_eq!(layer.evaluate(50, 50, 50), None);
    }

    #[test]
    fn shape_layer_biome_override() {

        let mut ctx = TerrainContext {
            x: 0,
            y: 0,
            z: 0,
            seed: 0,
            surface_y: None,
            biome: Some(Biome::Desert),
        };
        let layer = ShapeLayer {
            name: "jungle".into(),
            weight: 1.0,
            fill: FillMode::Replace(BlockType::Leaves),
            shapes: vec![ShapeSpec::Sphere(SphereShape {
                name: "s".into(),
                center: [0, 0, 0],
                radius: 3.0,
            })],
            biome_override: Some(Biome::Jungle),
            enabled: true,
        };
        use crate::world::terrain::TerrainModule;
        let _ = layer.decide(&mut ctx);
        assert_eq!(ctx.biome, Some(Biome::Jungle));
    }

    #[test]
    fn json_roundtrip_pipeline_spec() {
        let json = r#"{
            "name": "demo",
            "modules": [
                {"kind": "shape_layer", "name": "island", "weight": 9.0, "fill": {"Replace": "Dirt"}, "shapes": [
                    {"kind": "ellipsoid", "name": "e", "center": [0, 5, 0], "radii": [10.0, 4.0, 10.0]}
                ], "biome_override": "Jungle", "enabled": true}
            ],
            "vertical_min": 0,
            "vertical_max": 128,
            "seed": 42
        }"#;
        let spec: PipelineSpec = serde_json::from_str(json).expect("parse");
        assert_eq!(spec.name, "demo");
        assert_eq!(spec.modules.len(), 1);

        let back = serde_json::to_string(&spec).unwrap();
        let spec2: PipelineSpec = serde_json::from_str(&back).unwrap();
        assert_eq!(spec2.name, spec.name);
        assert_eq!(spec2.modules.len(), 1);
    }

    #[test]
    fn json_roundtrip_all_shape_kinds() {
        let json = r#"{
            "name": "all_shapes",
            "modules": [
                {"kind": "shape_layer", "name": "all", "weight": 1.0, "fill": {"Replace": "Stone"}, "shapes": [
                    {"kind": "box", "name": "b", "min": [0,0,0], "max": [1,1,1]},
                    {"kind": "sphere", "name": "s", "center": [5,5,5], "radius": 3.0},
                    {"kind": "cylinder", "name": "c", "center_x": 10, "center_z": 10, "y_min": 0, "y_max": 5, "radius": 2.0},
                    {"kind": "hill", "name": "h", "center_x": 0, "center_z": 0, "radius": 10.0, "max_height": 20.0},
                    {"kind": "noise_hill", "name": "nh", "center_x": 0, "center_z": 0, "radius": 20.0, "seed": 42, "freq": 0.05, "peak_height": 30.0},
                    {"kind": "subtract", "name": "sub", "base": {"kind": "sphere", "name": "a", "center": [0,0,0], "radius": 10.0}, "subtract": {"kind": "sphere", "name": "b", "center": [0,0,0], "radius": 5.0}}
                ], "biome_override": null, "enabled": true}
            ],
            "vertical_min": 0,
            "vertical_max": 128,
            "seed": 0
        }"#;
        let spec: PipelineSpec = serde_json::from_str(json).expect("parse all kinds");
        assert_eq!(spec.modules.len(), 1);
    }
}
