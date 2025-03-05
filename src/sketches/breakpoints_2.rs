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
pub struct Breakpoints2 {
    animation: Animation<ManualTiming>,
    controls: ControlScript<ManualTiming>,
    lanes: Vec<Vec<[f32; 2]>>,
    slew_limiter: SlewLimiter,
    hysteresis: Hysteresis,
    wave_folder: WaveFolder,
    quantizer: Quantizer,
    saturator: Saturator,
    ring_modulator: RingModulator,
}

pub fn init(_app: &App, ctx: &LatticeContext) -> Breakpoints2 {
    let timing = ManualTiming::new(ctx.bpm());
    let animation = Animation::new(timing.clone());
    let controls = ControlScript::from_path(
        to_absolute_path(file!(), "breakpoints_2.yaml"),
        timing,
    );

    let slew_limiter = SlewLimiter::default();
    let hysteresis = Hysteresis::default();
    let wave_folder = WaveFolder::default();
    let quantizer = Quantizer::default();
    let saturator = Saturator::default();
    let ring_modulator = RingModulator::default();

    Breakpoints2 {
        animation,
        controls,
        lanes: vec![],
        slew_limiter,
        hysteresis,
        wave_folder,
        quantizer,
        saturator,
        ring_modulator,
    }
}

impl Sketch for Breakpoints2 {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        self.controls.update();

        if self.controls.changed() {
            let slew = self.controls.bool("slew");
            let rise = self.controls.get("rise");
            let fall = self.controls.get("fall");
            self.slew_limiter.set_rates(rise, fall);

            let hyst = self.controls.bool("hyst");
            self.hysteresis.pass_through =
                self.controls.bool("hyst_pass_through");
            self.hysteresis.lower_threshold =
                self.controls.get("lower_threshold");
            self.hysteresis.upper_threshold =
                self.controls.get("upper_threshold");
            self.hysteresis.output_low = self.controls.get("output_low");
            self.hysteresis.output_high = self.controls.get("output_high");

            let fold = self.controls.bool("fold");
            self.wave_folder.gain = self.controls.get("fold_gain");
            self.wave_folder.iterations =
                self.controls.get("fold_iterations").floor() as usize;
            self.wave_folder.symmetry = self.controls.get("fold_symmetry");
            self.wave_folder.bias = self.controls.get("fold_bias");
            self.wave_folder.shape = self.controls.get("fold_shape");

            let quant = self.controls.bool("quant");
            self.quantizer.step = self.controls.get("quant_step");

            let sat = self.controls.bool("sat");
            self.saturator.drive = self.controls.get("sat_drive");

            let rm = self.controls.bool("rm");
            self.ring_modulator.mix = self.controls.get("rm_mix");

            let n_points = self.controls.get("n_points").floor() as usize;

            self.lanes.clear();
            self.lanes.extend(vec![
                create_points(
                    &mut self.animation,
                    &self.controls.breakpoints("points"),
                    n_points,
                    ternary!(slew, Some(&mut self.slew_limiter), None),
                    ternary!(hyst, Some(&mut self.hysteresis), None),
                    ternary!(fold, Some(&mut self.wave_folder), None),
                    ternary!(quant, Some(&mut self.quantizer), None),
                    ternary!(sat, Some(&mut self.saturator), None),
                ),
                create_points(
                    &mut self.animation,
                    &self.controls.breakpoints("points_2"),
                    n_points,
                    ternary!(slew, Some(&mut self.slew_limiter), None),
                    ternary!(hyst, Some(&mut self.hysteresis), None),
                    ternary!(fold, Some(&mut self.wave_folder), None),
                    ternary!(quant, Some(&mut self.quantizer), None),
                    ternary!(sat, Some(&mut self.saturator), None),
                ),
                create_points(
                    &mut self.animation,
                    &self.controls.breakpoints("points_3"),
                    n_points,
                    ternary!(slew, Some(&mut self.slew_limiter), None),
                    ternary!(hyst, Some(&mut self.hysteresis), None),
                    ternary!(fold, Some(&mut self.wave_folder), None),
                    ternary!(quant, Some(&mut self.quantizer), None),
                    ternary!(sat, Some(&mut self.saturator), None),
                ),
                create_modulated_points(
                    &mut self.animation,
                    &self.controls.breakpoints("points_2"),
                    &self.controls.breakpoints("points_3"),
                    n_points,
                    ternary!(slew, Some(&mut self.slew_limiter), None),
                    ternary!(hyst, Some(&mut self.hysteresis), None),
                    ternary!(fold, Some(&mut self.wave_folder), None),
                    ternary!(quant, Some(&mut self.quantizer), None),
                    ternary!(sat, Some(&mut self.saturator), None),
                    ternary!(rm, Some(&mut self.ring_modulator), None),
                ),
            ]);

            self.controls.mark_unchanged();
        }
    }

    fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
        let wr = ctx.window_rect();
        let draw = app.draw();

        draw.rect()
            .x_y(0.0, 0.0)
            .w_h(wr.w(), wr.h())
            .color(gray(0.1));

        let track_height = (wr.h() / self.lanes.len() as f32) - 6.0;
        let track_h_margin = 12.0;
        let track_v_margin = 12.0;
        let track_h_padding = 12.0;
        let track_v_padding = 4.0;
        let track_height_with_margin = track_height - (track_v_margin * 2.0);

        let get_y_offset = |i: usize| {
            (wr.h() / 2.0) - (track_height * (i as f32 + 0.5)) - track_v_margin
        };

        // Draw track backgrounds for each lane
        for (i, _) in self.lanes.iter().enumerate() {
            let y_offset = get_y_offset(i);

            draw.rect()
                .x_y(0.0, y_offset)
                .w_h(wr.w() - (track_h_margin * 2.0), track_height_with_margin)
                .color(gray(0.15));
        }

        // Draw points for each lane
        for (i, lane) in self.lanes.iter().enumerate() {
            let y_offset = get_y_offset(i);

            for point in lane {
                draw.ellipse()
                    .x_y(
                        map_range(
                            point[0],
                            0.0,
                            lane.last().unwrap()[0],
                            -wr.hw() + track_h_padding,
                            wr.hw() - track_h_padding,
                        ),
                        y_offset
                            + map_range(
                                point[1],
                                0.0,
                                1.0,
                                -(track_height_with_margin / 2.0)
                                    + track_v_padding,
                                track_height_with_margin / 2.0
                                    - track_v_padding,
                            ),
                    )
                    .radius(1.0)
                    .color(PALETURQUOISE);
            }
        }

        draw.to_frame(app, &frame).unwrap();
    }
}

