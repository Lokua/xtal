use arboard::Clipboard;
use nannou::prelude::*;
use nannou_egui::egui::{self, FontDefinitions, FontFamily};
use nannou_egui::Egui;
use once_cell::sync::Lazy;
use std::cell::Cell;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use lattice::framework::frame_controller;
use lattice::framework::prelude::*;
use lattice::runtime::prelude::*;
use lattice::sketches;

// Core trait for type erasure - all sketches must implement this
pub trait Sketch {
    fn update(&mut self, app: &App, update: Update);
    fn view(&self, app: &App, frame: Frame);
    fn event(&mut self, app: &App, event: &Event);
    fn controls(&mut self) -> Option<&mut dyn ControlProvider>;
    fn clear_color(&self) -> nannou::color::Rgba;
    fn set_window_rect(&mut self, rect: Rect);
}

// Adapter to implement Sketch for your existing SketchModel types
struct SketchAdapter<S: SketchModel> {
    model: S,
    update_fn: fn(&App, &mut S, Update),
    view_fn: fn(&App, &S, Frame),
}

impl<S: SketchModel> SketchAdapter<S> {
    fn new(
        model: S,
        update_fn: fn(&App, &mut S, Update),
        view_fn: fn(&App, &S, Frame),
    ) -> Self {
        Self {
            model,
            update_fn,
            view_fn,
        }
    }
}

impl<S: SketchModel> Sketch for SketchAdapter<S> {
    fn update(&mut self, app: &App, update: Update) {
        (self.update_fn)(app, &mut self.model, update);
    }

    fn view(&self, app: &App, frame: Frame) {
        (self.view_fn)(app, &self.model, frame);
    }

    fn event(&mut self, app: &App, event: &Event) {
        self.model.event(app, event);
    }

    fn controls(&mut self) -> Option<&mut dyn ControlProvider> {
        self.model.controls().map(|c| c as &mut dyn ControlProvider)
    }

    fn clear_color(&self) -> nannou::color::Rgba {
        self.model.clear_color()
    }

    fn set_window_rect(&mut self, rect: Rect) {
        self.model.set_window_rect(rect);
    }
}

static REGISTRY: Lazy<Mutex<SketchRegistry>> =
    Lazy::new(|| Mutex::new(SketchRegistry::new()));

// Registry for sketches to allow runtime loading
struct SketchRegistry {
    sketches: HashMap<String, SketchInfo>,
}

struct SketchInfo {
    name: &'static str,
    display_name: &'static str,
    config: &'static SketchConfig,
    factory: Box<
        dyn for<'a> Fn(&'a App, Rect) -> Box<dyn Sketch + 'static>
            + Send
            + Sync,
    >,
    // factory: Box<dyn Fn(&App, Rect) -> Box<dyn Sketch>>,
}

impl SketchRegistry {
    fn new() -> Self {
        Self {
            sketches: HashMap::new(),
        }
    }

    fn register<F>(
        &mut self,
        name: &'static str,
        display_name: &'static str,
        config: &'static SketchConfig,
        factory: F,
    ) where
        F: Fn(&App, Rect) -> Box<dyn Sketch> + Send + Sync + 'static,
    {
        self.sketches.insert(
            name.to_string(),
            SketchInfo {
                name,
                display_name,
                config,
                factory: Box::new(factory),
            },
        );
    }

    fn get_sketch_info(&self, name: &str) -> Option<&SketchInfo> {
        self.sketches.get(name)
    }

    fn get_sketch_names(&self) -> Vec<String> {
        self.sketches.keys().cloned().collect()
    }
}

// Example implementation for SimpleSketch
struct SimpleSketch {
    position: Point2,
    size: f32,
    color: Hsla,
    window_rect: WindowRect,
    controls: Controls,
}

impl SimpleSketch {
    fn new(rect: Rect) -> Self {
        let mut controls = Controls::new(vec![
            Control::slider("size", 50.0, (10.0, 200.0), 0.1),
            Control::slider("hue", 0.5, (0.0, 1.0), 0.01),
        ]);

        Self {
            position: pt2(0.0, 0.0),
            size: 50.0,
            color: hsla(0.5, 0.8, 0.6, 1.0),
            window_rect: WindowRect::new(rect),
            controls,
        }
    }
}

