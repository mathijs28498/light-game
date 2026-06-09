use nalgebra_glm as glm;

use bevy::prelude::*;

use bytemuck::{Pod, Zeroable};

use std::{f32::consts::PI, sync::Arc};

use vulkano::{
    buffer::{BufferContents, BufferUsage, ImmutableBuffer},
    device::Queue,
};

use bevy::ecs::{query::*, system::*};

use crate::{
    environment::{components::*, traits::*, traits_impl::*},
    general::{components::*, data_types::*, functions::*},
    rendering::functions::*, physics::data_types::Collision,
};

use super::shader_data_types::{CreatureVertex, BloomVertex};

#[derive(Debug, Component)]
pub struct LightComp {
    pub color: glm::Vec3,
    radius: f32,
    max_radius: f32,
    pub brightness: f32,
    pub polygon: Option<Vec<glm::Vec2>>,
    pub has_moved: bool,
    has_collided: bool,
}

#[derive(Debug, Component)]
pub struct CreatureComp {
    pub color: glm::Vec3,
}

#[derive(Component)]
pub struct BloomComp {

}

#[derive(Component, Clone)]
pub struct RenderObjectComp<T>
where
    T: Zeroable + Pod,
    [T]: BufferContents,
{
    pub(crate) vertex_buffer: Option<Arc<ImmutableBuffer<[T]>>>,
    pub(crate) index_buffer: Option<Arc<ImmutableBuffer<[u32]>>>,
}

impl LightComp {
    pub fn new(color: glm::Vec3, radius: f32, brightness: f32) -> Self {
        LightComp {
            color,
            radius,
            max_radius: radius,
            brightness,
            polygon: None,
            has_moved: false,
            has_collided: false,
        }
    }

    pub fn new_with_max_radius(
        color: glm::Vec3,
        center: glm::Vec2,
        radius: f32,
        max_radius: f32,
        brightness: f32,
    ) -> Self {
        LightComp {
            color,
            radius,
            max_radius,
            brightness,
            polygon: None,
            has_moved: false,
            has_collided: false,
        }
    }

    pub fn get_radius(&self) -> f32 {
        return self.radius;
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
        self.maybe_set_max_radius(radius);
    }

    pub fn maybe_set_max_radius(&mut self, max_radius: f32) {
        if self.max_radius > max_radius {
            return;
        }
        self.max_radius = max_radius;
        self.polygon = None;
    }

    fn get_collision_circle(&self, position: Option<glm::Vec2>) -> Circle {
        Circle {
            radius: self.max_radius * 1.1,
            center: position.unwrap_or(glm::vec2(0., 0.)),
        }
    }

    pub fn calculate_light_polygon(
        &mut self,
        position: &PositionComp,
        env_object_query: &Query<&AABBComp, With<EnvironmentObjectComp>>,
    ) -> (Vec<glm::Vec2>, bool) {
        // Collision circle which is slightly bigger than the light itself
        let collision_circle = self.get_collision_circle(Some(position.position.clone()));

        // TODO: Improve the readability!!
        // See if new polygon has to be calculated
        if let Some(polygon) = &self.polygon {
            if !self.has_moved {
                return (polygon.clone(), false);
            }

            if !self.has_collided {
                let mut has_collision = false;
                for env_obj in env_object_query {
                    if let Some(coll) = env_obj.circle_collision(&collision_circle) {
                        has_collision = true;
                        break;
                    }
                }
                if !has_collision {
                    return (polygon.clone(), false);
                }
            }
        }

        let mut points_to_add = collision_circle.get_segment_points(10, true);

        // Collide with environment objects
        let mut obstructing_objects = Vec::new();
        self.has_collided = false;
        for env_obj in env_object_query {
            if let Some(coll) = env_obj.circle_collision(&collision_circle) {
                self.has_collided = true;
                obstructing_objects.push(env_obj);

                for point in &coll.collision_points {
                    let mut point_ray =
                        Ray::new_between_points(collision_circle.center.clone(), point);
                    let angle_rays = point_ray.get_angle_rays(None);
                    for ray in angle_rays {
                        if let None = env_obj.ray_collision(&ray, true) {
                            points_to_add.push(ray.get_vector_with_t(collision_circle.radius));
                        } else {
                            points_to_add.push(ray.get_vector());
                        }
                    }
                }
            }
        }

        let mut actual_points = Vec::new();

        // Adding circle points that dont collide with object
        for point in points_to_add {
            let ray = Ray::new_between_points(collision_circle.center.clone(), &(point + collision_circle.center));
            if let Some(t) = Self::check_if_point_is_obstructed(&ray, &obstructing_objects) {
                actual_points.push(ray.get_vector_with_t(t));
            } else {
                actual_points.push(point)
            }
        }

        sort_points_clockwise(&mut actual_points, &glm::vec2(0., 0.));
        let polygon_c = actual_points.clone();
        self.polygon = Some(actual_points);
        self.has_moved = false;
        (polygon_c, true)
    }

