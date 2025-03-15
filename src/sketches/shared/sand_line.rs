use nannou::prelude::*;

use crate::framework::prelude::*;

pub trait NoiseStrategy {
    fn generate_noise(&self, n_points: usize, scale: f32) -> Vec<f32>;
}

pub trait PointDistributionStrategy {
    fn distribute_points(
        &self,
        reference_points: &[Vec2],
        noise_values: &[f32],
        points_per_segment: usize,
        angle_variation: f32,
    ) -> Vec<Vec2>;
}

pub struct SandLine {
    noise_strategy: Box<dyn NoiseStrategy>,
    distribution_strategy: Box<dyn PointDistributionStrategy>,
}
impl SandLine {
    pub fn new(
        noise_strategy: Box<dyn NoiseStrategy>,
        distribution_strategy: Box<dyn PointDistributionStrategy>,
    ) -> Self {
        Self {
            noise_strategy,
            distribution_strategy,
        }
    }

    pub fn generate(
        &self,
        reference_points: &[Vec2],
        noise_scale: f32,
        points_per_segment: usize,
        angle_variation: f32,
        passes: usize,
    ) -> Vec<Vec2> {
        let mut points = Vec::new();

        for _ in 0..passes {
            let noise = self
                .noise_strategy
                .generate_noise(reference_points.len(), noise_scale);

            let pass_points = self.distribution_strategy.distribute_points(
                reference_points,
                &noise,
                points_per_segment,
                angle_variation,
            );

            points.extend(pass_points);
        }

        points
    }
}

pub struct GaussianNoise;
impl NoiseStrategy for GaussianNoise {
    fn generate_noise(&self, n_points: usize, scale: f32) -> Vec<f32> {
        let mut noise = vec![0.0; n_points];
        for n in noise.iter_mut() {
            *n += random_normal(1.0) * scale;
        }
        noise
    }
}

pub struct OctaveNoise {
    /// Number of layers of noise to add together.
    /// Higher values (like 4-8) create more detail in the noise.
    /// Each octave adds finer detail but with decreasing influence.
    octaves: u32,

    /// How quickly each octave's influence decreases (between 0.0 and 1.0).
    ///
    /// - Values closer to 0 mean later octaves contribute very little
    /// - Values closer to 1 mean all octaves contribute more equally
    /// - Typical values are 0.5 to 0.8
    persistence: f32,
}
impl OctaveNoise {
    pub fn new(octaves: u32, persistence: f32) -> Self {
        Self {
            octaves,
            persistence,
        }
    }
}
impl NoiseStrategy for OctaveNoise {
    fn generate_noise(&self, n_points: usize, scale: f32) -> Vec<f32> {
        let mut noise = vec![0.0; n_points];
        for n in noise.iter_mut() {
            let mut amplitude = scale;

            for _ in 0..self.octaves {
                *n += random_normal(1.0) * amplitude;
                amplitude *= self.persistence;
            }
        }
        noise
    }
}

pub struct PerpendicularDistribution;
impl PointDistributionStrategy for PerpendicularDistribution {
    fn distribute_points(
        &self,
        reference_points: &[Vec2],
        noise_values: &[f32],
        points_per_segment: usize,
        angle_variation: f32,
    ) -> Vec<Vec2> {
        let mut output_points = Vec::new();

        for (index, point) in reference_points.iter().enumerate() {
            if index < reference_points.len() - 1 {
                let next_point = reference_points[index + 1];

                for _ in 0..points_per_segment {
                    // Step 1: Get random point along line (t = random value between 0 and 1)
                    //      next_point
                    //      |
                    //      |
                    //      |
                    //      |
                    //      |  * (base_point = lerp(point, next_point, t))
                    //      |
                    //      |
                    //      point
                    let t = random::<f32>();
                    let base_point = point.lerp(next_point, t);

                    //      // Step 2: Calculate perpendicular angle (with random variation)
                    //      next_point
                    //      |          θ
                    //      |       \ ↗
                    //      |        \   θ = PI/2 + random_variation
                    //      |         \
                    //      |  * ------
                    //      |
                    //      |
                    //      point
                    //
                    // NOTE: this is perpendicular to the x-axis, not the actual line direction
                    // should probably fix that?
                    let base_angle = PI / 2.0;
                    let angle = base_angle + random_normal(angle_variation);

                    // Step 3: Calculate noise amount by interpolating between noise values
                    //      next_point     noise_values[index + 1]
                    //      |          ↓
                    //      |       \  |  length = lerp(noise_values[index],
                    //      |        \ |                noise_values[index + 1],
                    //      |         \|                t)
                    //      |  * ------
                    //      |          |
                    //      |          ↑
                    //      point
                    let noise_amount =
                        lerp(noise_values[index], noise_values[index + 1], t);

                    // Step 4: Final point placement using angle and noise amount
                    //      next_point
                    //      |      * (final point = base_point + offset)
                    //      |     ↗    offset = vec2(noise_amount * cos(angle),
                    //      |    /             noise_amount * sin(angle))
                    //      |   /
                    //      |  *
                    //      |
                    //      |
                    //      point
                    let offset = vec2(
                        noise_amount * angle.cos(),
                        noise_amount * angle.sin(),
                    );

                    output_points.push(base_point + offset);
                }
            }
        }

        output_points
    }
}

