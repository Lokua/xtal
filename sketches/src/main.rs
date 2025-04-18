use xtal::prelude::*;

mod sketches;
pub use sketches::shared::util;
use sketches::*;

fn main() {
    init_logger();

    register!(
        // ---------------------------------------------------------------------
        // MAIN
        // ---------------------------------------------------------------------
        blob,
        breakpoints_2,
        brutalism,
        displacement_2a,
        drop,
        drop_tines,
        drop_walk,
        flow_field_basic,
        heat_mask,
        interference,
        kalos,
        kalos_2,
        sand_lines,
        shaxper,
        sierpinski_triangle,
        spiral,
        spiral_lines,
        symmetry,
        wave_fract,
        // ---------------------------------------------------------------------
        // DEV
        // ---------------------------------------------------------------------
        animation_dev,
        audio_controls_dev,
        audio_dev,
        bug_repro,
        control_script_dev,
        cv_dev,
        dynamic_uniforms,
        effects_wavefolder_dev,
        midi_dev,
        non_yaml_dev,
        osc_dev,
        osc_transport_dev,
        responsive_dev,
        shader_to_texture_dev,
        wgpu_compute_dev,
        // ---------------------------------------------------------------------
        // GENUARY 2025
        // ---------------------------------------------------------------------
        g25_1_horiz_vert,
        g25_2_layers,
        g25_5_isometric,
        g25_10_11_12,
        g25_13_triangle,
        g25_14_black_and_white,
        g25_18_wind,
        g25_19_op_art,
        g25_20_23_brutal_arch,
        g25_22_gradients_only,
        // ---------------------------------------------------------------------
        // SCRATCH
        // ---------------------------------------------------------------------
        bos,
        chromatic_aberration,
        displacement_1,
        displacement_1a,
        displacement_2,
        lines,
        noise,
        perlin_loop,
        sand_line,
        vertical,
        vertical_2,
        z_sim,
        // ---------------------------------------------------------------------
        // TEMPLATES
        // ---------------------------------------------------------------------
        template,
        basic_cube_shader_template,
        fullscreen_shader_template
    );

    run();
}
