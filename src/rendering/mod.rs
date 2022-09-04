pub(crate) mod components;
pub(crate) mod data_types;
pub(crate) mod functions;
pub(crate) mod shader_data_types;
pub(crate) mod system;
pub(crate) mod traits;
pub(crate) mod traits_impl;

use std::sync::Arc;

use nalgebra_glm as glm;

use rand::Rng;

use bevy_vulkano::{BevyVulkanoWindows, VulkanoWinitConfig};
use vulkano::device::{Features, Queue};
use vulkano_util::context::VulkanoConfig;

use bevy::{
    app::*,
    ecs::{
        schedule::*,
        system::{Commands, NonSend, ResMut},
    },
    prelude::*,
};

use crate::{
    environment::components::*,
    general::components::*,
    player::components::*,
    rendering::{components::*, data_types::*, functions::*, shader_data_types::*, system::*},
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub(crate) enum RenderStage {
    RenderStart,
    RenderLight,
    RenderCreature,
    RenderFinish,
}

pub(crate) struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraComp {
            position: glm::Vec2::new(0., 0.),
        })
        .add_startup_system(insert_render_pass_system)
        .add_startup_system(insert_initial_game_objects_system)
        .add_stage_after(
            CoreStage::PostUpdate,
            RenderStage::RenderStart,
            SystemStage::single_threaded(),
        )
        .add_stage_after(
            RenderStage::RenderStart,
            RenderStage::RenderLight,
            SystemStage::single_threaded(),
        )
        .add_stage_after(
            RenderStage::RenderLight,
            RenderStage::RenderCreature,
            SystemStage::single_threaded(),
        )
        .add_stage_after(
            RenderStage::RenderCreature,
            RenderStage::RenderFinish,
            SystemStage::single_threaded(),
        )
        // Render systems
        .add_system_set_to_stage(
            RenderStage::RenderStart,
            SystemSet::new().with_system(pre_render_setup_system),
        )
        .add_system_set_to_stage(
            RenderStage::RenderLight,
            SystemSet::new().with_system(light_render_system),
        )
        .add_system_set_to_stage(
            RenderStage::RenderCreature,
            SystemSet::new().with_system(creature_render_system),
        )
        .add_system_set_to_stage(
            RenderStage::RenderFinish,
            SystemSet::new().with_system(post_render_system),
        )
        .add_system(update_light_polygons_system)
        .add_system(insert_aabb_render_object_system)
        .add_system(regenerate_random_lights_system);
    }
}

#[derive(Component)]
pub struct RandomLightComp;

fn regenerate_random_lights_system(
    mut light_query: Query<(&mut PositionComp, &mut LightComp), With<RandomLightComp>>,
    keys: Res<Input<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::E) {
        let mut rng = rand::thread_rng();
        for (mut position, mut light) in light_query.iter_mut() {
            position.position =
                glm::Vec2::new(rng.gen_range(30.0..1250.0), rng.gen_range(30.0..690.0));
            light.has_moved = true;
        }
    }
    if keys.just_pressed(KeyCode::W) {
        for (mut position, _) in light_query.iter_mut() {
            position.position = glm::Vec2::new(-1000., -1000.);
        }
    }
}

fn insert_aabb_render_object_system(
    mut commands: Commands,
    env_objects: Query<
        (Entity, &AABBComp),
        (Without<RenderObjectComp<CreatureVertex>>, With<AABBComp>),
    >,
    vulkano_windows: NonSend<BevyVulkanoWindows>,
) {
    let window_renderer = vulkano_windows.get_primary_window_renderer().unwrap();
    let queue = window_renderer.graphics_queue();

    let mut rng = rand::thread_rng();
    let col_min = 0.;
    let col_max = 20.;

    for (entity, aabb) in env_objects.iter() {
        // println!("{:?}", entity.type_name());
        let mut render_object = RenderObjectComp::<CreatureVertex>::new();
        let size = aabb.max - aabb.min;
        render_object.create_aabb(size.x + 4., size.y + 4., queue.clone());

        commands
            .entity(entity)
            .insert(PositionComp {
                position: aabb.center.clone(),
            })
            .insert(render_object)
            .insert(CreatureComp {
                // color: glm::Vec3::new(
                //     rng.gen_range(col_min..col_max) - 10.,
                //     rng.gen_range(col_min..col_max) - 10.,
                //     rng.gen_range(col_min..col_max) - 10.,
                // ),
                color: glm::Vec3::new(10., 10., 10.),
            });
    }
}