fn create_points(
    animation: &mut Animation<ManualTiming>,
    breakpoints: &[Breakpoint],
    n_points: usize,
    mut slew_limiter: Option<&mut SlewLimiter>,
    mut hysteresis: Option<&mut Hysteresis>,
    mut wave_folder: Option<&mut WaveFolder>,
    mut quantizer: Option<&mut Quantizer>,
    mut saturator: Option<&mut Saturator>,
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
            &mut saturator,
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
    saturator: &mut Option<&mut Saturator>,
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
    if let Some(sat) = saturator {
        value = sat.apply(value);
    }
    value
}

fn create_modulated_points(
    animation: &mut Animation<ManualTiming>,
    carrier: &[Breakpoint],
    modulator: &[Breakpoint],
    n_points: usize,
    mut slew_limiter: Option<&mut SlewLimiter>,
    mut hysteresis: Option<&mut Hysteresis>,
    mut wave_folder: Option<&mut WaveFolder>,
    mut quantizer: Option<&mut Quantizer>,
    mut saturator: Option<&mut Saturator>,
    mut ring_modulator: Option<&mut RingModulator>,
) -> Vec<[f32; 2]> {
    let mut points: Vec<[f32; 2]> = vec![];
    let total_beats = carrier.last().unwrap().position;
    let step = total_beats / n_points as f32;
    for t in 0..n_points {
        animation.timing.set_beats(t as f32 * step);
        let c = animation.automate(carrier, Mode::Once);
        let m = animation.automate(modulator, Mode::Once);
        let processed = modulated_post_pipeline(
            c,
            m,
            &mut slew_limiter,
            &mut hysteresis,
            &mut wave_folder,
            &mut quantizer,
            &mut saturator,
            &mut ring_modulator,
        );
        points.push([animation.beats(), processed]);
    }
    points
}

fn modulated_post_pipeline(
    value_a: f32,
    value_b: f32,
    slew_limiter: &mut Option<&mut SlewLimiter>,
    hysteresis: &mut Option<&mut Hysteresis>,
    wave_folder: &mut Option<&mut WaveFolder>,
    quantizer: &mut Option<&mut Quantizer>,
    saturator: &mut Option<&mut Saturator>,
    ring_modulator: &mut Option<&mut RingModulator>,
) -> f32 {
    let mut value = value_a;
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
    if let Some(sat) = saturator {
        value = sat.apply(value);
    }
    if let Some(rm) = ring_modulator {
        value = rm.apply(value, value_b);
    }
    value
}
