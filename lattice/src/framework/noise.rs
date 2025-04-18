//! Wrappers around nannou::noise modules that simplify imports and work solely
//! with f32

use nannou::noise::{NoiseFn, OpenSimplex, Perlin, Seedable};

pub struct PerlinNoise {
    noise: Perlin,
}

impl PerlinNoise {
    pub fn new(seed: u32) -> Self {
        let noise = Perlin::new().set_seed(seed);
        Self { noise }
    }

    pub fn set_seed(&mut self, seed: u32) {
        self.noise = Perlin::new().set_seed(seed);
    }

    /// Returns random value in the range [-1.0, 1.0]
    pub fn get<const N: usize>(&self, point: [f32; N]) -> f32
    where
        Perlin: NoiseFn<[f64; N]>,
    {
        let point_f64 = point.map(|x| x as f64);
        self.noise.get(point_f64) as f32
    }
}

pub struct SimplexNoise {
    noise: OpenSimplex,
}

impl SimplexNoise {
    pub fn new(seed: u32) -> Self {
        let noise = OpenSimplex::new().set_seed(seed);
        Self { noise }
    }

    pub fn set_seed(&mut self, seed: u32) {
        self.noise = OpenSimplex::new().set_seed(seed);
    }

    /// Returns random value in the range [-1.0, 1.0]
    pub fn get<const N: usize>(&self, point: [f32; N]) -> f32
    where
        OpenSimplex: NoiseFn<[f64; N]>,
    {
        let point_f64 = point.map(|x| x as f64);
        self.noise.get(point_f64) as f32
    }
}