// This fn is only used for test purposes
// TODO: Add scenes/terrain generation module
fn insert_initial_game_objects_system(
    mut commands: Commands,
    vulkano_windows: NonSend<BevyVulkanoWindows>,
) {
    let window_renderer = vulkano_windows.get_primary_window_renderer().unwrap();
    let queue = window_renderer.graphics_queue();

    let light_render_object = RenderObjectComp::<LightVertex>::new();
    let mut image_render_object = RenderObjectComp::<CreatureVertex>::new();
    image_render_object.create_aabb(40., 100., queue.clone());

    commands
        .spawn()
        .insert(PositionComp {
            position: glm::Vec2::new(200., 450.),
        })
        .insert(VelocityComp {
            velocity: glm::Vec2::new(0., 0.),
            wanted_velocity: glm::Vec2::new(0., 0.),
            jump_pressed: false,
        })
        .insert(LightComp::new(glm::Vec3::new(0.1, 0.45, 0.7), 150., 2.5))
        .insert(PlayerLightComp)
        // .insert(MouseLight)
        .insert(light_render_object)
        .insert(image_render_object)
        .insert(CreatureComp {
            color: glm::Vec3::new(0.1, 0.8, 0.4),
        });

    // generate_random_faces(&mut commands, &queue, 5);
    // generate_face(&mut commands, &queue, 200., 200.);
    // generate_face(&mut commands, &queue, 400., 400.);
    // generate_face(&mut commands, &queue, 500., 100.);
    // generate_face(&mut commands, &queue, 800., 500.);
    generate_random_lights(&mut commands, 0);
    generate_random_aabbs(&mut commands, 0);
    generate_env_objects(&mut commands);
}

