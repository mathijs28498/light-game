use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    input::mouse::{self, MouseMotion},
    prelude::*,
    render::view::window,
    window::WindowId,
};
use bevy_vulkano::{BevyVulkanoWindows, PipelineSyncData};
use vulkano::{image::ImageAccess, sync::GpuFuture};

use crate::{
    game_object::game_object::*,
    vulkano_backend::vulkano_device::{RenderObject, SimpleVertex, VertexTest, VulkanoDevice},
    MousePosition,
};

use rand::Rng;
use std::sync::Arc;

use vulkano::device::Queue;

use nalgebra_glm as glm;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum RenderStage {
    GuiInit,
    GuiDefine,
    RenderStart,
    Render,
    RenderFinish,
}

pub struct MainRenderPlugin;

impl Plugin for MainRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(insert_render_pass_system)
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
            );
    }
}

fn insert_render_pass_system(mut commands: Commands, vulkano_windows: NonSend<BevyVulkanoWindows>) {
    let window_renderer = vulkano_windows.get_primary_window_renderer().unwrap();
    let queue = window_renderer.graphics_queue();
    let format = window_renderer.swapchain_format();

    let render_object = RenderObject::<SimpleVertex>::new(queue.clone());
    let vulkano_device = VulkanoDevice::new::<SimpleVertex>(queue, format);
    commands.insert_resource(vulkano_device);

    commands
        .spawn()
        .insert(Position {
            position: glm::Vec2::new(200., 450.),
        })
        .insert(Velocity {
            velocity: glm::Vec2::new(0., 0.),
            wanted_velocity: glm::Vec2::new(0., 0.),
            jump_pressed: false,
        })
        .insert(Light::new(glm::Vec3::new(0.1, 0.45, 0.7), 200., 1.5))
        .insert(PlayerLight)
        // .insert(MouseLight)
        .insert(render_object);

    // generate_random_lights(&mut commands, &queue, 1000);
    generate_random_aabbs(&mut commands, 0);

    commands
        .spawn()
        .insert(AABB::new(
            glm::Vec2::new(100., 530.),
            glm::Vec2::new(300., 550.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABB::new(
            glm::Vec2::new(100., 320.),
            glm::Vec2::new(300., 330.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABB::new(
            glm::Vec2::new(750., 600.),
            glm::Vec2::new(900., 640.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABB::new(
            glm::Vec2::new(850., 450.),
            glm::Vec2::new(900., 460.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABB::new(
            glm::Vec2::new(850., 300.),
            glm::Vec2::new(900., 310.),
        ))
        .insert(EnvironmentObjectComp);

    commands
        .spawn()
        .insert(AABB::new(
            glm::Vec2::new(850., 135.),
            glm::Vec2::new(900., 165.),
        ))
        .insert(EnvironmentObjectComp);
}

fn pre_render_setup_system(
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
    mut pipeline_frame_data: ResMut<PipelineSyncData>,
) {
    for (window_id, mut frame_data) in pipeline_frame_data.data_per_window.iter_mut() {
        let window_renderer =
            if let Some(window_renderer) = vulkano_windows.get_window_renderer_mut(*window_id) {
                window_renderer
            } else {
                return;
            };
        let before = match window_renderer.acquire() {
            Err(e) => {
                bevy::log::error!("Failed to start frame: {}", e);
                None
            }
            Ok(f) => Some(f),
        };
        frame_data.before = before;
    }
}

fn post_render_system(
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
    mut pipeline_frame_data: ResMut<PipelineSyncData>,
) {
    for (window_id, frame_data) in pipeline_frame_data.data_per_window.iter_mut() {
        let window_renderer =
            if let Some(window_renderer) = vulkano_windows.get_window_renderer_mut(*window_id) {
                window_renderer
            } else {
                return;
            };
        if let Some(after) = frame_data.after.take() {
            window_renderer.present(after, false);
        }
    }
}

pub fn main_render_system(
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
    diagnostics: Res<Diagnostics>,
    mut pipeline_frame_data: ResMut<PipelineSyncData>,
    mut vulkano_device: ResMut<VulkanoDevice>,
    light_query: Query<(&RenderObject<SimpleVertex>, &Position, &Light)>,
    env_object_query: Query<(&EnvironmentObjectComp, &AABB)>,
    mouse_position: Res<MousePosition>,
) {
    if let Some(diag) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(avg) = diag.average() {
            let primary = vulkano_windows
                .get_winit_window(WindowId::primary())
                .unwrap();
            primary.set_title(&format!(
                "Bevy Vulkano Game Of Life {:.2} fps ({:.2} ms/frame)",
                avg,
                1. / avg * 1000.
            ));
        }
    }

    let mut frame_data = pipeline_frame_data.get_mut(WindowId::primary()).unwrap();
    let window_renderer =
        if let Some(window_renderer) = vulkano_windows.get_primary_window_renderer_mut() {
            window_renderer
        } else {
            return;
        };

    // We take the before pipeline future leaving None in its place
    if let Some(before_future) = frame_data.before.take() {
        let final_image_view = window_renderer.swapchain_image_view();
        let mut after_future: Box<dyn GpuFuture> = vulkano_device.do_pass(
            before_future,
            window_renderer.swapchain_image_view(),
            light_query,
            &mouse_position,
        );

        let after_drawing = after_future.then_signal_fence_and_flush().unwrap().boxed();
        // Update after pipeline future (so post render will know to present frame)
        frame_data.after = Some(after_drawing);
    }
}

fn generate_random_aabbs(commands: &mut Commands, amount_of_aabbs: usize) {
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
            .insert(AABB::new(min, min + size))
            .insert(EnvironmentObjectComp);
    }
}

fn generate_random_lights(commands: &mut Commands, queue: &Arc<Queue>, amount_of_lights: usize) {
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
        let light = Light::new(
            colors[i % colors.len()] + color_offset,
            rng.gen_range(100.0..300.0),
            rng.gen_range(0.2..0.8),
        );
        let render_object = RenderObject::<SimpleVertex>::new(queue.clone());
        commands
            .spawn()
            .insert(light)
            .insert(render_object)
            .insert(Position {
                position: glm::Vec2::new(rng.gen_range(30.0..1250.0), rng.gen_range(30.0..690.0)),
            });
    }
}
