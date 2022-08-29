use nalgebra_glm as glm;

use bevy::prelude::*;

use crate::{
    general::data_types::*,
    glm_traits::{
        traits::*,
        traits_impl::*,
    },
    rendering::components::*,
};

// Components
#[derive(Component)]
pub struct EnvironmentObjectComp;

#[derive(Debug, Component)]
pub struct AABBComp {
    pub(crate) min: glm::Vec2,
    pub(crate) max: glm::Vec2,
    pub(crate) xminymax: glm::Vec2,
    pub(crate) xmaxymin: glm::Vec2,
    pub(crate) center: glm::Vec2,
}

#[derive(Debug, Component)]
pub struct LineComp {
    pub(crate) p0: glm::Vec2,
    pub(crate) p1: glm::Vec2,
}

#[derive(Debug, Component)]
pub struct DottedLineComp {
    pub(crate) bounding_line: LineComp,
    pub(crate) lines: Vec<LineComp>,
    pub(crate) gap_amount: u32,
}

// Implementations
impl AABBComp {
    pub fn new(min: glm::Vec2, max: glm::Vec2) -> Self {
        AABBComp {
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

impl LineComp {
    pub fn new(p0: glm::Vec2, p1: glm::Vec2) -> Self {
        LineComp { p0, p1 }
    }
}

impl DottedLineComp {
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
            lines.push(LineComp::new(p0 + offset_0, p0 + offset_1));
        }
        lines.push(LineComp::new(p1 - glm::Vec2::new(size, 0.), p1));

        Self {
            bounding_line: LineComp::new(p0, p1),
            lines,
            gap_amount,
        }
    }
}
