pub(crate) mod data_types;
pub(crate) mod system;

use bevy::{
    app::{App, Plugin},
    ecs::schedule::SystemStage,
    prelude::*,
    time::FixedTimestep,
};

use crate::{physics::system::*, rendering::*};

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
                    // .with_run_criteria(FixedTimestep::step(1. / 30.).with_label(LABEL))
                    .with_system(solve_position),
            );
    }
}
