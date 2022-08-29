use nalgebra_glm as glm;

use rand::Rng;

use std::sync::Arc;

use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    command_buffer::{CommandBufferExecFuture, PrimaryAutoCommandBuffer},
    device::Queue,
    sync::NowFuture,
};

use bevy::ecs::system::Commands;

use crate::{
    environment_objects::components::*,
    general::components::*,
    rendering::{components::*, shader_data_types::*},
};

pub(crate) fn calculate_indices_polygon(triangle_amount: usize) -> Vec<u32> {
    let mut indices = Vec::with_capacity(triangle_amount * 3);
    for i in 0..triangle_amount {
        let index = (i + 1) as u32;
        indices.push(0);
        indices.push(index);
        indices.push(index % triangle_amount as u32 + 1);
    }

    indices
}

pub(super) fn calculate_index_buffer_polygon(
    queue: &Arc<Queue>,
    amount_of_vertices: usize,
) -> (
    Arc<ImmutableBuffer<[u32]>>,
    CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>,
) {
    let indices = calculate_indices_polygon(amount_of_vertices - 1);
    ImmutableBuffer::from_iter(indices, BufferUsage::index_buffer(), queue.clone()).unwrap()
}
