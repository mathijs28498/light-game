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
    rendering::functions::*,
};

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

#[derive(Component)]
pub struct RenderObject<T>
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

    pub fn calculate_light_polygon(
        &mut self,
        position: &PositionComp,
        env_object_query: &Query<&AABBComp, With<EnvironmentObjectComp>>,
    ) -> (Vec<glm::Vec2>, bool) {
        let collision_circle = Circle {
            radius: self.max_radius * 1.05,
            center: position.position.clone(),
        };
        // TODO: Fix jitter

        // TODO: Improve this!!
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

        let mut circle_points = Vec::new();

        // Draw circle
        let amount_of_points = 10;
        let angle_diff = PI * 2. / amount_of_points as f32;
        for i in 0..amount_of_points {
            let mut angle = angle_diff * i as f32;
            circle_points.push(glm::Vec2::new(angle.cos(), angle.sin()) * collision_circle.radius);
        }

        let mut env_objects = Vec::new();
        let mut actual_points = Vec::new();

        // Collide with environment objects
        self.has_collided = false;
        for env_obj in env_object_query {
            if let Some(coll) = env_obj.circle_collision(&collision_circle) {
                self.has_collided = true;
                env_objects.push(env_obj);

                for cp in &coll.collision_points {
                    // TODO: Check among each other collided env_object
                    let p_ray =
                        Ray::new_between_points(collision_circle.center.clone(), cp, Some(1.));
                    let angle_rays = p_ray.get_angle_rays(None);
                    for r in angle_rays {
                        if let None = env_obj.ray_collision(&r, true) {
                            actual_points.push(r.dir * collision_circle.radius);
                        } else {
                            actual_points.push(cp - collision_circle.center);
                        }
                    }
                }
            }
        }

        // Adding circle points that dont collide with object
        for cp in circle_points {
            let mut add_circle_point = true;
            let ray =
                Ray::new_between_points(position.position.clone(), &(cp + position.position), None);
            for env_obj in &env_objects {
                if let Some(_) = env_obj.ray_collision(&ray, false) {
                    add_circle_point = false;
                    break;
                }
            }
            if add_circle_point {
                actual_points.push(cp);
            }
        }

        sort_points_clockwise(&mut actual_points, &glm::Vec2::new(0., 0.));
        let polygon_c = actual_points.clone();
        self.polygon = Some(actual_points);
        self.has_moved = false;
        (polygon_c, true)
    }
}

impl<T> RenderObject<T>
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

    pub fn update_vertex_buffer(&mut self, vertices: Vec<T>, queue: Arc<Queue>) {
        let (index_buffer, ib_future) = calculate_index_buffer_polygon(&queue, vertices.len());

        let (vertex_buffer, vb_future) =
            ImmutableBuffer::from_iter(vertices, BufferUsage::vertex_buffer(), queue).unwrap();

        // vb_future.
        // TODO: Await futures!!

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
    }
}
