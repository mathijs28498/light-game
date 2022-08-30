use nalgebra_glm as glm;

use crate::{
    general::data_types::*,
    physics::data_types::*,
};

pub trait EnvironmentObject {
    fn ray_collision(&self, ray: &Ray, ignore_t: bool) -> Option<Collision>;
    fn circle_collision(&self, circle: &Circle) -> Option<Collision>;
    fn get_corners(&self) -> Vec<glm::Vec2>;
}