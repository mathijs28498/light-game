use nalgebra_glm as glm;

pub trait GameObject {
    fn ray_collision(&self, ray: &Ray) -> Option<RayCollision>;
    fn get_corners(&self) -> Vec<glm::Vec2>;
}

pub struct RayCollision {

}

pub struct Ray {
    orig: glm::Vec2,
    dir: glm::Vec2,
    t: f32,
}

pub struct Line {
    p0: glm::Vec2,
    p1: glm::Vec2,
}

impl Line {
    pub fn new(p0: glm::Vec2, p1: glm::Vec2) -> Self {
        Line{ p0, p1 }
    }
}

impl GameObject for Line {
    fn ray_collision(&self, ray: &Ray) -> Option<RayCollision> {
        None
    }

    fn get_corners(&self) -> Vec<glm::Vec2> {
        vec![self.p0, self.p1]
    }
}

pub struct AABB {
    min: glm::Vec2,
    max: glm::Vec2,
    xminymax: glm::Vec2,
    xmaxymin: glm::Vec2,
}

impl AABB {
    pub fn new(min: glm::Vec2, max: glm::Vec2) -> Self {
        AABB{ 
            min, max,
            xminymax: glm::Vec2::new(min.x, max.y),
            xmaxymin: glm::Vec2::new(max.x, min.y),
        }
    }
}

impl GameObject for AABB {
    fn ray_collision(&self, ray: &Ray) -> Option<RayCollision> {
        None
    }

    fn get_corners(&self) -> Vec<glm::Vec2> {
        vec![
            self.min,
            self.max,
            self.xminymax,
            self.xmaxymin,
        ]
    }
}