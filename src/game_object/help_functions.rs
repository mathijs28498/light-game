use nalgebra_glm as glm;

use crate::game_object::game_object::GameObject;

// TODO: Allow any iterable form as argument
pub fn get_all_clockwise_points(game_objects: &Vec<impl GameObject>) -> Vec<glm::Vec2> {
    let mut points = vec![];
    for go in game_objects {
        points.extend(go.get_corners().iter());
    }

    get_clockwise_points(&points)
}

fn get_clockwise_points(points: &Vec<glm::Vec2>) -> Vec<glm::Vec2> {
    points.clone()
}