impl SketchModel for SimpleSketch {
    fn controls(&mut self) -> Option<&mut impl ControlProvider> {
        Some(&mut self.controls)
    }

    fn window_rect(&mut self) -> Option<&mut WindowRect> {
        Some(&mut self.window_rect)
    }

    fn clear_color(&self) -> nannou::color::Rgba {
        nannou::color::rgba(0.1, 0.1, 0.1, 1.0)
    }
}

fn update_simple_sketch(
    _app: &App,
    _model: &mut SimpleSketch,
    _update: Update,
) {
}

fn view_simple_sketch(app: &App, model: &SimpleSketch, frame: Frame) {
    let draw = app.draw();

    draw.ellipse()
        .xy(model.position)
        .radius(model.size)
        .color(model.color);

    draw.to_frame(app, &frame).unwrap();
}

// Dynamic model for nannou
struct DynamicModel {
    main_window_id: window::Id,
    gui_window_id: window::Id,
    egui: Egui,
    session_id: String,
    alert_text: String,
    clear_flag: Cell<bool>,
    recording_state: RecordingState,
    current_sketch: Box<dyn Sketch>,
    current_sketch_name: String,
    current_sketch_config: &'static SketchConfig,
    gui_visible: Cell<bool>,
    main_visible: Cell<bool>,
    main_maximized: Cell<bool>,
}

fn main() {
    // Create sketch registry

    // Register example sketch
    let simple_config = &SketchConfig {
        name: "simple",
        display_name: "Simple Circle Demo",
        fps: 60.0,
        bpm: 120.0,
        w: 800,
        h: 600,
        gui_w: Some(300),
        gui_h: Some(400),
        play_mode: PlayMode::Loop,
    };

    let mut registry = REGISTRY.lock().unwrap();

    registry.register(
        "simple",
        "Simple Circle Demo",
        simple_config,
        |_app, rect| {
            let model = SimpleSketch::new(rect);
            Box::new(SketchAdapter::new(
                model,
                update_simple_sketch,
                view_simple_sketch,
            ))
        },
    );

    // In a real implementation, you'd register all your sketches here
    // For example, adapting your existing sketches:
    registry.register(
        "spiral",
        "Spiral Demo",
        &sketches::spiral::SKETCH_CONFIG,
        |app, rect| {
            let model =
                sketches::spiral::init_model(app, WindowRect::new(rect));
            Box::new(SketchAdapter::new(
                model,
                sketches::spiral::update,
                sketches::spiral::view,
            ))
        },
    );

    // Start nannou with our model
    nannou::app(model)
        .update(update)
        .view(view)
        .event(event)
        .run();
}

