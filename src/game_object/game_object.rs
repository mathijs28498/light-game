use std::f32::consts::PI;

use crate::game_object::help_functions::sort_clockwise;

use bevy::prelude::*;
use nalgebra_glm as glm;

#[derive(Component)]
pub struct EnvironmentObjectComp;

pub trait EnvironmentObject {
    fn ray_collision(&self, ray: &Ray, ignore_t: bool) -> Option<Collision>;
    fn circle_collision(&self, circle: &Circle) -> Option<Collision>;
    fn get_corners(&self) -> Vec<glm::Vec2>;
}

pub struct Circle {
    center: glm::Vec2,
    radius: f32,
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

    fn get_point(&self) -> glm::Vec2 {
        self.get_point_from_t(self.t)
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

    fn circle_collision(&self, circle: &Circle) -> Option<Collision> {
        None
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
    center: glm::Vec2,
}

impl AABB {
    pub fn new(min: glm::Vec2, max: glm::Vec2) -> Self {
        AABB {
            min,
            max,
            xminymax: glm::Vec2::new(min.x, max.y),
            xmaxymin: glm::Vec2::new(max.x, min.y),
            center: (max - min) * 0.5 + min,
        }
    }

    pub fn get_circle_collision_points(&self, cp: &glm::Vec2, circle: &Circle) -> [glm::Vec2; 2] {
        let cp_to_circle = circle.center - cp;
        let a = cp_to_circle.norm();
        let cp_norm = cp_to_circle / a;
        let perp_cp = glm::Vec2::new(-cp_norm.y, cp_norm.x);

        let alpha = (a / circle.radius).acos();
        let o = alpha.sin() * circle.radius;

        [
            (perp_cp * o + cp).clamp(&self.min, &self.max),
            (perp_cp * -o + cp).clamp(&self.min, &self.max),
        ]
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

    fn circle_collision(&self, circle: &Circle) -> Option<Collision> {
        let rect_to_circle = circle.center - self.center;
        let rect_to_circle_clamped =
            rect_to_circle.clamp(&(self.min - self.center), &(self.max - self.center));

        let closest_point = rect_to_circle_clamped + self.center;
        let a_sq = (closest_point - circle.center).magnitude_squared();
        if a_sq > circle.radius * circle.radius {
            return None;
        }

        let cp0 = glm::Vec2::new(closest_point.x, circle.center.y);
        let cp1 = glm::Vec2::new(circle.center.x, closest_point.y);
        let mut collision_points: Vec<glm::Vec2> = Vec::new();

        if cp0 != circle.center {
            collision_points.extend(self.get_circle_collision_points(&cp0, &circle));
        }
        if cp1 != circle.center {
            collision_points.extend(self.get_circle_collision_points(&cp1, &circle));
        }

        Some(Collision::new(a_sq.sqrt(), collision_points))
    }

    fn get_corners(&self) -> Vec<glm::Vec2> {
        vec![self.min, self.max, self.xminymax, self.xmaxymin]
    }
}

pub trait ClampVec2 {
    fn clamp(&self, min: &glm::Vec2, max: &glm::Vec2) -> glm::Vec2;
}

impl ClampVec2 for glm::Vec2 {
    fn clamp(&self, min: &Self, max: &Self) -> Self {
        glm::Vec2::new(self.x.clamp(min.x, max.x), self.y.clamp(min.y, max.y))
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

    fn circle_collision(&self, circle: &Circle) -> Option<Collision> {
        None
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

#[derive(Debug, Component)]
pub struct Light {
    pub color: glm::Vec3,
    radius: f32,
    max_radius: f32,
    pub brightness: f32,
    pub polygon: Option<Vec<glm::Vec2>>,
    pub has_moved: bool,
    has_collided: bool,
}

impl Light {
    pub fn new(color: glm::Vec3, radius: f32, brightness: f32) -> Self {
        Light {
            color,
            radius,
            max_radius: radius,
            brightness,
            polygon: None,
            has_moved: false,
            has_collided: false,
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
            has_moved: false,
            has_collided: false,
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

    pub fn calculate_light_polygon(
        &mut self,
        position: &Position,
        env_object_query: &Query<&AABB, With<EnvironmentObjectComp>>,
    ) -> (Vec<glm::Vec2>, bool) {
        let collision_circle = Circle {
            radius: self.max_radius * 1.05,
            center: position.position.clone(),
        };
        // TODO: Fix jitter

        // TODO: Improve this!!
        // See if new polygon has to be calculated
        if let Some(polygon) = &self.polygon {
            if !self.has_moved {
                return (polygon.clone(), false);
            }

            if !self.has_collided {
                let mut has_collision = false;
                for env_obj in env_object_query {
                    if let Some(coll) = env_obj.circle_collision(&collision_circle) {
                        has_collision = true;
                        break;
                    }
                }
                if !has_collision {
                    return (polygon.clone(), false);
                }
            }
        }

        let mut circle_points = Vec::new();

        // Draw circle
        let amount_of_points = 10;
        let angle_diff = PI * 2. / amount_of_points as f32;
        for i in 0..amount_of_points {
            let mut angle = angle_diff * i as f32;
            circle_points.push(glm::Vec2::new(angle.cos(), angle.sin()) * collision_circle.radius);
        }

        let mut env_objects = Vec::new();
        let mut actual_points = Vec::new();

        // Collide with environment objects
        self.has_collided = false;
        for env_obj in env_object_query {
            if let Some(coll) = env_obj.circle_collision(&collision_circle) {
                self.has_collided = true;
                env_objects.push(env_obj);

                for cp in &coll.collision_points {
                    // TODO: Check among each other collided env_object
                    let p_ray =
                        Ray::new_between_points(collision_circle.center.clone(), cp, Some(1.));
                    let angle_rays = p_ray.get_angle_rays(None);
                    for r in angle_rays {
                        if let None = env_obj.ray_collision(&r, true) {
                            actual_points.push(r.dir * collision_circle.radius);
                        } else {
                            actual_points.push(cp - collision_circle.center);
                        }
                    }
                }
            }
        }

        // Adding circle points that dont collide with object
        for cp in circle_points {
            let mut add_circle_point = true;
            let ray = Ray::new_between_points(
                position.position.clone(),
                &(cp + position.position),
                None,
            );
            for env_obj in &env_objects {
                if let Some(_) = env_obj.ray_collision(&ray, false) {
                    add_circle_point = false;
                    break;
                }
            }
            if add_circle_point {
                actual_points.push(cp);
            }
        }

        sort_clockwise(&mut actual_points, &glm::Vec2::new(0., 0.));
        let polygon_c = actual_points.clone();
        self.polygon = Some(actual_points);
        self.has_moved = false;
        (polygon_c, true)
    }
}
