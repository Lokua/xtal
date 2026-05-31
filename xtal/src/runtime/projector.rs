use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProjectorQuality {
    Crisp,
    Balanced,
    Fast,
    Faster,
    Emergency,
}

impl Default for ProjectorQuality {
    fn default() -> Self {
        Self::Balanced
    }
}

impl ProjectorQuality {
    pub fn max_pixels(self) -> u64 {
        match self {
            Self::Crisp => 2_560 * 1_440,
            Self::Balanced => 1_920 * 1_080,
            Self::Fast => 1_280 * 720,
            Self::Faster => 1_120 * 630,
            Self::Emergency => 960 * 540,
        }
    }
}

pub fn internal_render_size(
    surface_size: [u32; 2],
    projector_mode_enabled: bool,
    quality: ProjectorQuality,
) -> [u32; 2] {
    let width = surface_size[0].max(1);
    let height = surface_size[1].max(1);

    if !projector_mode_enabled {
        return [width, height];
    }

    let current_pixels = width as u64 * height as u64;
    let max_pixels = quality.max_pixels();
    if current_pixels <= max_pixels {
        return [width, height];
    }

    let scale = (max_pixels as f64 / current_pixels as f64).sqrt();
    let scaled_width = ((width as f64 * scale).floor() as u32).max(1);
    let scaled_height = ((height as f64 * scale).floor() as u32).max(1);

    [scaled_width, scaled_height]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_native_size_when_disabled() {
        assert_eq!(
            internal_render_size([3840, 2160], false, ProjectorQuality::Fast),
            [3840, 2160]
        );
    }

    #[test]
    fn keeps_native_size_when_under_budget() {
        assert_eq!(
            internal_render_size([1280, 720], true, ProjectorQuality::Balanced),
            [1280, 720]
        );
    }

    #[test]
    fn caps_common_4k_balanced_to_1080p_budget() {
        assert_eq!(
            internal_render_size(
                [3840, 2160],
                true,
                ProjectorQuality::Balanced
            ),
            [1920, 1080]
        );
    }

    #[test]
    fn caps_common_4k_faster_to_intermediate_budget() {
        assert_eq!(
            internal_render_size([3840, 2160], true, ProjectorQuality::Faster),
            [1120, 630]
        );
    }

    #[test]
    fn preserves_wide_aspect_without_assuming_common_output_size() {
        let size = internal_render_size(
            [6000, 2000],
            true,
            ProjectorQuality::Balanced,
        );
        assert!(
            size[0] as u64 * size[1] as u64
                <= ProjectorQuality::Balanced.max_pixels()
        );
        assert!((size[0] as f64 / size[1] as f64 - 3.0).abs() < 0.01);
    }

    #[test]
    fn preserves_portrait_aspect_without_assuming_common_output_size() {
        let size = internal_render_size(
            [1800, 3200],
            true,
            ProjectorQuality::Emergency,
        );
        assert!(
            size[0] as u64 * size[1] as u64
                <= ProjectorQuality::Emergency.max_pixels()
        );
        assert!(
            (size[0] as f64 / size[1] as f64 - 1800.0 / 3200.0).abs() < 0.01
        );
    }

    #[test]
    fn clamps_zero_dimensions() {
        assert_eq!(
            internal_render_size([0, 0], true, ProjectorQuality::Emergency),
            [1, 1]
        );
    }
}
