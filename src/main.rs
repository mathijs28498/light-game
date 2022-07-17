// TODO: Implement 2d raytracing
// TODO: Draw using vulkano with a texture in a quad
//          This will make ui libraries work
// TODO: Make image buffer gpu only

#![allow(unused_imports)]

use bevy::{app::AppExit, prelude::*, window::WindowResizeConstraints};
use bevy_pixels::prelude::*;
use nalgebra::*;
use rand::prelude::*;

mod vulkano_backend;
use vulkano::pipeline::compute;
use vulkano_backend::compute_device::{WIDTH, HEIGHT, BUFFER_SIZE, self, ComputeDevice, PushConstants};

use rand::Rng;


// #[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
// enum AppStage {
//     DrawBackground,
//     DrawObjects,
// }

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Hello Bevy Pixels".to_string(),
            width: WIDTH as f32,
            height: HEIGHT as f32,
            resize_constraints: WindowResizeConstraints {
                min_width: WIDTH as f32,
                min_height: HEIGHT as f32,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert_resource(
            compute_device::ComputeDevice::new()
        )
        // .insert_resource(GameObjects {
        //     balls: vec![
        //         Ball::new(Vector3::new(200., 200., 10.), 50., [255, 0, 0, 255]),
        //         Ball::new(Vector3::new(150., 150., 5.), 50., [0, 255, 0, 255]),
        //     ],
        // })
        .insert_resource(PixelsOptions {
            width: WIDTH,
            height: HEIGHT,
        })
        .add_startup_system(init_compute_device_system)
        .add_plugins(DefaultPlugins)
        .add_plugin(PixelsPlugin)
        .add_system(exit_on_escape_system)
        // .add_stage_after(
        //     PixelsStage::Draw,
        //     AppStage::DrawBackground,
        //     SystemStage::parallel(),
        // )
        // .add_stage_after(
        //     AppStage::DrawBackground,
        //     AppStage::DrawObjects,
        //     SystemStage::parallel(),
        // )
        .add_system(draw_objects_system)
        // .add_system_to_stage(AppStage::DrawBackground, draw_background_system)
        // .add_system_to_stage(AppStage::DrawObjects, draw_objects_system)
        .run();
}

fn exit_on_escape_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        app_exit_events.send(AppExit);
    }
}

fn init_compute_device_system(
    mut compute_device: ResMut<ComputeDevice>
) {
    compute_device.init();
    compute_device.create_buffers();
    compute_device.create_pipeline();
}

fn draw_objects_system(
    mut pixels_resource: ResMut<PixelsResource>,
    mut compute_device: ResMut<ComputeDevice>
    // game_objects: Res<GameObjects>,
) {


    let frame: &mut[u8] = pixels_resource.pixels.get_frame();
    compute_device.execute(
        PushConstants::new(
            0.,
            WIDTH,
            HEIGHT
        )
    );
    compute_device.await_future();
    compute_device.fill_u8(frame).unwrap();
    // *frame = frame_buffer.clone().try_into();
    // for (i, pix) in frame_buffer.iter().enumerate() {
    //     frame[i] = *pix;
    // }
}

fn get_sq_dist(v0: &Vector3<f32>, v1: &Vector3<f32>) -> f32 {
    let x = v0 - v1;
    return x.x.powi(2) + x.y.powi(2) + x.z.powi(2);
}
