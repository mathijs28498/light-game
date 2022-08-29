use nalgebra_glm as glm;

use crate::{
    ext_traits::glm_traits::traits::*,
};

impl ExtraGLMVec2Func for glm::Vec2 {
    fn clamp(&self, min: &Self, max: &Self) -> Self {
        glm::Vec2::new(self.x.clamp(min.x, max.x), self.y.clamp(min.y, max.y))
    }
    
    fn cross_vec2(&self, b: &Self) -> f32 {
        self.x * b.y - self.y * b.x
    }
}