use nannou::{
    glam::Vec2,
    rand::{random_f32, random_range},
};
use std::f32::consts::PI;

use crate::framework::prelude::*;

// https://github.com/Lokua/p5/blob/main/src/sketches/drop.mjs
// https://github.com/Lokua/p5/blob/main/src/sketches/drop3.mjs

pub struct Drop {
    center: Vec2,
    resolution: usize,
    radius: f32,
    vertices: Vec<Vec2>,
}

#[allow(dead_code)]
impl Drop {
    pub fn new(center: Vec2, radius: f32, resolution: usize) -> Self {
        let mut drop = Drop {
            center,
            resolution,
            radius,
            vertices: Vec::new(),
        };
        drop.vertices = drop.create_vertices();
        drop
    }

    pub fn update(&mut self, center: Vec2, radius: f32, resolution: usize) {
        self.center = center;
        self.radius = radius;
        self.resolution = resolution;
        self.vertices = self.create_vertices();
    }

    fn create_vertices(&self) -> Vec<Vec2> {
        (0..self.resolution)
            .map(|i| {
                let angle = (i as f32 / self.resolution as f32) * 2.0 * PI;
                Vec2::new(angle.cos(), angle.sin()) * self.radius + self.center
            })
            .collect()
    }

    // https://people.csail.mit.edu/jaffer/Marbling/Dropping-Paint
    // C + (P − C) * sqrt (1 + r^2 / ||P − C||^2)
    pub fn marble(&self, other: &mut Drop) {
        for v in &mut other.vertices {
            // (P - C)
            let center_to_point = *v - self.center;

            // ||P − C||^2
            let mag_sq = center_to_point.length_squared();

            // sqrt(1 + r^2 / ||P − C||^2)
            let scale = if mag_sq == 0.0 {
                1.0
            } else {
                (1.0 + self.radius.powi(2) / mag_sq).sqrt()
            };

            // NewP = C + scaled (P − C)
            *v = self.center + center_to_point * scale;
        }
    }

    // https://people.csail.mit.edu/jaffer/Marbling/Mathematics
    // Fv(x, y) = (x, y + z*u|x−xL|)
    // u = 1/2^1/c
    pub fn tine_vertical_only(
        &mut self,
        line_x: f32,
        displacement: f32,
        falloff: f32,
    ) {
        let falloff_factor = 1.0 / 2.0f32.powf(1.0 / falloff);

        for vertex in &mut self.vertices {
            let distance_from_center_line = (vertex.x - line_x).abs();
            let displacement_magnitude =
                displacement * falloff_factor.powf(distance_from_center_line);
            *vertex = Vec2::new(vertex.x, vertex.y + displacement_magnitude);
        }
    }

    // P = P + z + u^d * m
    // d = (P - B) dot N
    pub fn tine(
        &mut self,
        start: Vec2,
        direction: Vec2,
        displacement_magnitude: f32,
        falloff_control: f32,
    ) {
        let falloff_factor = 1.0 / 2.0f32.powf(1.0 / falloff_control);
        // Perpendicular to direction
        let normal = Vec2::new(-direction.y, direction.x);

        for vertex in &mut self.vertices {
            // Vector from vertex to line base
            let to_base = *vertex - start;
            // Projection of toBase onto normal
            let distance = to_base.dot(normal).abs();
            let scaled_displacement =
                displacement_magnitude * falloff_factor.powf(distance);
            *vertex += direction * scaled_displacement;
        }
    }

    pub fn vertices(&self) -> &Vec<Vec2> {
        &self.vertices
    }
}

pub struct DropZone {
    pub center: Vec2,
}

#[allow(dead_code)]
impl DropZone {
    pub fn new(center: Vec2) -> Self {
        Self { center }
    }

    pub fn point_within_circular_zone(
        &self,
        inner_radius: f32,
        outer_radius: f32,
    ) -> Vec2 {
        let angle = random_f32() * TWO_PI;
        let radius = f32::sqrt(random_range(
            inner_radius * inner_radius,
            outer_radius * outer_radius,
        ));
        Vec2::new(
            self.center.x + radius * angle.cos(),
            self.center.y + radius * angle.sin(),
        )
    }

    pub fn point_within_rectangular_zone_advanced(
        &self,
        inner_radius: f32,
        outer_radius: f32,
        x_min: f32,
        x_max: f32,
        y_min: f32,
        y_max: f32,
    ) -> Vec2 {
        let random = || {
            (
                self.center.x + random_range(-x_min, x_max),
                self.center.y + random_range(-y_min, y_max),
            )
        };
        let mut point = Vec2::from(random());
        while !self.is_in_rectangular_zone(point, inner_radius, outer_radius) {
            point = Vec2::from(random());
        }
        point
    }

    pub fn point_within_rectangular_zone(
        &self,
        inner_radius: f32,
        outer_radius: f32,
    ) -> Vec2 {
        self.point_within_rectangular_zone_advanced(
            inner_radius,
            outer_radius,
            outer_radius,
            outer_radius,
            outer_radius,
            outer_radius,
        )
    }

    pub fn point_within_rectangular_zone_top_bottom(
        &self,
        inner_radius: f32,
        outer_radius: f32,
    ) -> Vec2 {
        self.point_within_rectangular_zone_advanced(
            inner_radius,
            outer_radius,
            inner_radius,
            inner_radius,
            outer_radius,
            outer_radius,
        )
    }

    pub fn point_within_rectangular_zone_top(
        &self,
        inner_radius: f32,
        outer_radius: f32,
    ) -> Vec2 {
        self.point_within_rectangular_zone_advanced(
            inner_radius,
            outer_radius,
            inner_radius,
            inner_radius,
            inner_radius,
            outer_radius,
        )
    }
    pub fn point_within_rectangular_zone_right(
        &self,
        inner_radius: f32,
        outer_radius: f32,
    ) -> Vec2 {
        self.point_within_rectangular_zone_advanced(
            inner_radius,
            outer_radius,
            inner_radius,
            outer_radius,
            inner_radius,
            inner_radius,
        )
    }
    pub fn point_within_rectangular_zone_bottom(
        &self,
        inner_radius: f32,
        outer_radius: f32,
    ) -> Vec2 {
        self.point_within_rectangular_zone_advanced(
            inner_radius,
            outer_radius,
            inner_radius,
            inner_radius,
            outer_radius,
            inner_radius,
        )
    }
    pub fn point_within_rectangular_zone_left(
        &self,
        inner_radius: f32,
        outer_radius: f32,
    ) -> Vec2 {
        self.point_within_rectangular_zone_advanced(
            inner_radius,
            outer_radius,
            outer_radius,
            inner_radius,
            inner_radius,
            inner_radius,
        )
    }

    pub fn is_in_rectangular_zone(
        &self,
        point: Vec2,
        inner_radius: f32,
        outer_radius: f32,
    ) -> bool {
        let delta = (point - self.center).abs();
        let max_dist = delta.x.max(delta.y);
        max_dist >= inner_radius && max_dist <= outer_radius
    }
}
