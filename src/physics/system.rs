use bevy::app::App;
use bevy::app::Plugin;
use bevy::ecs::system::Res;
use bevy::input::{keyboard::KeyCode, Input};
use bevy::prelude::*;

use nalgebra_glm as glm;

use crate::{
    environment::{components::*, traits::*, traits_impl::*},
    general::{components::*, data_types::*},
    player::components::*,
    rendering::components::*,
};

pub(crate) fn player_input_system(
    mut player_query: Query<(&mut VelocityComp, &mut PositionComp), With<PlayerLightComp>>,
    aabb_query: Query<&AABBComp, With<EnvironmentObjectComp>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    let speed_mult = 300.;
    let (mut velocity, mut position) = player_query.single_mut();
    let mut velocity_vec = glm::vec2(0., 0.);
    velocity_vec += glm::vec2(0., 0.01);

    let ray = Ray::new(position.position.clone(), velocity_vec.normalize(), 50.);
    let mut grounded = false;

    for aabb in &aabb_query {
        if let Some(coll) = aabb.ray_collision(&ray, false) {
            grounded = true;
            if velocity.velocity.y > 0. {
                velocity.velocity.y = 0.;
            };
            velocity_vec = glm::vec2(0., 0.);
            break;
        }
    }

    if keyboard_input.pressed(KeyCode::A) {
        velocity_vec -= glm::vec2(1., 0.);
    }
    if keyboard_input.pressed(KeyCode::D) {
        velocity_vec += glm::vec2(1., 0.);
    }
    if keyboard_input.pressed(KeyCode::R) {
        position.position = glm::vec2(200., 450.);
        velocity.velocity = glm::vec2(0., 0.);
    }
    if keyboard_input.just_pressed(KeyCode::Space) {
        if grounded {
            velocity.velocity.y = 0.;
            velocity.jump_pressed = true
        }
    }

    if velocity.jump_pressed {
        velocity_vec.y = -2.5;
    }
    velocity.wanted_velocity = velocity_vec * speed_mult + glm::vec2(0., velocity.velocity.y);
}

pub(crate) fn solve_position(
    mut last_time: Local<f64>,
    time: Res<Time>,
    mut velocity_position_query: Query<(&mut VelocityComp, &mut PositionComp, &mut LightComp)>,
) {
    let fixed_delta_time = time.seconds_since_startup() - *last_time;
    for (mut velocity, mut position, mut light) in velocity_position_query.iter_mut() {
        if velocity.jump_pressed
            || velocity.wanted_velocity.magnitude_squared() > 0.001 * 0.001
            || velocity.velocity.magnitude_squared() > 0.001 * 0.001
        {
            // light.polygon = None;
            light.has_moved = true;
            let mut wv = velocity.wanted_velocity;
            if velocity.jump_pressed {
                velocity.jump_pressed = false;
            }
            velocity.velocity = wv;
            position.position += velocity.velocity * fixed_delta_time as f32;
        }
    }
    *last_time = time.seconds_since_startup();
}
