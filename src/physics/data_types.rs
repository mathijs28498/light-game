use nalgebra_glm as glm;

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