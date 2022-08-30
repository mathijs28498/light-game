use nalgebra_glm as glm;

pub(crate) fn sort_points_clockwise(points: &mut Vec<glm::Vec2>, center: &glm::Vec2) {
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