pub(super) fn generate_env_objects(commands: &mut Commands) {
    commands
        .spawn()
        .insert(AABBComp::new(
            glm::Vec2::new(100., 300.),
            glm::Vec2::new(600., 330.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABBComp::new(
            glm::Vec2::new(100., 600.),
            glm::Vec2::new(500., 620.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABBComp::new(
            glm::Vec2::new(750., 600.),
            glm::Vec2::new(900., 640.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABBComp::new(
            glm::Vec2::new(850., 450.),
            glm::Vec2::new(900., 460.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABBComp::new(
            glm::Vec2::new(850., 300.),
            glm::Vec2::new(900., 310.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABBComp::new(
            glm::Vec2::new(850., 135.),
            glm::Vec2::new(900., 165.),
        ))
        .insert(EnvironmentObjectComp);
}

fn generate_random_faces(commands: &mut Commands, queue: &Arc<Queue>, amount: u32) {
    let mut rng = rand::thread_rng();
    for i in 0..amount {
        generate_face(
            commands,
            queue,
            rng.gen_range(80.0..1200.),
            rng.gen_range(80.0..660.),
        );
    }
}

pub(super) fn generate_face(
    commands: &mut Commands,
    queue: &Arc<Queue>,
    center_x: f32,
    eye_y: f32,
) {
    let eye_dist = 25.;
    let mouth_y = eye_y + 50.;
    {
        // Eyes

        let mut rng = rand::thread_rng();
        let eye_size = 20.;
        let pupil_size = rng.gen_range(4.0..9.);
        generate_creature(
            commands,
            queue.clone(),
            20.,
            glm::Vec2::new(center_x - eye_dist, eye_y),
            glm::Vec3::new(0.1, 0.3, 0.7),
        );

        generate_creature(
            commands,
            queue.clone(),
            pupil_size,
            glm::Vec2::new(center_x + eye_dist, eye_y),
            glm::Vec3::new(0., 0., 0.),
        );

        generate_creature(
            commands,
            queue.clone(),
            20.,
            glm::Vec2::new(center_x + eye_dist, eye_y),
            glm::Vec3::new(0.1, 0.3, 0.7),
        );

        generate_creature(
            commands,
            queue.clone(),
            pupil_size,
            glm::Vec2::new(center_x - eye_dist, eye_y),
            glm::Vec3::new(0., 0., 0.),
        );
    }

    {
        // Mouth
        let teeth_amount = 10;
        let teeth_size = 5.;
        let teeth_gap = 2.;
        let left = center_x
            - (teeth_amount as f32 - 1.) * teeth_size * 0.5
            - teeth_gap * (teeth_amount as f32 - 1.) * 0.5;
        for i in 0..teeth_amount {
            generate_creature(
                commands,
                queue.clone(),
                teeth_size,
                glm::Vec2::new(
                    left + (teeth_size + teeth_gap) * i as f32,
                    mouth_y - teeth_size - teeth_gap * 0.5,
                ),
                // glm::Vec3::new(2., 2., 2.),
                glm::Vec3::new(2., 0., 0.),
            );

            generate_creature(
                commands,
                queue.clone(),
                teeth_size,
                glm::Vec2::new(
                    left + (teeth_size + teeth_gap) * i as f32,
                    mouth_y + teeth_size + teeth_gap * 0.5,
                ),
                // glm::Vec3::new(2., 2., 2.),
                glm::Vec3::new(2., 0., 0.),
            );
        }
    }
}

pub(super) fn generate_creature(
    commands: &mut Commands,
    queue: Arc<Queue>,
    size: f32,
    position: glm::Vec2,
    color: glm::Vec3,
) {
    let mut image_render_object = RenderObjectComp::<CreatureVertex>::new();
    image_render_object.create_aabb(size, size, queue);
    commands
        .spawn()
        .insert(image_render_object)
        .insert(PositionComp { position })
        .insert(CreatureComp { color });
}

pub(super) fn generate_random_aabbs(commands: &mut Commands, amount_of_aabbs: usize) {
    let offset = 20.;
    let min_size = 30.;
    let max_size = 100.;

    let mut rng = rand::thread_rng();
    for i in 0..amount_of_aabbs {
        let min = glm::Vec2::new(
            rng.gen_range(offset..1280. - offset - max_size),
            rng.gen_range(offset..720. - offset - max_size),
        );
        let size = glm::Vec2::new(
            rng.gen_range(min_size..max_size),
            rng.gen_range(min_size..max_size),
        );
        commands
            .spawn()
            .insert(AABBComp::new(min, min + size))
            .insert(EnvironmentObjectComp);
    }
}

pub(super) fn generate_random_lights(commands: &mut Commands, amount_of_lights: usize) {
    let colors = vec![
        glm::Vec3::new(0.85, 0.33, 0.04),
        glm::Vec3::new(0.23, 0.85, 0.09),
        glm::Vec3::new(0.85, 0.06, 0.2),
        glm::Vec3::new(0.09, 0.7, 0.7),
        glm::Vec3::new(0.85, 0.06, 0.06),
    ];

    let positions = vec![
        glm::Vec2::new(101., 101.),
        glm::Vec2::new(351., 101.),
        glm::Vec2::new(701., 101.),
        glm::Vec2::new(101., 351.),
        glm::Vec2::new(351., 351.),
        glm::Vec2::new(701., 351.),
        glm::Vec2::new(101., 551.),
        glm::Vec2::new(351., 551.),
        glm::Vec2::new(701., 551.),
    ];

    let mut rng = rand::thread_rng();
    for i in 0..amount_of_lights {
        let color_offset = glm::Vec3::new(
            rng.gen_range(0.0..0.2) - 0.1,
            rng.gen_range(0.0..0.2) - 0.1,
            rng.gen_range(0.0..0.2) - 0.1,
        );
        let light = LightComp::new(
            colors[i % colors.len()] + color_offset,
            rng.gen_range(100.0..150.0),
            rng.gen_range(0.2..0.8),
        );
        let render_object = RenderObjectComp::<LightVertex>::new();
        commands
            .spawn()
            .insert(light)
            .insert(render_object)
            .insert(PositionComp {
                position: glm::Vec2::new(rng.gen_range(30.0..1250.0), rng.gen_range(30.0..690.0)),
            })
            .insert(RandomLightComp);
    }
}
