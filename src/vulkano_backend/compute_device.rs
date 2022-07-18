use std::{borrow::BorrowMut, cell::RefMut, sync::Arc};

use vulkano::{
    buffer::{BufferContents, BufferUsage, CpuAccessibleBuffer, ImmutableBuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage,
        PrimaryAutoCommandBuffer,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    instance::Instance,
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    sync::{self, FenceSignalFuture, GpuFuture, NowFuture},
};

use nalgebra_glm as glm;


pub const WIDTH: u32 = 480;
pub const HEIGHT: u32 = 480;
pub const BUFFER_SIZE: usize = (WIDTH * HEIGHT * 4) as usize;
pub const WORK_GROUP_SIZE: i32 = WIDTH as i32 * HEIGHT as i32 / 256;

pub struct PushConstants{
    time_passed: f32,
    width: u32, 
    height: u32,
    mouse_x: f32,
    mouse_y: f32,
}

impl PushConstants {
    pub fn new(time_passed: f32, width: u32, height: u32, mouse_x: f32, mouse_y: f32) -> Self {
        PushConstants { time_passed, width, height, mouse_x, mouse_y }
    }
}

pub struct ComputeDevice {
    device: Option<Arc<Device>>,
    queue: Option<Arc<Queue>>,
    pixel_buffer: Option<Arc<CpuAccessibleBuffer<[u32]>>>,
    command_buffer: Option<PrimaryAutoCommandBuffer>,
    pipeline: Option<Arc<ComputePipeline>>,
    set: Option<Arc<PersistentDescriptorSet>>,
    buffer_u32: Option<Vec<u32>>,
    future: Option<FenceSignalFuture<CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>>>,
}

impl ComputeDevice {
    pub fn new() -> Self {
        ComputeDevice {
            device: None,
            queue: None,
            pixel_buffer: None,
            command_buffer: None,
            pipeline: None,
            set: None,
            buffer_u32: None,
            future: None,
        }
    }

    pub fn init(&mut self) {
        let instance = Instance::new(Default::default()).unwrap();

        let device_extensions = DeviceExtensions {
            khr_storage_buffer_storage_class: true,
            ..DeviceExtensions::none()
        };
        let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
            .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
            .filter_map(|p| {
                p.queue_families()
                    .find(|&q| q.supports_compute())
                    .map(|q| (p, q))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
            })
            .unwrap();

        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type
        );

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: physical_device
                    .required_extensions()
                    .union(&device_extensions),
                queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
                ..Default::default()
            },
        )
        .unwrap();
        let queue = queues.next().unwrap();

        self.device = Some(device);
        self.queue = Some(queue);
    }

    // TODO: Look if the initialization can be improved of the buffer
    pub fn create_buffers(&mut self) {
        // TODO: Change to CpuBufferPool
        let pixel_buffer = {
            let data_iter = (0..(WIDTH * HEIGHT) as usize).map(|n| 0_u32);
            CpuAccessibleBuffer::from_iter(
                self.device.as_ref().unwrap().clone(),
                BufferUsage {
                    storage_buffer: true,
                    ..BufferUsage::none()
                },
                false,
                data_iter,
            )
            .unwrap()
        };

        self.pixel_buffer = Some(pixel_buffer);
    }

    pub fn create_pipeline(&mut self) {
        let pipeline = {
            mod cs {
                vulkano_shaders::shader! {
                    ty: "compute",
                    path: "src/shaders/basic_shader.comp"
                }
            }
            let shader = cs::load(self.device.as_ref().unwrap().clone()).unwrap();

            ComputePipeline::new(
                self.device.as_ref().unwrap().clone(),
                shader.entry_point("main").unwrap(),
                &(),
                None,
                |_| {},
            )
            .unwrap()
        };

        let layout = pipeline.layout().set_layouts().get(0).unwrap();

        let set = PersistentDescriptorSet::new(
            layout.clone(),
            [
                WriteDescriptorSet::buffer(0, self.pixel_buffer.as_ref().unwrap().clone()),
            ],
        )
        .unwrap();

        self.pipeline = Some(pipeline);
        self.set = Some(set);
    }

    pub fn execute(&mut self, push_constants: PushConstants) {
        let mut builder = AutoCommandBufferBuilder::primary(
            self.device.as_ref().unwrap().clone(),
            self.queue.as_ref().unwrap().family(),
            CommandBufferUsage::MultipleSubmit,
        )
        .unwrap();

        builder
            .bind_pipeline_compute(self.pipeline.as_ref().unwrap().clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                self.pipeline.as_ref().unwrap().layout().clone(),
                0,
                self.set.as_ref().unwrap().clone(),
            )
            .push_constants(
                self.pipeline.as_ref().unwrap().layout().clone(),
                0,
                push_constants,
            )
            .dispatch([WORK_GROUP_SIZE as u32, 1, 1])
            .unwrap();

        let command_buffer = builder.build().unwrap();

        let future = sync::now(self.device.as_ref().unwrap().clone())
            .then_execute(self.queue.as_ref().unwrap().clone(), command_buffer)
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();

        self.future = Some(future);
    }

    pub fn await_future(&mut self) -> bool {
        let future = match self.future.as_ref() {
            Some(f) => f,
            None => return false,
        };
        future.wait(None).unwrap();

        let data_buffer_content = self.pixel_buffer.as_ref().unwrap().read().unwrap();
        self.buffer_u32 = Some(data_buffer_content.to_owned());
        true
    }

    pub fn fill_u8(&self, buffer_to_fill: &mut [u8]) -> Result<(), &str> {
        let buffer = match self.buffer_u32.as_ref() {
            Some(b) => b,
            None => return Err("Command has never executed, so buffer is None"),
        };

        let mut last_index = 0;

        buffer_to_fill.clone_from_slice(buffer.as_bytes());

        Ok(())
    }
}
