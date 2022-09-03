use nalgebra_glm as glm;

pub(crate) trait ExtraGLMVec2Func {
    fn clamp(&self, min: &glm::Vec2, max: &glm::Vec2) -> glm::Vec2;
    fn cross_vec2(&self, b: &Self) -> f32;
}