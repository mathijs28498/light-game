use nalgebra_glm as glm;

use crate::game_object::game_object::EnvironmentObject;

// TODO: Allow any iterable form as argument
pub fn get_all_points(env_objects: &Vec<Box<dyn EnvironmentObject>>) -> Vec<glm::Vec2> {
    let mut points = vec![];
    for go in env_objects {
        points.extend(go.get_corners().iter());
    }

    points
}

pub fn sort_clockwise(points: &mut Vec<glm::Vec2>, center: &glm::Vec2) {
    points.sort_by(|a, b| {
        let vec0 = a - center;
        let vec1 = b - center;
        let dir0 = vec0.normalize();
        let dir1 = vec1.normalize();

        let mut alpha0 = dir0[0].acos();
        if dir0[1] < 0. {
            alpha0 *= -1.;
        }

        let mut alpha1 = dir1[0].acos();
        if dir1[1] < 0. {
            alpha1 *= -1.;
        }

        if alpha0.is_nan() {
            alpha0 = 0.;
        }

        if alpha1.is_nan() {
            alpha1 = 0.;
        }

        alpha0.partial_cmp(&alpha1).expect(&format!(
            "Failed to order vectors, a: {:?} alpha0: {:?} dir0: {:?} - b: {:?} alpha1: {:?} dir1: {:?}",
            a, alpha0, dir0, b, alpha1, dir1
        ))
    });
}

pub fn calculate_indices_polygon(triangle_amount: usize) -> Vec<u32> {
    let mut indices = Vec::with_capacity(triangle_amount * 3);
    for i in 0..triangle_amount {
        let index = (i + 1) as u32;
        indices.push(0);
        indices.push(index);
        indices.push(index % triangle_amount as u32 + 1);
    }

    indices
}
