// TODO: Implement 2d raytracing
// TODO: Draw using vulkano with a texture in a quad
//          This will make ui libraries work
// TODO: Make image buffer gpu only

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unreachable_code)]

use bevy::{
    app::AppExit,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::mouse::MouseMotion,
    prelude::*,
    window::WindowResizeConstraints,
};
use nalgebra::*;
use rand::prelude::*;

mod vulkano_backend;
use vulkano::pipeline::compute;
use vulkano_backend::{
    compute_device::{self}, //ComputeDevice, PushConstants, BUFFER_SIZE, HEIGHT, WIDTH,},
    vulkano_device::{self, VulkanoDevice},
    test_multi_render_passes::multi_main
};

mod game_object;

use rand::Rng;
use std::sync::Arc;

fn main() {
    let mut v_device = VulkanoDevice::new_with_initialization();
    v_device.run();
}
