use std::sync::Arc;
use std::time::Instant;

use log::{error, info, warn};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

use super::events::{
    RuntimeCommand, RuntimeCommandReceiver, RuntimeEvent, RuntimeEventSender,
    command_channel, event_channel,
};
use super::frame_clock::FrameClock;
use super::registry::RuntimeRegistry;
use super::web_view;
use super::web_view_bridge::WebViewBridge;
use crate::context::Context;
use crate::control::{ControlCollection, ControlHub, ControlValue};
use crate::frame::Frame;
use crate::framework::{frame_controller, logging};
use crate::gpu::CompiledGraph;
use crate::graph::GraphBuilder;
use crate::motion::{Bpm, Timing};
use crate::sketch::{Sketch, SketchConfig, TimingMode};
use crate::uniforms::UniformBanks;

pub fn run_registry(
    registry: RuntimeRegistry,
    initial_sketch: Option<&str>,
) -> Result<(), String> {
    let (_, command_rx) = command_channel();
    run_registry_with_channels(registry, initial_sketch, command_rx, None)
}

pub fn run_registry_with_web_view(
    registry: RuntimeRegistry,
    initial_sketch: Option<&str>,
) -> Result<(), String> {
    let (command_tx, command_rx) = command_channel();
    let (event_tx, event_rx) = event_channel();

    let _bridge = WebViewBridge::launch(command_tx, event_rx)?;

    run_registry_with_channels(
        registry,
        initial_sketch,
        command_rx,
        Some(event_tx),
    )
}

pub fn run_registry_with_channels(
    registry: RuntimeRegistry,
    initial_sketch: Option<&str>,
    command_rx: RuntimeCommandReceiver,
    event_tx: Option<RuntimeEventSender>,
) -> Result<(), String> {
    logging::init_logger();

    let event_loop = EventLoop::new().map_err(|err| err.to_string())?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut runner =
        RegistryRunner::new(registry, initial_sketch, command_rx, event_tx)?;

    event_loop
        .run_app(&mut runner)
        .map_err(|err| err.to_string())
}

struct RegistryRunner {
    registry: RuntimeRegistry,
    active_sketch_name: String,
    config: &'static SketchConfig,
    sketch: Box<dyn Sketch>,
    frame_clock: FrameClock,
    render_requested: bool,
    command_rx: RuntimeCommandReceiver,
    event_tx: Option<RuntimeEventSender>,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    context: Option<Context>,
    uniforms: Option<UniformBanks>,
    graph: Option<CompiledGraph>,
    control_hub: Option<ControlHub<Timing>>,
    perf_mode: bool,
}

impl RegistryRunner {
    fn new(
        registry: RuntimeRegistry,
        initial_sketch: Option<&str>,
        command_rx: RuntimeCommandReceiver,
        event_tx: Option<RuntimeEventSender>,
    ) -> Result<Self, String> {
        let active_name =
            select_initial_sketch_name(&registry, initial_sketch)?;

        let (config, sketch) = instantiate_sketch(&registry, &active_name)
            .map_err(|err| {
                format!(
                    "failed to initialize sketch '{}': {}",
                    active_name, err
                )
            })?;

        Ok(Self {
            registry,
            active_sketch_name: active_name,
            config,
            sketch,
            frame_clock: FrameClock::new(config.fps),
            render_requested: false,
            command_rx,
            event_tx,
            window: None,
            window_id: None,
            surface: None,
            surface_config: None,
            context: None,
            uniforms: None,
            graph: None,
            control_hub: None,
            perf_mode: false,
        })
    }

    fn init_runtime(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), String> {
        let attrs = WindowAttributes::default()
            .with_title(self.config.display_name)
            .with_inner_size(LogicalSize::new(self.config.w, self.config.h));

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .map_err(|err| err.to_string())?,
        );
        anchor_window_top_left(window.as_ref());