fn model(app: &App) -> DynamicModel {
    // Get initial sketch name from args or use default
    let args: Vec<String> = env::args().collect();
    let initial_sketch = args
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "simple".to_string());

    let registry = REGISTRY.lock().unwrap();

    // Get sketch info from registry
    let sketch_info = registry
        .get_sketch_info(&initial_sketch)
        .unwrap_or_else(|| panic!("Sketch not found: {}", initial_sketch));

    let sketch_config = sketch_info.config;

    // Create main window
    let main_window_id = app
        .new_window()
        .title(sketch_info.display_name)
        .size(sketch_config.w as u32, sketch_config.h as u32)
        .build()
        .unwrap();

    let window_rect = app
        .window(main_window_id)
        .expect("Unable to get window")
        .rect();

    // Create initial sketch instance
    let mut current_sketch = (sketch_info.factory)(app, window_rect);

    // Create control window with appropriate dimensions
    let calc_gui_dimensions =
        |controls: Option<&mut dyn ControlProvider>| -> (u32, u32) {
            let control_count = controls
                .map(|c| c.as_controls().get_controls().len())
                .unwrap_or(0);
            // Basic calculation - you might want to refine this
            let height = 40 + (control_count as u32 * 26) + 40;
            (300, height)
        };

    let (gui_w, gui_h) = calc_gui_dimensions(current_sketch.controls());

    let gui_window_id = app
        .new_window()
        .title(format!("{} Controls", sketch_config.display_name))
        .size(
            sketch_config.gui_w.unwrap_or(gui_w),
            sketch_config.gui_h.unwrap_or(gui_h),
        )
        .view(view_gui)
        .resizable(true)
        .raw_event(|_app, model: &mut DynamicModel, event| {
            model.egui.handle_raw_event(event);
        })
        .build()
        .unwrap();

    // Get egui instance for the GUI window
    let egui = Egui::from_window(&app.window(gui_window_id).unwrap());

    // Create session ID for recording
    let session_id = uuid_v4(); // You'll need to implement this
    let recording_dir = PathBuf::new(); // You'll need to implement frames_dir

    // Create our dynamic model
    DynamicModel {
        main_window_id,
        gui_window_id,
        egui,
        session_id,
        alert_text: "Lattice Dynamic initialized".into(),
        clear_flag: Cell::new(false),
        recording_state: RecordingState::new(Some(recording_dir)),
        current_sketch,
        current_sketch_name: initial_sketch,
        current_sketch_config: sketch_config,
        gui_visible: Cell::new(true),
        main_visible: Cell::new(true),
        main_maximized: Cell::new(false),
    }
}

fn update(app: &App, model: &mut DynamicModel, update: Update) {
    // Update EGUI
    model.egui.set_elapsed_time(update.since_start);
    let ctx = model.egui.begin_frame();
    update_gui(app, &ctx);

    // Update sketch with current window rect
    if let Some(window) = app.window(model.main_window_id) {
        let rect = window.rect();
        model.current_sketch.set_window_rect(rect);
    }

    // Use the frame controller for consistent timing
    frame_controller::wrapped_update(
        app,
        &mut model.current_sketch,
        update,
        |app, sketch, update| sketch.update(app, update),
    );
}

fn update_gui(app: &App, ctx: &egui::Context) {
    // Get available sketches for dropdown
    let registry = REGISTRY.lock().unwrap();

    let sketch_names = registry.get_sketch_names();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Lattice Dynamic");

        // Sketch selection dropdown
        egui::ComboBox::from_label("Current Sketch")
            .selected_text(&model.current_sketch_name)
            .show_ui(ui, |ui| {
                for name in &sketch_names {
                    if ui
                        .selectable_label(
                            model.current_sketch_name == *name,
                            name,
                        )
                        .clicked()
                    {
                        // User selected a new sketch - switch to it
                        if model.current_sketch_name != *name {
                            if let Some(sketch_info) =
                                registry.get_sketch_info(name)
                            {
                                switch_sketch(app, model, name, sketch_info);
                            }
                        }
                    }
                }
            });

        ui.separator();

        // Basic controls
        if ui.button("Clear").clicked() {
            model.clear_flag.set(true);
            model.alert_text = "Cleared".into();
        }

        // Standard controls for frame control
        if ui
            .button(if frame_controller::is_paused() {
                "Play"
            } else {
                "Pause"
            })
            .clicked()
        {
            let paused = !frame_controller::is_paused();
            frame_controller::set_paused(paused);
            model.alert_text = if paused {
                "Paused".into()
            } else {
                "Playing".into()
            };
        }

        if ui
            .add_enabled(
                frame_controller::is_paused(),
                egui::Button::new("Advance"),
            )
            .clicked()
        {
            frame_controller::advance_single_frame();
        }

        ui.separator();

        // Draw sketch-specific controls
        if let Some(controls) = model.current_sketch.controls() {
            draw_controls(controls.as_controls(), ui);
        }

        // Display status/alert text
        ui.separator();
        ui.label(&model.alert_text);
    });
}

