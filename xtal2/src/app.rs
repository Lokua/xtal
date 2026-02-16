use std::sync::Arc;

use log::{error, info, warn};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::context::Context;
use crate::controls::{
    ControlDefaults, ControlScriptWatcher, resolve_control_script_path,
};
use crate::frame::Frame;
use crate::gpu::CompiledGraph;
use crate::graph::GraphBuilder;
use crate::sketch::{FullscreenShaderSketch, Sketch, SketchConfig};
use crate::uniforms::UniformBanks;

pub fn run<S: Sketch + 'static>(
    config: &'static SketchConfig,
    sketch: S,
) -> Result<(), String> {
    run_inner(config, sketch, None)
}

fn run_inner<S: Sketch + 'static>(
    config: &'static SketchConfig,
    sketch: S,
    control_script: Option<ControlScriptRuntime>,
) -> Result<(), String> {
    let _ = env_logger::try_init();

    let event_loop = EventLoop::new().map_err(|err| err.to_string())?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut runner = Runner::new(config, sketch, control_script);
    event_loop
        .run_app(&mut runner)
        .map_err(|err| err.to_string())
}

pub fn run_with_control_script<S: Sketch + 'static>(
    config: &'static SketchConfig,
    sketch: S,
    control_script_path: impl Into<std::path::PathBuf>,
) -> Result<(), String> {
    let control_script_path = resolve_control_script_path(control_script_path)?;
    let control_defaults = ControlDefaults::load(&control_script_path)?;
    let watcher = ControlScriptWatcher::start(control_script_path.clone())
        .map_err(|err| {
            format!(
                "failed to watch control script '{}': {}",
                control_script_path.display(),
                err
            )
        })?;

    let control_script = ControlScriptRuntime {
        path: control_script_path,
        defaults: control_defaults,
        watcher,
    };

    run_inner(config, sketch, Some(control_script))
}

pub fn run_fullscreen_shader(
    config: &'static SketchConfig,
    shader_path: impl Into<std::path::PathBuf>,
    control_script_path: impl Into<std::path::PathBuf>,
) -> Result<(), String> {
    let sketch = FullscreenShaderSketch::new(shader_path);
    run_with_control_script(config, sketch, control_script_path)
}

struct ControlScriptRuntime {
    path: std::path::PathBuf,
    defaults: ControlDefaults,
    watcher: ControlScriptWatcher,
}

struct Runner<S: Sketch> {
    config: &'static SketchConfig,
    sketch: S,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    context: Option<Context>,
    uniforms: Option<UniformBanks>,
    graph: Option<CompiledGraph>,
    control_script: Option<ControlScriptRuntime>,
}

impl<S: Sketch> Runner<S> {
    fn new(
        config: &'static SketchConfig,
        sketch: S,
        control_script: Option<ControlScriptRuntime>,
    ) -> Self {
        Self {
            config,
            sketch,
            window: None,
            window_id: None,
            surface: None,
            surface_config: None,
            context: None,
            uniforms: None,
            graph: None,
            control_script,
        }
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

        let mut graph_builder = GraphBuilder::new();
        self.sketch.setup(&mut graph_builder);
        let graph_spec = graph_builder.build();

        let uniforms =
            UniformBanks::new(device.as_ref(), self.config.banks.max(1));

        let graph = CompiledGraph::compile(
            device.as_ref(),
            queue.as_ref(),
            format,
            graph_spec,
            uniforms.bind_group_layout(),
        )?;

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
        self.uniforms = Some(uniforms);
        self.graph = Some(graph);

        Ok(())
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

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        self.reload_control_script_if_changed();

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
        let control_defaults =
            self.control_script.as_ref().map(|script| &script.defaults);

        self.sketch.update(context);

        let [w, h] = context.resolution();
        uniforms.set_resolution(w, h);
        uniforms.set_beats(context.elapsed_seconds());
        apply_control_defaults(control_defaults, uniforms);
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
    }

    fn reload_control_script_if_changed(&mut self) {
        let Some(control_script) = self.control_script.as_mut() else {
            return;
        };

        if !control_script.watcher.take_changed() {
            return;
        }

        match ControlDefaults::load(&control_script.path) {
            Ok(defaults) => {
                let defaults_summary = defaults
                    .values()
                    .iter()
                    .map(|v| format!("{}={:.3}", v.id, v.value))
                    .collect::<Vec<_>>()
                    .join(", ");

                control_script.defaults = defaults;
                info!(
                    "reloaded control script: {} [{}]",
                    control_script.path.display(),
                    defaults_summary
                );
            }
            Err(err) => {
                warn!(
                    "failed to reload control script '{}': {}",
                    control_script.path.display(),
                    err
                );
            }
        }
    }
}

impl<S: Sketch> ApplicationHandler for Runner<S> {
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
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => self.resize(new_size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(context) = self.context.as_mut() {
                    context.set_scale_factor(scale_factor);
                }
            }
            WindowEvent::RedrawRequested => self.render(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
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

fn apply_control_defaults(
    control_defaults: Option<&ControlDefaults>,
    uniforms: &mut UniformBanks,
) {
    let Some(control_defaults) = control_defaults else {
        return;
    };

    for control in control_defaults.values() {
        if matches!(control.id.as_str(), "a1" | "a2" | "a3") {
            continue;
        }

        if let Err(err) = uniforms.set(&control.id, control.value) {
            warn!("ignoring control '{}': {}", control.id, err);
        }
    }
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