        let instance =
            wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let surface = instance
            .create_surface(window.clone())
            .map_err(|err| err.to_string())?;

        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            },
        ))
        .map_err(|err| err.to_string())?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("xtal2-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::default(),
            },
        ))
        .map_err(|err| err.to_string())?;

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let caps = surface.get_capabilities(&adapter);
        let format = choose_surface_format(&caps.formats)
            .ok_or_else(|| "surface has no supported formats".to_string())?;

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let context = Context::new(
            device.clone(),
            queue.clone(),
            [width, height],
            window.scale_factor(),
        );

        self.window_id = Some(window.id());
        self.window = Some(window);
        self.surface = Some(surface);
        self.surface_config = Some(surface_config);
        self.context = Some(context);

        self.rebuild_graph_state()?;

        Ok(())
    }

    fn rebuild_graph_state(&mut self) -> Result<(), String> {
        let Some(context) = self.context.as_ref() else {
            return Err("runtime context not initialized".to_string());
        };

        let Some(surface_config) = self.surface_config.as_ref() else {
            return Err("surface config not initialized".to_string());
        };

        let mut graph_builder = GraphBuilder::new();
        self.sketch.setup(&mut graph_builder);
        let graph_spec = graph_builder.build();

        let mut uniforms = UniformBanks::new(
            context.device.as_ref(),
            self.config.banks.max(1),
        );
        self.apply_sketch_defaults(&mut uniforms);

        self.control_hub = self.build_control_hub();

        let graph = CompiledGraph::compile(
            context.device.as_ref(),
            context.queue.as_ref(),
            surface_config.format,
            graph_spec,
            uniforms.bind_group_layout(),
        )?;

        self.uniforms = Some(uniforms);
        self.graph = Some(graph);

        Ok(())
    }

    fn apply_sketch_defaults(&self, uniforms: &mut UniformBanks) {
        for (id, value) in self.sketch.default_uniforms() {
            if let Err(err) = uniforms.set(id, *value) {
                warn!("invalid sketch default uniform '{}': {}", id, err);
            }
        }
    }

    fn build_control_hub(&self) -> Option<ControlHub<Timing>> {
        let path = self.sketch.control_script()?;

        if !path.exists() {
            warn!(
                "control script for sketch '{}' does not exist: {}",
                self.config.name,
                path.display()
            );
            return None;
        }

        let bpm = Bpm::new(self.config.bpm);
        let timing = match self.sketch.timing_mode() {
            TimingMode::Frame => Timing::frame(bpm),
            TimingMode::Osc => Timing::osc(bpm),
            TimingMode::Midi => Timing::midi(bpm),
            TimingMode::Hybrid => Timing::hybrid(bpm),
            TimingMode::Manual => Timing::manual(bpm),
        };

        let mut hub = ControlHub::from_path(path, timing);
        // Until UI/state sync is wired, prefer control script defaults on
        // reload so edits are immediately visible during sketching.
        hub.set_preserve_values_on_reload(false);
        hub.mark_unchanged();
        Some(hub)
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        let Some(surface_config) = self.surface_config.as_mut() else {
            return;
        };
        let Some(context) = self.context.as_mut() else {
            return;
        };

        surface_config.width = new_size.width;
        surface_config.height = new_size.height;

        surface.configure(context.device.as_ref(), surface_config);
        context.set_window_size([new_size.width, new_size.height]);
    }

    fn emit_event(&self, event: RuntimeEvent) {
        let Some(event_tx) = self.event_tx.as_ref() else {
            return;
        };

        if let Err(err) = event_tx.send(event) {
            warn!("failed to emit runtime event: {}", err);
        }
    }

    fn emit_web_view_event(&self, event: web_view::Event) {
        self.emit_event(RuntimeEvent::WebView(event));
    }

    fn emit_web_view_init(&self) {
        let event = web_view::Event::Init {
            audio_device: String::new(),
            audio_devices: vec![],
            hrcc: false,
            images_dir: String::new(),
            is_light_theme: true,
            mappings_enabled: false,
            midi_clock_port: String::new(),
            midi_input_port: String::new(),
            midi_output_port: String::new(),
            midi_input_ports: vec![],
            midi_output_ports: vec![],
            osc_port: 0,
            sketch_names: self.registry.sketch_names().to_vec(),
            sketch_catalog: Some(web_view::sketch_catalog_from_registry(
                &self.registry,
            )),
            sketch_name: self.active_sketch_name.clone(),
            transition_time: 4.0,
            user_data_dir: String::new(),
            videos_dir: String::new(),
        };

        self.emit_web_view_event(event);
    }

    fn emit_web_view_load_sketch(&self) {
        let controls = self
            .control_hub
            .as_ref()
            .map_or_else(Vec::new, web_view::controls_from_hub);

        let bypassed = self
            .control_hub
            .as_ref()
            .map_or_else(Default::default, ControlHub::bypassed);

        let snapshot_slots = self
            .control_hub
            .as_ref()
            .map_or_else(Vec::new, ControlHub::snapshot_keys_sorted);

        let snapshot_sequence_enabled = self
            .control_hub
            .as_ref()
            .is_some_and(ControlHub::snapshot_sequence_enabled);

        let event = web_view::Event::LoadSketch {
            bpm: self.config.bpm,
            bypassed,
            controls,
            display_name: self.config.display_name.to_string(),
            fps: self.config.fps,
            mappings: Default::default(),
            paused: self.frame_clock.paused(),
            perf_mode: self.perf_mode,
            sketch_name: self.active_sketch_name.clone(),
            sketch_width: self.config.w as i32,
            sketch_height: self.config.h as i32,
            snapshot_slots,
            snapshot_sequence_enabled,
            tap_tempo_enabled: false,
            exclusions: vec![],
        };

        self.emit_web_view_event(event);
    }

    fn apply_control_update(&mut self, name: String, value: ControlValue) {
        let Some(hub) = self.control_hub.as_mut() else {
            warn!(
                "ignoring control update for '{}' because no control hub is active",
                name
            );
            return;
        };

        hub.ui_controls.set(&name, value);
        self.frame_clock.advance_single_frame();

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn switch_sketch(&mut self, name: &str) -> Result<(), String> {
        let (config, sketch) = instantiate_sketch(&self.registry, name)?;

        self.active_sketch_name = name.to_string();
        self.config = config;
        self.sketch = sketch;
        self.frame_clock.set_fps(self.config.fps);
        frame_controller::set_fps(self.config.fps);

        if let Some(window) = self.window.as_ref() {
            window.set_title(self.config.display_name);
            if !self.perf_mode {
                anchor_window_top_left(window.as_ref());
                let _ = window.request_inner_size(LogicalSize::new(
                    self.config.w,
                    self.config.h,
                ));
            }
            window.request_redraw();
        }

        if !self.perf_mode {
            self.resize(winit::dpi::PhysicalSize::new(
                self.config.w.max(1),
                self.config.h.max(1),
            ));
        }

        self.rebuild_graph_state()?;

        info!(
            "switched sketch to '{}' ({})",
            self.active_sketch_name, self.config.display_name
        );

        self.emit_event(RuntimeEvent::SketchSwitched(
            self.active_sketch_name.clone(),
        ));
        self.emit_web_view_load_sketch();

        Ok(())
    }

    fn set_perf_mode(&mut self, perf_mode: bool) {
        if self.perf_mode == perf_mode {
            self.emit_web_view_event(web_view::Event::PerfMode(perf_mode));
            return;
        }

        self.perf_mode = perf_mode;
        info!("performance mode set to {}", self.perf_mode);

        if let Some(window) = self.window.as_ref() {
            if !self.perf_mode {
                anchor_window_top_left(window.as_ref());
                let _ = window.request_inner_size(LogicalSize::new(
                    self.config.w,
                    self.config.h,
                ));
            }

            window.request_redraw();
        }

        if !self.perf_mode {
            self.resize(winit::dpi::PhysicalSize::new(
                self.config.w.max(1),
                self.config.h.max(1),
            ));
        }

        self.emit_web_view_event(web_view::Event::PerfMode(perf_mode));
        self.emit_web_view_load_sketch();
    }

    fn toggle_fullscreen(&self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        if window.fullscreen().is_some() {
            window.set_fullscreen(None);
            return;
        }

        let monitor = window.current_monitor();
        window.set_fullscreen(Some(Fullscreen::Borderless(monitor)));
    }

    fn focus_main_window(&self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        window.set_visible(true);
        window.focus_window();
    }

    fn process_commands(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(command) = self.command_rx.try_recv() {
            match command {
                RuntimeCommand::AdvanceSingleFrame => {
                    self.frame_clock.advance_single_frame();
                }
                RuntimeCommand::Pause(paused) => {
                    self.frame_clock.set_paused(paused);
                    frame_controller::set_paused(paused);
                    self.emit_web_view_event(web_view::Event::Paused(paused));
                }
                RuntimeCommand::SetPerfMode(perf_mode) => {
                    self.set_perf_mode(perf_mode);
                }
                RuntimeCommand::Quit => {
                    self.emit_event(RuntimeEvent::Stopped);
                    event_loop.exit();
                    return;
                }
                RuntimeCommand::ReloadControls => {
                    if let Some(hub) = self.control_hub.as_ref() {
                        hub.request_reload();
                    }
                }
                RuntimeCommand::SwitchSketch(name) => {
                    if let Err(err) = self.switch_sketch(&name) {
                        error!("failed to switch sketch '{}': {}", name, err);
                    }
                }
                RuntimeCommand::ToggleFullScreen => {
                    self.toggle_fullscreen();
                }
                RuntimeCommand::ToggleMainFocus => {
                    self.focus_main_window();
                }
                RuntimeCommand::UpdateControlBool { name, value } => {
                    self.apply_control_update(name, ControlValue::Bool(value));
                }
                RuntimeCommand::UpdateControlFloat { name, value } => {
                    self.apply_control_update(name, ControlValue::Float(value));
                }
                RuntimeCommand::UpdateControlString { name, value } => {
                    self.apply_control_update(
                        name,
                        ControlValue::String(value),
                    );
                }
            }
        }
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        if !self.render_requested {
            return;
        }

        self.render_requested = false;

        let Some(context) = self.context.as_mut() else {
            return;
        };
        let Some(uniforms) = self.uniforms.as_mut() else {
            return;
        };
        let Some(graph) = self.graph.as_mut() else {
            return;
        };
        let Some(surface_config) = self.surface_config.as_ref() else {
            return;
        };

        frame_controller::set_frame_count(self.frame_clock.frame_count() as u32);

        self.sketch.update(context);

        let [w, h] = context.resolution();
        uniforms.set_resolution(w, h);

        let mut web_view_events = Vec::new();

        if let Some(hub) = self.control_hub.as_mut() {
            hub.update();

            for (id, value) in hub.var_values() {
                if let Err(err) = uniforms.set(&id, value) {
                    warn!(
                        "ignoring control var '{}' for sketch '{}': {}",
                        id, self.config.name, err
                    );
                }
            }

            if hub.changed() {
                let controls = web_view::controls_from_hub(hub);
                let bypassed = hub.bypassed();
                let snapshot_sequence_enabled = hub.snapshot_sequence_enabled();

                web_view_events.push(web_view::Event::HubPopulated((
                    controls.clone(),
                    bypassed,
                )));
                web_view_events
                    .push(web_view::Event::UpdatedControls(controls));
                web_view_events.push(web_view::Event::SnapshotSequenceEnabled(
                    snapshot_sequence_enabled,
                ));

                hub.mark_unchanged();
            }

            uniforms.set_beats(hub.beats());
        } else {
            uniforms.set_beats(context.elapsed_seconds());
        }

        uniforms.upload(context.queue.as_ref());

        let Some(surface) = self.surface.as_mut() else {
            return;
        };

        let output = match surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                surface.configure(context.device.as_ref(), surface_config);
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => {
                warn!("surface timeout while acquiring frame");
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                error!("surface out of memory; exiting");
                event_loop.exit();
                return;
            }
            Err(wgpu::SurfaceError::Other) => {
                warn!("surface error while acquiring frame");
                return;
            }
        };

        let mut frame =
            Frame::new(context.device.as_ref(), context.queue.clone(), output);

        self.sketch.view(&mut frame, context);

        if let Err(err) = graph.execute(
            context.device.as_ref(),
            &mut frame,
            uniforms,
            context.resolution_u32(),
        ) {
            error!("graph execution error: {}", err);
            event_loop.exit();
            return;
        }

        frame.submit();
        context.next_frame();
        self.emit_event(RuntimeEvent::FrameAdvanced(
            self.frame_clock.frame_count(),
        ));

        for event in web_view_events {
            self.emit_web_view_event(event);
        }
    }
}

