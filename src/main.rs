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

use vulkano::{device::Features, pipeline::compute};
use vulkano_util::{context::VulkanoConfig, window::VulkanoWindows};

use nalgebra_glm as glm;

use std::sync::Arc;

use bevy_vulkano::{VulkanoWinitConfig, VulkanoWinitPlugin};

pub(crate) mod environment;
pub(crate) mod general;
pub(crate) mod ext_traits;
pub(crate) mod physics;
pub(crate) mod player;
pub(crate) mod rendering;

use crate::{
    environment::{components::*, *},
    general::{components::*, *},
    physics::*,
    player::{components::*, *},
    rendering::{components::*, data_types::*, shader_data_types::*, *},
};

fn main() {
    App::new()
        .add_plugins(GeneralPluginBundle)
        .run();
}

