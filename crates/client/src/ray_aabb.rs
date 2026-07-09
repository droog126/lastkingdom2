use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RayAabb {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RayAabbHit {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

impl RayAabb {
    pub(crate) fn new(a: Vec3, b: Vec3) -> Self {
        Self { min: a.min(b), max: a.max(b) }
    }

    pub(crate) fn contains(self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    pub(crate) fn hit_ray(self, origin: Vec3, direction: Vec3) -> Option<RayAabbHit> {
        ray_aabb_hit(origin, direction, self.min, self.max)
    }
}

pub(crate) fn ray_aabb_hit(
    origin: Vec3,
    direction: Vec3,
    min: Vec3,
    max: Vec3,
) -> Option<RayAabbHit> {
    let aabb = RayAabb::new(min, max);
    let direction = direction.normalize_or_zero();
    if direction.length_squared() <= f32::EPSILON {
        return None;
    }

    let mut t_min = 0.0;
    let mut t_max = f32::INFINITY;
    let mut normal = Vec3::ZERO;

    for axis in 0..3 {
        let origin_axis = origin[axis];
        let direction_axis = direction[axis];
        let min_axis = aabb.min[axis];
        let max_axis = aabb.max[axis];

        if direction_axis.abs() <= f32::EPSILON {
            if origin_axis < min_axis || origin_axis > max_axis {
                return None;
            }
            continue;
        }

        let inv_direction = 1.0 / direction_axis;
        let mut near = (min_axis - origin_axis) * inv_direction;
        let mut far = (max_axis - origin_axis) * inv_direction;
        let mut axis_normal = Vec3::ZERO;
        axis_normal[axis] = -direction_axis.signum();

        if near > far {
            std::mem::swap(&mut near, &mut far);
        }

        if near > t_min {
            t_min = near;
            normal = axis_normal;
        }
        t_max = t_max.min(far);

        if t_min > t_max {
            return None;
        }
    }

    let distance = t_min.max(0.0);
    let point = origin + direction * distance;
    if distance == 0.0 && aabb.contains(origin) {
        normal = Vec3::ZERO;
    }
    Some(RayAabbHit { distance, point, normal })
}

pub(crate) fn nearest_ray_aabb_hit<I>(
    origin: Vec3,
    direction: Vec3,
    boxes: I,
) -> Option<(usize, RayAabbHit)>
where
    I: IntoIterator<Item = (usize, RayAabb)>,
{
    boxes
        .into_iter()
        .filter_map(|(index, aabb)| aabb.hit_ray(origin, direction).map(|hit| (index, hit)))
        .min_by(|(_, a), (_, b)| a.distance.total_cmp(&b.distance))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hits_front_face() {
        let aabb = RayAabb::new(Vec3::ZERO, Vec3::ONE);
        let hit = aabb.hit_ray(Vec3::new(0.5, 0.5, -2.0), Vec3::Z).expect("ray should hit box");

        assert!((hit.distance - 2.0).abs() < 0.0001);
        assert!((hit.point - Vec3::new(0.5, 0.5, 0.0)).length() < 0.0001);
        assert_eq!(hit.normal, Vec3::NEG_Z);
    }

    #[test]
    fn ray_misses_parallel_outside_slab() {
        let aabb = RayAabb::new(Vec3::ZERO, Vec3::ONE);

        assert_eq!(aabb.hit_ray(Vec3::new(2.0, 0.5, -2.0), Vec3::Z), None);
    }

    #[test]
    fn ray_starting_inside_hits_at_zero_distance() {
        let aabb = RayAabb::new(Vec3::ZERO, Vec3::ONE);
        let hit = aabb
            .hit_ray(Vec3::new(0.5, 0.5, 0.5), Vec3::X)
            .expect("origin inside box should count as a hit");

        assert_eq!(hit.distance, 0.0);
        assert_eq!(hit.point, Vec3::new(0.5, 0.5, 0.5));
        assert_eq!(hit.normal, Vec3::ZERO);
    }

    #[test]
    fn constructor_accepts_reversed_corners() {
        let aabb = RayAabb::new(Vec3::ONE, Vec3::ZERO);
        let hit = aabb
            .hit_ray(Vec3::new(0.5, 2.0, 0.5), Vec3::NEG_Y)
            .expect("ray should hit normalized box");

        assert!((hit.distance - 1.0).abs() < 0.0001);
        assert_eq!(hit.normal, Vec3::Y);
    }

    #[test]
    fn nearest_hit_picks_lowest_positive_distance() {
        let hit = nearest_ray_aabb_hit(
            Vec3::new(0.5, 0.5, -3.0),
            Vec3::Z,
            [
                (
                    10,
                    RayAabb::new(Vec3::new(0.0, 0.0, 3.0), Vec3::new(1.0, 1.0, 4.0)),
                ),
                (20, RayAabb::new(Vec3::ZERO, Vec3::ONE)),
            ],
        )
        .expect("ray should hit both boxes");

        assert_eq!(hit.0, 20);
        assert!((hit.1.distance - 3.0).abs() < 0.0001);
    }
}
