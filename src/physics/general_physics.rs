use bevy::app::App;
use bevy::app::Plugin;
use bevy::ecs::system::Res;
use bevy::input::{keyboard::KeyCode, Input};
use bevy::prelude::*;
use bevy::time::{FixedTimestep, FixedTimesteps};

use nalgebra_glm as glm;

use crate::game_object::game_object::*;

use crate::bevy_render_plugin::main_render_plugin::*;

pub struct PhysicsPlugin;
const LABEL: &str = "my_fixed_timestep";

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum PhysicsStage {
    FixedUpdate,
    SolveVelocity,
}

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(player_input_system)
            .add_stage_after(
                RenderStage::RenderFinish,
                PhysicsStage::FixedUpdate,
                SystemStage::parallel(),
            )
            .add_stage_after(
                PhysicsStage::FixedUpdate,
                PhysicsStage::SolveVelocity,
                SystemStage::parallel()
                    .with_run_criteria(FixedTimestep::step(1. / 30.).with_label(LABEL))
                    .with_system(solve_position),
            );
    }
}

fn player_input_system(
    mut player_query: Query<(&mut Velocity, &mut Position), With<PlayerLight>>,
    aabb_query: Query<&AABB, With<EnvironmentObjectComp>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    // return;
    let speed_mult = 300.;
    let (mut velocity, mut position) = player_query.single_mut();
    let mut velocity_vec = glm::Vec2::new(0., 0.);
    velocity_vec += glm::Vec2::new(0., 0.05);

    let ray = Ray::new(position.position.clone(), velocity_vec.normalize(), 50.);
    let mut grounded = false;

    for aabb in &aabb_query {
        if let Some(coll) = aabb.ray_collision(&ray, false) {
            grounded = true;
            if velocity.velocity.y > 0. {
                velocity.velocity.y = 0.;
            };
            velocity_vec = glm::Vec2::new(0., 0.);
            break;
        }
    }

    if keyboard_input.pressed(KeyCode::A) {
        velocity_vec -= glm::Vec2::new(1., 0.);
    }
    if keyboard_input.pressed(KeyCode::D) {
        velocity_vec += glm::Vec2::new(1., 0.);
    } 
    if keyboard_input.pressed(KeyCode::R) {
        position.position = glm::Vec2::new(200., 450.);
        velocity.velocity = glm::Vec2::new(0., 0.);
    } 
    if keyboard_input.just_pressed(KeyCode::Space) {
        if grounded {
            velocity.velocity.y = 0.;
            velocity.jump_pressed = true
        }
    }

    velocity.wanted_velocity = velocity_vec * speed_mult + glm::Vec2::new(0., velocity.velocity.y);
}

fn solve_position(mut last_time: Local<f64>, time: Res<Time>, mut velocity_position_query: Query<(&mut Velocity, &mut Position, &mut Light)>) {
    let fixed_delta_time = time.seconds_since_startup() - *last_time;
    for (mut velocity, mut position, mut light) in velocity_position_query.iter_mut() {
        if velocity.jump_pressed || velocity.wanted_velocity.magnitude_squared() > 0.001 * 0.001 || velocity.velocity.magnitude_squared() > 0.001 * 0.001{
            // light.polygon = None;
            light.has_moved = true;
            let mut wv = velocity.wanted_velocity;
            if velocity.jump_pressed {
                wv -= glm::Vec2::new(0., 1. * 300.);
                velocity.jump_pressed = false;
            }
            velocity.velocity = wv;
            position.position += velocity.velocity * fixed_delta_time as f32;
        }
    }
    *last_time = time.seconds_since_startup(); 
}
