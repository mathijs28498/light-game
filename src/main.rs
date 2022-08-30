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

use crate::general::GeneralPluginBundle;

fn main() {
    App::new().add_plugins(GeneralPluginBundle).run();
}