pub struct CurvedDistribution {
    /// Controls how much the distribution curves away from perpendicular.
    /// Recommended range: 0.0 to 1.0
    ///
    /// - 0.0: No curve (same as perpendicular)
    /// - 1.0: Curves up to ±57 degrees from perpendicular
    /// - PI/2 (≈1.57): Curves up to ±90 degrees
    /// - Values above 2.0 create extreme curves
    curvature: f32,
}
impl CurvedDistribution {
    pub fn new(curvature: f32) -> Self {
        Self { curvature }
    }
}
impl PointDistributionStrategy for CurvedDistribution {
    fn distribute_points(
        &self,
        reference_points: &[Vec2],
        noise_values: &[f32],
        points_per_segment: usize,
        angle_variation: f32,
    ) -> Vec<Vec2> {
        let mut output_points = Vec::new();

        for (index, point) in reference_points.iter().enumerate() {
            if index < reference_points.len() - 1 {
                let next_point = reference_points[index + 1];

                for i in 0..points_per_segment {
                    let t = i as f32 / points_per_segment as f32;
                    let base_point = point.lerp(next_point, t);

                    // Add curved offset based on parameter
                    let curve_factor = (t * PI).sin() * self.curvature;
                    let base_angle = PI / 2.0 + curve_factor;
                    let angle = base_angle + random_normal(angle_variation);
                    let noise_amount =
                        lerp(noise_values[index], noise_values[index + 1], t);
                    let offset = vec2(
                        noise_amount * angle.cos(),
                        noise_amount * angle.sin(),
                    );

                    output_points.push(base_point + offset);
                }
            }
        }

        output_points
    }
}

pub struct TrigFnDistribution {
    curvature: f32,
    trig_fn_a: fn(f32) -> f32,
    trig_fn_b: fn(f32) -> f32,
}
impl TrigFnDistribution {
    pub fn new(
        curvature: f32,
        trig_fn_a: fn(f32) -> f32,
        trig_fn_b: fn(f32) -> f32,
    ) -> Self {
        Self {
            curvature,
            trig_fn_a,
            trig_fn_b,
        }
    }
}
impl PointDistributionStrategy for TrigFnDistribution {
    fn distribute_points(
        &self,
        reference_points: &[Vec2],
        noise_values: &[f32],
        points_per_segment: usize,
        angle_variation: f32,
    ) -> Vec<Vec2> {
        let mut output_points = Vec::new();

        for (index, point) in reference_points.iter().enumerate() {
            if index < reference_points.len() - 1 {
                let next_point = reference_points[index + 1];

                for i in 0..points_per_segment {
                    let t = i as f32 / points_per_segment as f32;
                    let base_point = point.lerp(next_point, t);

                    let curve_factor = (t * PI).sin() * self.curvature;
                    let base_angle = PI / 2.0 + curve_factor;
                    let angle = base_angle + random_normal(angle_variation);

                    let noise_amount =
                        lerp(noise_values[index], noise_values[index + 1], t);

                    let offset = vec2(
                        noise_amount * (self.trig_fn_a)(angle),
                        noise_amount * (self.trig_fn_b)(angle),
                    );

                    output_points.push(base_point + offset);
                }
            }
        }

        output_points
    }
}
