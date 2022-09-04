use bevy::{
    app::{AppExit, PluginGroupBuilder},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::{
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
        ButtonState,
    },
    prelude::*,
    window::{close_on_esc, CursorMoved, PresentMode, WindowMode, WindowResizeConstraints},
};

use nalgebra::*;
use rand::prelude::*;
use rand::Rng;

use bevy_vulkano::BevyVulkanoWindows;

use vulkano::{device::Features, pipeline::compute, image::ImageAccess};
use vulkano_util::{context::VulkanoConfig, window::VulkanoWindows};

use nalgebra_glm as glm;

use std::sync::Arc;

use bevy_vulkano::{VulkanoWinitConfig, VulkanoWinitPlugin};

use crate::{
    environment::components::*,
    general::{components::*, data_types::*},
    player::components::*,
    rendering::{components::*, data_types::*, shader_data_types::*},
};

// TODO: Make system on startup for renderobject that gets its queue
pub(super) fn shoot_light_system(
    mut commands: Commands,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mouse_position: Res<MousePosition>,
    mut camera: ResMut<CameraComp>,
    player_query: Query<(&PositionComp, &LightComp), With<PlayerLightComp>>,
) {
    let mp = mouse_position.as_ref();

    let colors = vec![
        glm::Vec3::new(0.85, 0.33, 0.04),
        glm::Vec3::new(0.23, 0.85, 0.09),
        glm::Vec3::new(0.85, 0.06, 0.2),
        glm::Vec3::new(0.09, 0.7, 0.7),
        glm::Vec3::new(0.85, 0.06, 0.06),
    ];
    let mut rng = rand::thread_rng();

    for event in mouse_button_input_events.iter() {
        if event.state == ButtonState::Pressed && event.button == MouseButton::Left {
            let (player_pos, light) = match player_query.get_single() {
                Ok(player) => player,
                Err(_) => return,
            };

            let dir = (mp.position + camera.position - player_pos.position).normalize();
            let light_pos = player_pos.position + dir * 50.;

            commands
                .spawn()
                .insert(PositionComp {
                    position: light_pos.clone(),
                })
                .insert(EnvironmentObjectComp)
                .insert(LightComp::new(
                    colors[rng.gen_range(0..5)]
                        + glm::Vec3::new(
                            rng.gen_range(0.0..0.2) - 0.1,
                            rng.gen_range(0.0..0.2) - 0.1,
                            rng.gen_range(0.0..0.2) - 0.1,
                        ),
                    100.,
                    3.,
                ))
                .insert(RenderObjectComp::<LightVertex>::new())
                .insert(VelocityComp {
                    velocity: dir * 350.,
                    wanted_velocity: dir * 350.,
                    jump_pressed: false,
                });
        }
    }
}

pub(super) fn move_camera_system(
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
    mut camera: ResMut<CameraComp>,
    player_query: Query<&PositionComp, With<PlayerLightComp>>,
) {
    let player_pos = match player_query.get_single() {
        Ok(player) => player,
        Err(_) => return,
    };
    let window_renderer = match vulkano_windows.get_primary_window_renderer_mut() {
        Some(window_renderer) => window_renderer,
        None => return,
    };

    // let t = window_renderer.swapchain_image_view();
    // let t2 = t.image().
    let dims = window_renderer.swapchain_image_view().image().dimensions().width_height();
    let camera_offset = glm::Vec2::new(dims[0] as f32, dims[1] as f32) * 0.5;

    camera.position = player_pos.position - camera_offset;
}
