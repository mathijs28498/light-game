use nalgebra_glm as glm;

use crate::{
    general::data_types::*,
    environment::{
        traits::*,
        components::*,
    },
    ext::glm_ext::{
        traits::*,
        traits_impl::*,
    },
    physics::data_types::*,
};

impl EnvironmentObject for AABBComp {
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

        Some(Collision::new(t, vec![ray.get_point_with_t(t)]))
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

        let cp0 = glm::vec2(closest_point.x, circle.center.y);
        let cp1 = glm::vec2(circle.center.x, closest_point.y);
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

impl EnvironmentObject for LineComp {
    fn ray_collision(&self, ray: &Ray, ignore_t: bool) -> Option<Collision> {
        let v0 = ray.orig - self.p0;
        let v1 = self.p1 - self.p0;
        let v2 = glm::vec2(-ray.dir.y, ray.dir.x);
        let t0 = v1.cross_vec2(&v0) / v1.dot(&v2);
        let t1 = v0.dot(&v2) / v1.dot(&v2);

        if t0 < 0. || t1 < 0. || t1 > 1. || (!ignore_t && t0 > ray.t) {
            return None;
        }
        Some(Collision::new(t0, vec![ray.get_point_with_t(t0)]))
    }

    fn circle_collision(&self, circle: &Circle) -> Option<Collision> {
        None
    }

    fn get_corners(&self) -> Vec<glm::Vec2> {
        vec![self.p0, self.p1]
    }
}


impl EnvironmentObject for DottedLineComp {
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