impl ApplicationHandler for RegistryRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        if let Err(err) = self.init_runtime(event_loop) {
            error!("failed to initialize xtal2 runtime: {}", err);
            event_loop.exit();
            return;
        }

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }

        frame_controller::set_fps(self.config.fps);
        self.emit_web_view_init();
        self.emit_web_view_load_sketch();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window_id != Some(window_id) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.emit_event(RuntimeEvent::Stopped);
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => self.resize(new_size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(context) = self.context.as_mut() {
                    context.set_scale_factor(scale_factor);
                }
            }
            WindowEvent::RedrawRequested => {
                self.process_commands(event_loop);
                self.render(event_loop);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_commands(event_loop);

        let tick = self.frame_clock.tick(Instant::now());

        if tick.should_render {
            self.render_requested = true;
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        } else {
            self.emit_event(RuntimeEvent::FrameSkipped);
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            self.frame_clock.next_deadline(),
        ));
    }
}

fn select_initial_sketch_name(
    registry: &RuntimeRegistry,
    initial_sketch: Option<&str>,
) -> Result<String, String> {
    if let Some(initial_sketch) = initial_sketch {
        if registry.get(initial_sketch).is_some() {
            return Ok(initial_sketch.to_string());
        }

        warn!(
            "requested initial sketch '{}' does not exist; falling back",
            initial_sketch
        );
    }

    registry
        .first_sketch_name()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "runtime registry is empty".to_string())
}

fn instantiate_sketch(
    registry: &RuntimeRegistry,
    name: &str,
) -> Result<(&'static SketchConfig, Box<dyn Sketch>), String> {
    let Some(entry) = registry.get(name) else {
        return Err(format!("sketch '{}' is not registered", name));
    };

    let config = entry.config;
    let sketch = (entry.factory)();

    Ok((config, sketch))
}

fn choose_surface_format(
    formats: &[wgpu::TextureFormat],
) -> Option<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(|f| *f == wgpu::TextureFormat::Bgra8UnormSrgb)
        .or_else(|| formats.first().copied())
}

fn anchor_window_top_left(window: &Window) {
    let Some(monitor) = window.current_monitor() else {
        return;
    };

    let monitor_origin = monitor.position();
    let x = monitor_origin.x;
    let y = monitor_origin.y;

    window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
}
