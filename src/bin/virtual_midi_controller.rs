//! Provides an on-screen MIDI controller which is useful for debugging or
//! simulating hardware

use nannou::prelude::*;
use nannou::winit::event::WindowEvent;
use nannou_egui::{egui, Egui};
use std::cell::RefCell;

use lattice::framework::prelude::*;
use lattice::runtime::gui::gui;
use lattice::runtime::gui::theme;
use lattice::*;

fn main() {
    init_logger();
    gui::init();
    nannou::app(model).update(update).run();
}

struct Model {
    egui: Egui,
    port: RefCell<String>,
    midi_out: RefCell<midi::MidiOut>,
    channel: u8,
    controls: Vec<f32>,
    hi_res: bool,
}

fn model(app: &App) -> Model {
    let window_id = app
        .new_window()
        .title("VMC")
        .size(870, 408)
        .resizable(false)
        .view(view_gui)
        .raw_event(handle_raw_event)
        .build()
        .unwrap();

    let window = app.window(window_id).unwrap();
    let egui = Egui::from_window(&window);

    let port = config::MIDI_CONTROL_OUT_PORT;
    let mut midi_out = midi::MidiOut::new(port);
    match midi_out.connect() {
        Err(e) => panic!("{}", e),
        _ => {}
    }

    Model {
        egui,
        midi_out: RefCell::new(midi_out),
        controls: vec![0.0; 128],
        port: RefCell::new(port.to_string()),
        channel: 0,
        hi_res: false,
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    let ctx = model.egui.begin_frame();
    theme::apply(&ctx);
    let colors = theme::Colors::current();

    let mut style = (*ctx.style()).clone();
    style.spacing.slider_width = 200.0;
    ctx.set_style(style);

    let mut changed: Vec<(u8, f32)> = vec![];

    let ports =
        midi::list_ports(midi::InputsOrOutputs::Outputs).unwrap_or(vec![]);

    let mut selected_port = model.port.borrow().clone();

    egui::CentralPanel::default()
        .frame(
            egui::Frame::default()
                .fill(colors.bg_primary)
                .inner_margin(egui::Margin::same(16.0)),
        )
        .show(&ctx, |ui| {
            ui.horizontal(|ui| {
                egui::Frame::none().show(ui, |ui| {
                    egui::ComboBox::from_label("Port")
                        .selected_text(&selected_port)
                        .show_ui(ui, |ui| {
                            for (_, port_name) in ports {
                                ui.selectable_value(
                                    &mut selected_port,
                                    port_name.clone(),
                                    port_name,
                                );
                            }
                        });
                });

                ui.separator();

                egui::Frame::none().show(ui, |ui| {
                    egui::ComboBox::from_label("Channel")
                        .selected_text(&model.channel.to_string())
                        .width(48.0)
                        .show_ui(ui, |ui| {
                            ui.set_min_width(48.0);

                            for channel in 0..=15 {
                                ui.selectable_value(
                                    &mut model.channel,
                                    channel,
                                    channel.to_string(),
                                );
                            }
                        });
                });

                ui.separator();

                ui.add(egui::Checkbox::new(&mut model.hi_res, "Hi-Res"))
            });

            ui.separator();

            egui::Grid::new("sliders_grid")
                .num_columns(8)
                .spacing([4.0, 4.0])
                .min_col_width(50.0)
                .show(ui, |ui| {
                    for row in 0..16 {
                        for col in 0..8 {
                            let i = row * 8 + col;

                            let precision = ternary!(model.hi_res, 2, 0);

                            let number_box = ui.add(
                                egui::DragValue::new(&mut model.controls[i])
                                    .speed(0.5)
                                    .fixed_decimals(precision)
                                    .clamp_range(0.0..=127.0),
                            );

                            if number_box.changed() {
                                changed.push((i as u8, model.controls[i]));
                            }

                            ui.label(i.to_string());
                        }
                        ui.end_row();
                    }
                });
        });

    if selected_port != *model.port.borrow() {
        *model.port.borrow_mut() = selected_port.clone();
        let mut midi_out = midi::MidiOut::new(selected_port.as_str());
        match midi_out.connect() {
            Err(e) => panic!("{}", e),
            _ => {}
        }
        *model.midi_out.borrow_mut() = midi_out;
    }

    for (control, value) in changed {
        let mut midi_out = model.midi_out.borrow_mut();

        if model.hi_res {
            let status = 0xB0 | model.channel;

            let value_14bit = (value * 128.0).round() as u16;
            let value_msb = (value_14bit >> 7) as u8;
            let value_lsb = (value_14bit & 0x7F) as u8;

            let messages = [
                [status, control, value_msb],
                [status, control + 32, value_lsb],
            ];

            for message in messages {
                if let Err(e) = midi_out.send(&message) {
                    error!("{}", e);
                }
            }
        } else {
            if let Err(e) = midi_out.send(&[
                0xB0 + model.channel,
                control,
                value.round() as u8,
            ]) {
                error!("{}", e);
            }
        }
    }
}

fn view_gui(_app: &App, model: &Model, frame: Frame) {
    frame.clear(BLACK);
    model.egui.draw_to_frame(&frame).unwrap();
}

fn handle_raw_event(_app: &App, model: &mut Model, event: &WindowEvent) {
    log_resize(&event);
    model.egui.handle_raw_event(event)
}

fn log_resize(event: &WindowEvent) {
    if let WindowEvent::Resized(physical_size) = event {
        debug!(
            "Window resized: {}x{}",
            physical_size.width / 2,
            physical_size.height / 2
        );
    }
}
