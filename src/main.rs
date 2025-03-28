use lattice::framework::prelude::init_logger;
use lattice::framework::prelude::SketchAll;
use lattice::{register_sketches, runtime};

fn main() {
    init_logger();

    let mut registry = runtime::registry::REGISTRY.write().unwrap();

    register_sketches!(
        registry,
        // ---------------------------------------------------------------------
        // MAIN
        // ---------------------------------------------------------------------
        blob,
        breakpoints_2,
        brutalism,
        displacement_2a,
        drop,
        drop_walk,
        flow_field_basic,
        heat_mask,
        interference,
        kalos,
        kalos_2,
        sand_lines,
        sierpinski_triangle,
        spiral,
        spiral_lines,
        wave_fract,
        // ---------------------------------------------------------------------
        // DEV
        // ---------------------------------------------------------------------
        animation_dev,
        audio_controls_dev,
        audio_dev,
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
        g25_26_symmetry,
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
        shader_experiments,
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

    registry.prepare();
    drop(registry);

    runtime::app::run();
}
