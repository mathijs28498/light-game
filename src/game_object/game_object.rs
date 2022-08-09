use crate::game_object::help_functions::calculate_clockwise_points;

use bevy::prelude::*;
use nalgebra_glm as glm;


#[derive(Component)]
pub struct EnvironmentObjectComp;

pub trait EnvironmentObject {
    fn ray_collision(&self, ray: &Ray, ignore_t: bool) -> Option<Collision>;
    fn get_corners(&self) -> Vec<glm::Vec2>;
}

#[derive(Debug)]
pub struct Collision {
    pub t: f32,
    pub collision_points: Vec<glm::Vec2>,
}

impl Collision {
    pub fn new(t: f32, collision_points: Vec<glm::Vec2>) -> Self {
        Self {
            t,
            collision_points,
        }
    }
}

#[derive(Debug)]
pub struct Ray {
    pub orig: glm::Vec2,
    pub dir: glm::Vec2,
    pub inv_dir: glm::Vec2,
    pub t: f32,
}

impl Ray {
    pub fn new(orig: glm::Vec2, dir: glm::Vec2, t: f32) -> Self {
        Self {
            orig,
            dir,
            inv_dir: glm::Vec2::new(1. / dir.x, 1. / dir.y),
            t,
        }
    }

    fn new_between_points(orig: glm::Vec2, p: &glm::Vec2, offset_perc: Option<f32>) -> Self {
        let mut dir = p - orig;
        let t = dir.normalize_mut();

        Self::new(orig, dir, t * offset_perc.unwrap_or(0.9))
    }

    fn new_from_angle(orig: glm::Vec2, angle: f32, t: f32) -> Self {
        Self::new(orig, glm::Vec2::new(angle.cos(), angle.sin()), t)
    }

    fn get_point_from_t(&self, t: f32) -> glm::Vec2 {
        self.orig + self.dir * t
    }

    fn get_angle_rays(&self, epsilon: Option<f32>) -> [Ray; 2] {
        let e = epsilon.unwrap_or(0.0001);
        let mut alpha = self.dir.x.acos();

        if self.dir.y < 0. {
            alpha *= -1.;
        }

        let alpha_min = alpha - e;
        let alpha_plus = alpha + e;

        return [
            Ray::new_from_angle(self.orig.clone(), alpha_min, self.t),
            Ray::new_from_angle(self.orig.clone(), alpha_plus, self.t),
        ];
    }
}

#[derive(Debug, Component)]
pub struct Line {
    p0: glm::Vec2,
    p1: glm::Vec2,
}

impl Line {
    pub fn new(p0: glm::Vec2, p1: glm::Vec2) -> Self {
        Line { p0, p1 }
    }
}