fn switch_sketch(
    app: &App,
    model: &mut DynamicModel,
    name: &str,
    sketch_info: &SketchInfo,
) {
    // Update window title
    if let Some(window) = app.window(model.main_window_id) {
        window.set_title(sketch_info.display_name);
    }

    // Get current window rect
    let rect = app
        .window(model.main_window_id)
        .expect("Unable to get window")
        .rect();

    // Create new sketch instance
    let new_sketch = (sketch_info.factory)(app, rect);

    // Update model
    model.current_sketch = new_sketch;
    model.current_sketch_name = name.to_string();
    model.current_sketch_config = sketch_info.config;

    // Update control window title
    if let Some(window) = app.window(model.gui_window_id) {
        window.set_title(&format!("{} Controls", sketch_info.display_name));
    }

    // Update frame controller for new sketch
    frame_controller::ensure_controller(sketch_info.config.fps);

    if sketch_info.config.play_mode != PlayMode::Loop {
        frame_controller::set_paused(true);
    }

    // Clear any existing state
    model.clear_flag.set(true);

    model.alert_text = format!("Switched to {}", sketch_info.display_name);
}

fn view(app: &App, model: &DynamicModel, frame: Frame) {
    // Clear if requested
    if model.clear_flag.get() {
        frame.clear(model.current_sketch.clear_color());
        model.clear_flag.set(false);
    }

    // Use the frame controller for consistent rendering
    frame_controller::wrapped_view(
        app,
        &model.current_sketch,
        frame,
        |app, sketch, frame| sketch.view(app, frame),
    );
}

fn view_gui(app: &App, model: &DynamicModel, frame: Frame) {
    model.egui.draw_to_frame(&frame).unwrap();
}

fn event(app: &App, model: &mut DynamicModel, event: Event) {
    // Pass events to current sketch
    model.current_sketch.event(app, &event);

    // Handle window management events
    match event {
        Event::WindowEvent {
            id,
            simple: Some(KeyPressed(key)),
            ..
        } if id == model.main_window_id => {
            handle_key_event(app, model, key);
        }
        _ => {}
    }
}

fn handle_key_event(app: &App, model: &mut DynamicModel, key: Key) {
    match key {
        Key::A => {
            // Advance single frame when paused
            if frame_controller::is_paused() {
                frame_controller::advance_single_frame();
            }
        }
        Key::C => {
            // Toggle control panel visibility
            if let Some(window) = app.window(model.gui_window_id) {
                let visible = model.gui_visible.get();
                window.set_visible(!visible);
                model.gui_visible.set(!visible);
            }
        }
        Key::F => {
            // Toggle fullscreen
            if let Some(window) = app.window(model.main_window_id) {
                if model.main_maximized.get() {
                    window.set_inner_size_points(
                        model.current_sketch_config.w as f32,
                        model.current_sketch_config.h as f32,
                    );
                    model.main_maximized.set(false);
                } else {
                    if let Some(monitor) = window.current_monitor() {
                        let monitor_size = monitor.size();
                        window.set_inner_size_pixels(
                            monitor_size.width,
                            monitor_size.height,
                        );
                        model.main_maximized.set(true);
                    }
                }
            }
        }
        _ => {}
    }
}

// Placeholder for a UUID function - you'll need to implement this
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("session-{}", now)
}

// Placeholder for drawing controls - you'll need to replace with your implementation
fn draw_controls(controls: &Controls, ui: &mut egui::Ui) -> bool {
    let mut changed = false;

    for control in controls.get_controls() {
        match control {
            Control::Slider {
                name,
                value,
                min,
                max,
                step,
                ..
            } => {
                let mut current_value = *value;
                if ui
                    .add(
                        egui::Slider::new(&mut current_value, *min..*max)
                            .text(name),
                    )
                    .changed()
                {
                    controls
                        .update_value(name, ControlValue::Float(current_value));
                    changed = true;
                }
            }
            Control::Checkbox { name, value, .. } => {
                let mut current_value = *value;
                if ui.checkbox(&mut current_value, name).changed() {
                    controls
                        .update_value(name, ControlValue::Bool(current_value));
                    changed = true;
                }
            }
            // Implement other control types as needed
            _ => {}
        }
    }

    changed
}
