#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unreachable_code)]

pub(crate) mod environment;
pub(crate) mod ext_traits;
pub(crate) mod general;
pub(crate) mod physics;
pub(crate) mod player;
pub(crate) mod rendering;

use bevy::app::App;

use crate::{
    environment::{components::*, *},
    general::{components::*, *},
    physics::*,
    player::{components::*, *},
    rendering::{components::*, data_types::*, shader_data_types::*, *},
};

fn main() {
    App::new().add_plugins(GeneralPluginBundle).run();
}