fn cross_vec2(a: &glm::Vec2, b: &glm::Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

impl EnvironmentObject for Line {
    fn ray_collision(&self, ray: &Ray, ignore_t: bool) -> Option<Collision> {
        let v0 = ray.orig - self.p0;
        let v1 = self.p1 - self.p0;
        let v2 = glm::Vec2::new(-ray.dir.y, ray.dir.x);
        let t0 = cross_vec2(&v1, &v0) / v1.dot(&v2);
        let t1 = v0.dot(&v2) / v1.dot(&v2);

        if t0 < 0. || t1 < 0. || t1 > 1. || (!ignore_t && t0 > ray.t) {
            return None;
        }
        Some(Collision::new(t0, vec![ray.get_point_from_t(t0)]))
    }

    fn get_corners(&self) -> Vec<glm::Vec2> {
        vec![self.p0, self.p1]
    }
}

#[derive(Debug, Component)]
pub struct AABB {
    pub min: glm::Vec2,
    pub max: glm::Vec2,
    xminymax: glm::Vec2,
    xmaxymin: glm::Vec2,
}

impl AABB {
    pub fn new(min: glm::Vec2, max: glm::Vec2) -> Self {
        AABB {
            min,
            max,
            xminymax: glm::Vec2::new(min.x, max.y),
            xmaxymin: glm::Vec2::new(max.x, min.y),
        }
    }
}

impl EnvironmentObject for AABB {
    fn ray_collision(&self, ray: &Ray, ignore_t: bool) -> Option<Collision> {
        let (mut tmin, mut tmax, tymin, tymax);

        if ray.inv_dir.x >= 0. {
            tmin = (self.min.x - ray.orig.x) * ray.inv_dir.x;
            tmax = (self.max.x - ray.orig.x) * ray.inv_dir.x;
        } else {
            tmin = (self.max.x - ray.orig.x) * ray.inv_dir.x;
            tmax = (self.min.x - ray.orig.x) * ray.inv_dir.x;
        }

        if ray.inv_dir.y >= 0. {
            tymin = (self.min.y - ray.orig.y) * ray.inv_dir.y;
            tymax = (self.max.y - ray.orig.y) * ray.inv_dir.y;
        } else {
            tymin = (self.max.y - ray.orig.y) * ray.inv_dir.y;
            tymax = (self.min.y - ray.orig.y) * ray.inv_dir.y;
        }

        if (tmin > tymax) || (tymin > tmax) {
            return None;
        }

        if tymin > tmin {
            tmin = tymin;
        }
        if tymax < tmax {
            tmax = tymax;
        }

        let t = if tmin > 0. { tmin } else { tmax };

        if !ignore_t && t > ray.t || t < 0. {
            return None;
        }

        Some(Collision::new(t, vec![ray.get_point_from_t(t)]))
    }

    fn get_corners(&self) -> Vec<glm::Vec2> {
        vec![self.min, self.max, self.xminymax, self.xmaxymin]
    }
}

// 2 dotted lines in parralel form optical illusion when light in center
#[derive(Debug, Component)]
pub struct DottedLine {
    bounding_line: Line,
    lines: Vec<Line>,
    gap_amount: u32,
}

impl DottedLine {
    pub fn new(p0: glm::Vec2, p1: glm::Vec2, gap_amount: u32) -> Self {
        // Use direction for offset
        let mut line_dir = p1 - p0;
        let magnitude = line_dir.magnitude();
        line_dir = line_dir / magnitude;

        let size = magnitude / (gap_amount as f32 * 2. + 1.);

        let mut lines = Vec::with_capacity(gap_amount as usize + 1);
        for i in 0..gap_amount {
            let offset = i as f32 * size * 2.;
            let offset_0 = line_dir * offset;
            let offset_1 = line_dir * (offset + size);
            lines.push(Line::new(p0 + offset_0, p0 + offset_1));
        }
        lines.push(Line::new(p1 - glm::Vec2::new(size, 0.), p1));

        Self {
            bounding_line: Line::new(p0, p1),
            lines,
            gap_amount,
        }
    }
}

impl EnvironmentObject for DottedLine {
    fn get_corners(&self) -> Vec<glm::Vec2> {
        let mut res = Vec::with_capacity(self.lines.len() * 2);

        for l in &self.lines {
            res.extend(l.get_corners());
        }

        res
    }

    fn ray_collision(&self, ray: &Ray, ignore_t: bool) -> Option<Collision> {
        // Check if this collision is a bottleneck
        //  - could be optimized to O = log(N) (in stead of O = N^^2) to look for bounding line and then subdivide
        for l in &self.lines {
            if let Some(coll) = l.ray_collision(ray, ignore_t) {
                return Some(coll);
            }
        }
        None
    }
}

#[derive(Component)]
pub struct Position {
    pub position: glm::Vec2,
}

#[derive(Component)]
pub struct Velocity {
    pub velocity: glm::Vec2,
    pub wanted_velocity: glm::Vec2,
    pub jump_pressed: bool,
}

#[derive(Component)]
pub struct MouseLight;

#[derive(Component)]
pub struct PlayerLight;

#[derive(Component)]
pub struct Light {
    pub color: glm::Vec3,
    radius: f32,
    max_radius: f32,
    pub brightness: f32,
    pub polygon: Option<Vec<glm::Vec2>>,
}

impl Light {
    pub fn new(color: glm::Vec3, radius: f32, brightness: f32) -> Self {
        Light {
            color,
            radius,
            max_radius: radius,
            brightness,
            polygon: None,
        }
    }

    pub fn new_with_max_radius(
        color: glm::Vec3,
        center: glm::Vec2,
        radius: f32,
        max_radius: f32,
        brightness: f32,
    ) -> Self {
        Light {
            color,
            radius,
            max_radius,
            brightness,
            polygon: None,
        }
    }

    pub fn get_radius(&self) -> f32 {
        return self.radius;
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
        self.maybe_set_max_radius(radius);
    }

    pub fn maybe_set_max_radius(&mut self, max_radius: f32) {
        if self.max_radius > max_radius {
            return;
        }
        self.max_radius = max_radius;
        self.polygon = None;
    }

    // TODO: Add optimizations back in
    pub fn calculate_light_polygon(
        &mut self,
        position: &Position,
        env_object_query: &Query<&AABB, With<EnvironmentObjectComp>>,
    ) -> (Vec<glm::Vec2>, bool) {
        if let Some(polygon) = &self.polygon {
            return (polygon.clone(), false);
        }

        // let mut p_rays_go: Vec<(&Box<dyn EnvironmentObject>, Vec<Ray>)> = Vec::new(); // Vec::with_capacity(points.len());
        let mut p_rays_go: Vec<Ray> = Vec::new(); // Vec::with_capacity(points.len());

        for go in env_object_query {
            // let mut ray_env_object = None;
            for p in go.get_corners() {
                let p_ray = Ray::new_between_points(position.position.clone(), &p, None);
                let mut add_point = true;
                for go_ in env_object_query {
                    if let Some(coll) = go_.ray_collision(&p_ray, false) {
                        add_point = false;
                        break;
                    }
                }

                if add_point {
                    p_rays_go.push(p_ray);
                    // if let Some(ray_go) = ray_env_object {
                    //     // let test = p_rays_go.last_mut().unwrap().1.push(p_ray);
                    //     p_rays_go.last_mut().expect("Couldn't get last item of p_rays_go").1.push(p_ray);
                    // } else {
                    //     p_rays_go.push((go, vec![p_ray]));
                    //     ray_env_object = Some(go);
                    // }
                }
            }
        }

        let mut actual_points: Vec<glm::Vec2> = Vec::with_capacity(p_rays_go.len() * 2);
        // for (ray_go, p_rays) in p_rays_go {
        for p_ray in p_rays_go {
            let p_angle_ray = p_ray.get_angle_rays(None);

            for p_ray in p_angle_ray {
                let mut t_near = f32::MAX;
                let mut closest_point = None;

                // if let Some(coll) = ray_go.ray_collision(&p_ray, true) {
                //     actual_points.push(coll.collision_points[0]);
                //     continue;
                // }

                for go in env_object_query {
                    if let Some(coll) = go.ray_collision(&p_ray, true) {
                        if coll.t < t_near {
                            t_near = coll.t;
                            closest_point = Some(coll.collision_points[0]);
                        }
                    }
                }

                if let Some(cp) = closest_point {
                    actual_points.push(cp);
                }
            }
        }
        // }

        let polygon = calculate_clockwise_points(actual_points, position.position);
        let polygon_c = polygon.clone();
        self.polygon = Some(polygon);

        (polygon_c, true)
    }
}
