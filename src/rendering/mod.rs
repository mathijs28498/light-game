pub(crate) mod components;
pub(crate) mod data_types;
pub(crate) mod functions;
pub(crate) mod shader_data_types;
pub(crate) mod system;

use nalgebra_glm as glm;

use rand::Rng;

use bevy_vulkano::VulkanoWinitConfig;
use vulkano::device::Features;
use vulkano_util::context::VulkanoConfig;

use bevy::{
    app::*,
    ecs::{schedule::*, system::Commands},
};

use crate::{
    environment::components::*,
    general::components::*,
    player::components::*,
    rendering::{components::*, functions::*, shader_data_types::*, system::*},
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub(crate) enum RenderStage {
    GuiInit,
    GuiDefine,
    RenderStart,
    Render,
    RenderFinish,
}

pub(crate) struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(insert_render_pass_system)
            .add_startup_system(insert_initial_game_objects_system)
            .add_stage_after(
                CoreStage::PostUpdate,
                RenderStage::GuiInit,
                SystemStage::single_threaded(),
            )
            .add_stage_after(
                RenderStage::GuiInit,
                RenderStage::GuiDefine,
                SystemStage::parallel(),
            )
            .add_stage_after(
                RenderStage::GuiDefine,
                RenderStage::RenderStart,
                SystemStage::single_threaded(),
            )
            .add_stage_after(
                RenderStage::RenderStart,
                RenderStage::Render,
                SystemStage::single_threaded(),
            )
            .add_stage_after(
                RenderStage::Render,
                RenderStage::RenderFinish,
                SystemStage::single_threaded(),
            )
            // Render systems
            .add_system_set_to_stage(
                RenderStage::RenderStart,
                SystemSet::new().with_system(pre_render_setup_system),
            )
            .add_system_set_to_stage(
                RenderStage::Render,
                SystemSet::new().with_system(main_render_system),
            )
            .add_system_set_to_stage(
                RenderStage::RenderFinish,
                SystemSet::new().with_system(post_render_system),
            )
            .add_system(update_light_polygons_system);
    }
}

// This fn is only used for test purposes
// TODO: Add scenes/terrain generation module
fn insert_initial_game_objects_system(mut commands: Commands) {
    let render_object = RenderObject::<LightVertex>::new();

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
        .insert(LightComp::new(glm::Vec3::new(0.1, 0.45, 0.7), 200., 1.5))
        .insert(PlayerLightComp)
        // .insert(MouseLight)
        .insert(render_object);

    // generate_random_lights(&mut commands, 1000);
    generate_random_aabbs(&mut commands, 0);

    commands
        .spawn()
        .insert(AABBComp::new(
            glm::Vec2::new(100., 530.),
            glm::Vec2::new(300., 550.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABBComp::new(
            glm::Vec2::new(100., 320.),
            glm::Vec2::new(300., 330.),
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
            rng.gen_range(100.0..300.0),
            rng.gen_range(0.2..0.8),
        );
        let render_object = RenderObject::<LightVertex>::new();
        commands
            .spawn()
            .insert(light)
            .insert(render_object)
            .insert(PositionComp {
                position: glm::Vec2::new(rng.gen_range(30.0..1250.0), rng.gen_range(30.0..690.0)),
            });
    }
}
