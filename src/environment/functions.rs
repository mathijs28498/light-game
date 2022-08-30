use nalgebra_glm as glm;

use crate::{
    environment::traits::*,
};

// TODO: Allow any iterable form as argument
pub(crate) fn get_all_points(env_objects: &Vec<Box<dyn EnvironmentObject>>) -> Vec<glm::Vec2> {
    let mut points = vec![];
    for go in env_objects {
        points.extend(go.get_corners().iter());
    }

    points
}