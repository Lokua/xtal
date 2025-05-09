use xtal::prelude::*;

mod sketches;
pub use sketches::common::util;
use sketches::*;

fn main() {
    init_logger();

    register!(
        // ---------------------------------------------------------------------
        // MAIN
        // ---------------------------------------------------------------------
        blob,
        cloud_tunnel,
        d_warp,
        displacement_2a,
        drop,
        drop_tines,
        drop_walk,
        dynamic_uniforms,
        g_warp,
        grid_splash,
        heat_mask,
        interference,
        kalos,
        kalos_2,
        marcher,
        neon,
        ray_marching,
        sand_lines,
        shaxper,
        spiral,
        spiral_lines,
        swirl,
        symmetry,
        toxy,
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
        basics,
        bos,
        breakpoints_2,
        brutalism,
        chromatic_aberration,
        displacement_1,
        displacement_1a,
        displacement_2,
        easing_vis,
        fb,
        flow_field_basic,
        lines,
        noise,
        perlin_loop,
        rm_rep,
        sand_line,
        sand_lines,
        sierpinski_triangle,
        unt,
        vertical,
        vertical_2,
        z_sim,
        // ---------------------------------------------------------------------
        // TEMPLATES
        // ---------------------------------------------------------------------
        template,
        basic_cube_shader_template,
        du_fs_template,
        du_fs_texture_template,
        fullscreen_shader_template
    );

    run();
}
