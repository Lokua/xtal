use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "breakpoints_2",
    display_name: "Breakpoints 2",
    play_mode: PlayMode::Advance,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(700),
};

#[derive(SketchComponents)]
pub struct Model {
    animation: Animation<ManualTiming>,
    controls: ControlScript<ManualTiming>,
    wr: WindowRect,
    lanes: Vec<Vec<[f32; 2]>>,
    slew_limiter: SlewLimiter,
    hysteresis: Hysteresis,
    wave_folder: WaveFolder,
    quantizer: Quantizer,
}

pub fn init_model(_app: &App, wr: WindowRect) -> Model {
    let timing = ManualTiming::new(SKETCH_CONFIG.bpm);
    let animation = Animation::new(timing.clone());
    let controls = ControlScript::new(
        to_absolute_path(file!(), "breakpoints_2.yaml"),
        timing,
    );

    let slew_limiter = SlewLimiter::default();
    let hysteresis = Hysteresis::new(0.3, 0.7, 0.0, 1.0);
    let wave_folder = WaveFolder::default();
    let quantizer = Quantizer::default();

    Model {
        animation,
        controls,
        wr,
        lanes: vec![],
        slew_limiter,
        hysteresis,
        wave_folder,
        quantizer,
    }
}

pub fn update(_app: &App, m: &mut Model, _update: Update) {
    m.controls.update();

    if m.controls.changed() {
        let slew = m.controls.bool("slew");
        let rise = m.controls.get("rise");
        let fall = m.controls.get("fall");
        m.slew_limiter.set_rates(rise, fall);

        let hyst = m.controls.bool("hyst");
        m.hysteresis.pass_through = m.controls.bool("hyst_pass_through");
        m.hysteresis.lower_threshold = m.controls.get("lower_threshold");
        m.hysteresis.upper_threshold = m.controls.get("upper_threshold");
        m.hysteresis.output_low = m.controls.get("output_low");
        m.hysteresis.output_high = m.controls.get("output_high");

        let fold = m.controls.bool("fold");
        m.wave_folder.gain = m.controls.get("fold_gain");
        m.wave_folder.iterations =
            m.controls.get("fold_iterations").floor() as usize;
        m.wave_folder.symmetry = m.controls.get("fold_symmetry");
        m.wave_folder.bias = m.controls.get("fold_bias");
        m.wave_folder.shape = m.controls.get("fold_shape");

        let quant = m.controls.bool("quant");
        m.quantizer.step = m.controls.get("quant_step");

        let n_points = m.controls.get("n_points").floor() as usize;

        m.lanes.clear();
        m.lanes.extend(vec![
            create_points(
                &mut m.animation,
                &m.controls.breakpoints("points"),
                n_points,
                ternary!(slew, Some(&mut m.slew_limiter), None),
                ternary!(hyst, Some(&mut m.hysteresis), None),
                ternary!(fold, Some(&mut m.wave_folder), None),
                ternary!(quant, Some(&mut m.quantizer), None),
            ),
            create_points(
                &mut m.animation,
                &m.controls.breakpoints("points_2"),
                n_points,
                ternary!(slew, Some(&mut m.slew_limiter), None),
                ternary!(hyst, Some(&mut m.hysteresis), None),
                ternary!(fold, Some(&mut m.wave_folder), None),
                ternary!(quant, Some(&mut m.quantizer), None),
            ),
        ]);

        m.controls.mark_unchanged();
    }
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .color(gray(0.1));

    let track_height = m.wr.h() / 4.0;
    let track_h_margin = 12.0;
    let track_v_margin = 12.0;
    let track_h_padding = 12.0;
    let track_v_padding = 4.0;
    let track_height_with_margin = track_height - (track_v_margin * 2.0);

    let get_y_offset = |i: usize| {
        (m.wr.h() / 2.0) - (track_height * (i as f32 + 0.5)) + track_v_margin
    };

    // Draw track backgrounds for each lane
    for (i, _) in m.lanes.iter().enumerate() {
        let y_offset = get_y_offset(i);

        draw.rect()
            .x_y(0.0, y_offset)
            .w_h(m.wr.w() - track_h_margin, track_height_with_margin)
            .color(gray(0.15));
    }

    // Draw points for each lane
    for (i, lane) in m.lanes.iter().enumerate() {
        let y_offset = get_y_offset(i);

        for point in lane {
            draw.ellipse()
                .x_y(
                    map_range(
                        point[0],
                        0.0,
                        lane.last().unwrap()[0],
                        -m.wr.hw() + track_h_padding,
                        m.wr.hw() - track_h_padding,
                    ),
                    y_offset
                        + map_range(
                            point[1],
                            0.0,
                            1.0,
                            -(track_height_with_margin / 2.0) + track_v_padding,
                            track_height_with_margin / 2.0 - track_v_padding,
                        ),
                )
                .radius(1.0)
                .color(PALETURQUOISE);
        }
    }

    draw.to_frame(app, &frame).unwrap();
}

fn create_points(
    animation: &mut Animation<ManualTiming>,
    breakpoints: &[Breakpoint],
    n_points: usize,
    mut slew_limiter: Option<&mut SlewLimiter>,
    mut hysteresis: Option<&mut Hysteresis>,
    mut wave_folder: Option<&mut WaveFolder>,
    mut quantizer: Option<&mut Quantizer>,
) -> Vec<[f32; 2]> {
    let mut points: Vec<[f32; 2]> = vec![];
    let total_beats = breakpoints.last().unwrap().position;
    let step = total_beats / n_points as f32;
    for t in 0..n_points {
        animation.timing.set_beats(t as f32 * step);
        let anim = animation.automate(breakpoints, Mode::Once);
        let processed = post_pipeline(
            anim,
            &mut slew_limiter,
            &mut hysteresis,
            &mut wave_folder,
            &mut quantizer,
        );
        points.push([animation.beats(), processed]);
    }
    points
}

fn post_pipeline(
    value: f32,
    slew_limiter: &mut Option<&mut SlewLimiter>,
    hysteresis: &mut Option<&mut Hysteresis>,
    wave_folder: &mut Option<&mut WaveFolder>,
    quantizer: &mut Option<&mut Quantizer>,
) -> f32 {
    let mut value = value;
    if let Some(slew) = slew_limiter {
        value = slew.apply(value);
    }
    if let Some(hyst) = hysteresis {
        value = hyst.apply(value);
    }
    if let Some(fold) = wave_folder {
        value = fold.apply(value);
    }
    if let Some(quant) = quantizer {
        value = quant.apply(value);
    }
    value
}
