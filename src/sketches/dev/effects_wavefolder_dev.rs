use nannou::color::*;
use nannou::prelude::*;

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "effects_wavefolder_dev",
    display_name: "Effects WaveFolder Dev",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 500,
    h: 500,
    gui_w: None,
    gui_h: Some(300),
};

const N_POINTS: usize = 2048;

#[derive(SketchComponents)]
pub struct EffectsWavefolderDev {
    animation: Animation<ManualTiming>,
    lanes: Vec<Vec<[f32; 2]>>,
    wave_folder: WaveFolder,
    controls: Controls,
}

pub fn init(_app: &App, ctx: LatticeContext) -> EffectsWavefolderDev {
    let animation = Animation::new(ManualTiming::new(ctx.bpm()));
    let wave_folder = WaveFolder::default();

    let controls = Controls::new(vec![
        Control::slider("gain", 1.0, (1.0, 5.0), 0.125),
        Control::slider("iterations", 1.0, (1.0, 5.0), 1.0),
        Control::slider("symmetry", 1.0, (-1.0, 1.0), 0.125),
        Control::slider("bias", 0.0, (-1.0, 2.0), 0.125),
        Control::slider("shape", 0.0, (-2.0, 2.0), 0.125),
    ]);

    EffectsWavefolderDev {
        lanes: vec![],
        wave_folder,
        animation,
        controls,
    }
}

impl Sketch for EffectsWavefolderDev {
    fn update(&mut self, _app: &App, _update: Update, _ctx: &LatticeContext) {
        self.wave_folder.gain = self.controls.get("gain");
        self.wave_folder.iterations = self.controls.get("iterations") as usize;
        self.wave_folder.symmetry = self.controls.get("symmetry");
        self.wave_folder.bias = self.controls.get("bias");
        self.wave_folder.shape = self.controls.get("shape");

        self.lanes = vec![create_points(
            &mut self.animation,
            &[
                Breakpoint::ramp(0.0, 0.0, Easing::Linear),
                Breakpoint::ramp(1.0, 1.0, Easing::Linear),
                Breakpoint::ramp(2.0, 0.0, Easing::Linear),
                Breakpoint::ramp(3.0, 1.0, Easing::Linear),
                Breakpoint::ramp(4.0, 0.0, Easing::Linear),
                Breakpoint::ramp(5.0, 1.0, Easing::Linear),
                Breakpoint::end(6.0, 0.0),
            ],
            N_POINTS,
            &self.wave_folder,
        )];
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
    wave_folder: &WaveFolder,
) -> Vec<[f32; 2]> {
    let mut points: Vec<[f32; 2]> = vec![];
    let total_beats = breakpoints.last().unwrap().position;
    let step = total_beats / n_points as f32;
    for t in 0..n_points {
        animation.timing.set_beats(t as f32 * step);
        let value = animation.automate(breakpoints, Mode::Once);
        let folded = wave_folder.apply(value);
        points.push([animation.beats(), folded]);
    }
    points
}