    fn check_if_point_is_obstructed(
        ray: &Ray,
        env_objects: &Vec<&AABBComp>,
    ) -> Option<f32> {
        let mut add_circle_point = true;
        let mut min_t = f32::MAX;
        for env_obj in env_objects {
            if let Some(coll) = env_obj.ray_collision(&ray, true) {
                if coll.t < min_t{
                    min_t = coll.t;
                }
            }
        }
        if min_t == f32::MAX {
            None
        } else {
            Some(min_t)
        }
    }
}

impl<T> RenderObjectComp<T>
where
    T: Zeroable + Pod,
    [T]: BufferContents,
{
    pub fn new() -> Self {
        Self {
            vertex_buffer: None,
            index_buffer: None,
        }
    }

    pub fn set_buffers(&mut self, vertices: Vec<T>, indices: Vec<u32>, queue: Arc<Queue>) {
        let (index_buffer, ib_future) = create_index_buffer(indices, queue.clone());

        let (vertex_buffer, vb_future) =
            ImmutableBuffer::from_iter(vertices, BufferUsage::vertex_buffer(), queue).unwrap();

        // TODO: Await futures!!

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
    }

    pub fn update_vertex_buffer_light(&mut self, vertices: Vec<T>, queue: Arc<Queue>) {
        let (index_buffer, ib_future) = calculate_index_buffer_polygon(&queue, vertices.len());

        let (vertex_buffer, vb_future) =
            ImmutableBuffer::from_iter(vertices, BufferUsage::vertex_buffer(), queue).unwrap();

        // vb_future.
        // TODO: Await futures!!

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
    }
}

impl RenderObjectComp<CreatureVertex> {
    pub(crate) fn create_aabb(&mut self, width: f32, height: f32, queue: Arc<Queue>) {
        let half_width = width * 0.5;
        let half_height = height * 0.5;
        self.set_buffers(
            vec![
                CreatureVertex {
                    position: [-half_width, -half_height],
                },
                CreatureVertex {
                    position: [-half_width, half_height],
                },
                CreatureVertex {
                    position: [half_width, -half_height],
                },
                CreatureVertex {
                    position: [half_width, half_height],
                },
            ],
            vec![0, 1, 2, 2, 1, 3],
            queue,
        );
    }
}

impl RenderObjectComp<BloomVertex> {
    pub(crate) fn create_aabb(&mut self, width: f32, height: f32, queue: Arc<Queue>) {
        let half_width = width * 0.5;
        let half_height = height * 0.5;
        self.set_buffers(
            vec![
                BloomVertex {
                    position: [-half_width, -half_height],
                },
                BloomVertex {
                    position: [-half_width, half_height],
                },
                BloomVertex {
                    position: [half_width, -half_height],
                },
                BloomVertex {
                    position: [half_width, half_height],
                },
            ],
            vec![0, 1, 2, 2, 1, 3],
            queue,
        );
    }
}