use nannou::prelude::*;
use std::fmt::{Debug, Formatter, Result};
use std::sync::Arc;

pub type CustomDistanceFn =
    Option<Arc<dyn Fn(Vec2, Vec2) -> f32 + Send + Sync>>;

pub struct Displacer {
    pub position: Vec2,
    pub radius: f32,
    pub strength: f32,
    pub custom_distance_fn: CustomDistanceFn,
}

impl Displacer {
    pub fn new(
        position: Vec2,
        radius: f32,
        strength: f32,
        custom_distance_fn: CustomDistanceFn,
    ) -> Self {
        Self {
            position,
            radius,
            strength,
            custom_distance_fn,
        }
    }

    pub fn new_with_position(position: Vec2) -> Self {
        Self::new(position, 50.0, 10.0, None)
    }

    pub fn new_at_origin() -> Self {
        Self::new_with_position(vec2(0.0, 0.0))
    }

    pub fn influence(&self, grid_point: Vec2) -> Vec2 {
        self.core_influence(grid_point, 2.0)
    }

    pub fn core_influence(&self, grid_point: Vec2, scaling_power: f32) -> Vec2 {
        // Ensure radius is never zero to avoid division by zero
        let radius = self.radius.max(f32::EPSILON);

        let distance_to_center = match &self.custom_distance_fn {
            Some(f) => f(grid_point, self.position),
            None => grid_point.distance(self.position),
        };

        if distance_to_center == 0.0 {
            return vec2(0.0, 0.0);
        }

        // Calculate how close the point is to the displacer on a 0-1 scale
        // 1.0 = point is at center, 0.0 = point is at or beyond twice the radius
        let proximity = 1.0 - distance_to_center / (radius * 2.0);

        // Ensure proximity doesn't go negative (keeps points beyond radius * 2 at zero)
        let distance_factor = proximity.max(0.0);

        // Calculate the angle between the grid point and displacer center
        // atan2 gives us angle in radians (-π to π) based on x,y differences
        let angle = (grid_point.y - self.position.y)
            .atan2(grid_point.x - self.position.x);

        // Calculate force magnitude:
        // - Squared distance factor makes influence drop off quadratically
        // - Multiply by strength to control overall displacement amount
        let force = self.strength * distance_factor.powf(scaling_power);

        // Convert polar coordinates (angle & force) to cartesian (x,y):
        let dx = angle.cos() * force;
        let dy = angle.sin() * force;

        vec2(dx, dy)
    }

    pub fn attract(&self, grid_point: Vec2, scaling_power: f32) -> Vec2 {
        let radius = self.radius.max(f32::EPSILON);

        let distance_to_center = match &self.custom_distance_fn {
            Some(f) => {
                let dist = f(grid_point, self.position);
                if dist.is_nan() || dist < 0.0 {
                    return vec2(0.0, 0.0);
                }
                dist
            }
            None => grid_point.distance(self.position),
        };

        if distance_to_center == 0.0 {
            return vec2(0.0, 0.0);
        }

        let proximity =
            1.0 - (distance_to_center / (radius * 2.0)).clamp(0.0, 1.0);
        let distance_factor = proximity.max(0.0);

        let force = self.strength
            * distance_factor
            * (distance_to_center / radius)
                .clamp(0.0, f32::MAX)
                .powf(scaling_power);

        if !force.is_finite() {
            return vec2(0.0, 0.0);
        }

        let angle = (grid_point.y - self.position.y)
            .atan2(grid_point.x - self.position.x);

        let dx = -angle.cos() * force;
        let dy = -angle.sin() * force;

        vec2(dx, dy)
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.position = position;
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    pub fn set_strength(&mut self, strength: f32) {
        self.strength = strength;
    }

    pub fn set_custom_distance_fn(
        &mut self,
        custom_distance_fn: CustomDistanceFn,
    ) {
        self.custom_distance_fn = custom_distance_fn;
    }
}

impl Debug for Displacer {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Displacer")
            .field("position", &self.position)
            .field("radius", &self.radius)
            .field("strength", &self.strength)
            .field("custom_distance_fn", &"<function>")
            .finish()
    }
}
