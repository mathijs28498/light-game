use nalgebra_glm as glm;

pub struct Circle {
    pub(crate) center: glm::Vec2,
    pub(crate) radius: f32,
}


#[derive(Debug)]
pub struct Ray {
    pub(crate) orig: glm::Vec2,
    pub(crate) dir: glm::Vec2,
    pub(crate) inv_dir: glm::Vec2,
    pub(crate) t: f32,
}

impl Ray {
    pub(crate) fn new(orig: glm::Vec2, dir: glm::Vec2, t: f32) -> Self {
        Self {
            orig,
            dir,
            inv_dir: glm::Vec2::new(1. / dir.x, 1. / dir.y),
            t,
        }
    }

    pub(crate) fn new_between_points(orig: glm::Vec2, p: &glm::Vec2, offset_perc: Option<f32>) -> Self {
        let mut dir = p - orig;
        let t = dir.normalize_mut();

        Self::new(orig, dir, t * offset_perc.unwrap_or(0.9))
    }

    pub(crate) fn new_from_angle(orig: glm::Vec2, angle: f32, t: f32) -> Self {
        Self::new(orig, glm::Vec2::new(angle.cos(), angle.sin()), t)
    }

    pub(crate) fn get_point_from_t(&self, t: f32) -> glm::Vec2 {
        self.orig + self.dir * t
    }

    pub(crate) fn get_point(&self) -> glm::Vec2 {
        self.get_point_from_t(self.t)
    }

    pub(crate) fn get_angle_rays(&self, epsilon: Option<f32>) -> [Ray; 2] {
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