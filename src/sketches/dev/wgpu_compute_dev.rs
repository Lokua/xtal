use nannou::prelude::*;
use std::sync::{Arc, Mutex};

use crate::framework::prelude::*;

pub const SKETCH_CONFIG: SketchConfig = SketchConfig {
    name: "wgpu_compute_dev",
    display_name: "WGPU Compute Test",
    play_mode: PlayMode::Loop,
    fps: 60.0,
    bpm: 134.0,
    w: 700,
    h: 700,
    gui_w: None,
    gui_h: Some(300),
};

const MAX_POINTS: u32 = 5_000_000;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeParams {
    n_segments: u32,
    points_per_segment: u32,
    noise_scale: f32,
    angle_variation: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct InputPoint {
    pos: [f32; 2],
    // The _padding fields are needed because WGPU/WebGPU requires struct members to be
    // aligned to 8-byte boundaries for storage buffers.
    // Without padding, our vec2 (two f32s = 8 bytes) would make the struct 8 bytes,
    // but the next struct in the array would start at byte 8,
    // violating the 16-byte alignment requirement.
    // We could avoid it by using vec4 instead:
    //      `pos: [f32; 4]`
    // Using all 4 components naturally aligns to 16 bytes
    _padding: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct OutputPoint {
    pos: [f32; 2],
    _padding: [f32; 2],
}

#[derive(LegacySketchComponents)]
pub struct Model {
    animation: Animation<Timing>,
    controls: Controls,
    wr: WindowRect,
    compute_pipeline: wgpu::ComputePipeline,
    params_buffer: wgpu::Buffer,
    params_bind_group: wgpu::BindGroup,
    input_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,
    reference_points: Vec<InputPoint>,
    computed_points: Arc<Mutex<Vec<OutputPoint>>>,
}

pub fn init_model(app: &App, wr: WindowRect) -> Model {
    let animation = Animation::new(Timing::new(SKETCH_CONFIG.bpm));

    let controls = Controls::with_previous(vec![
        Control::checkbox("show_ref_line", false),
        Control::checkbox("show_sand_line", true),
        Control::slider("noise_scale", 0.02, (0.001, 0.1), 0.001),
        Control::slider("angle_variation", 0.2, (0.0, 1.0), 0.0001),
        Control::slider("points_per_segment", 100.0, (10.0, 500.0), 1.0),
        Control::slider("ref_segments", 4.0, (2.0, 20.0), 1.0),
        Control::slider("ref_deviation", 0.1, (0.0, 0.5), 0.0001),
    ]);

    let window = app.main_window();
    let device = window.device();

    // Create shader module
    let shader = wgpu::include_wgsl!("wgpu_compute_dev.wgsl");
    let compute_module = device.create_shader_module(shader);

    // Create compute pipeline
    let params_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // Compute parameters
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<ComputeParams>() as _,
                        ),
                    },
                    count: None,
                },
                // Input points
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output points
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: false,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("compute_bind_group_layout"),
        });

    let pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&params_bind_group_layout],
            push_constant_ranges: &[],
        });

    let compute_pipeline =
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Sand Line Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compute_module,
            entry_point: "main",
        });

    // Create initial parameters
    let params = ComputeParams {
        // n_segments: (reference_points.len() - 1) as u32,
        n_segments: 0,
        points_per_segment: 100,
        noise_scale: 0.02,
        angle_variation: 0.2,
    };

    // Create buffers
    let params_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Compute Params Buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    let reference_points = Vec::new();
    // Max ref_segments + 1
    let max_ref_points = 21;
    let empty_points = vec![
        InputPoint {
            pos: [0.0, 0.0],
            _padding: [0.0, 0.0]
        };
        max_ref_points
    ];
    let input_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input Points Buffer"),
            contents: bytemuck::cast_slice(&empty_points),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

    // Create output buffer with maximum possible size
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Points Buffer"),
        size: (MAX_POINTS * std::mem::size_of::<OutputPoint>() as u32) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create bind group
    let params_bind_group =
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &params_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
            label: Some("compute_bind_group"),
        });

    let computed_points = Arc::new(Mutex::new(vec![
        OutputPoint {
            pos: [0.0, 0.0],
            _padding: [0.0, 0.0]
        };
        MAX_POINTS as usize
    ]));

    Model {
        animation,
        controls,
        wr,
        compute_pipeline,
        params_buffer,
        params_bind_group,
        input_buffer,
        output_buffer,
        reference_points,
        computed_points,
    }
}

pub fn update(app: &App, m: &mut Model, _update: Update) {
    let segments = m.controls.float("ref_segments") as usize;
    let deviation = m.controls.float("ref_deviation");
    let points_per_segment = m.controls.float("points_per_segment") as u32;

    if m.controls.changed() {
        if m.controls
            .any_changed_in(&["ref_segments", "ref_deviation"])
        {
            m.reference_points = generate_reference_points(segments, deviation);
        }
        m.controls.mark_unchanged();
    }

    let n_segments = (m.reference_points.len() - 1) as u32;

    let (ns_min, _ns_max) = m.controls.slider_range("noise_scale");
    let (ns_min, ns_max) = safe_range(ns_min, m.controls.float("noise_scale"));

    let (angle_min, _angle_max) = m.controls.slider_range("angle_variation");
    let (angle_min, angle_max) =
        safe_range(angle_min, m.controls.float("angle_variation"));

    let params = ComputeParams {
        n_segments,
        points_per_segment,
        noise_scale: map_range(m.animation.tri(8.0), 0.0, 1.0, ns_min, ns_max),
        angle_variation: map_range(
            m.animation.tri(3.0),
            0.0,
            1.0,
            angle_min,
            angle_max,
        ),
    };

    let window = app.main_window();
    let device = window.device();

    // Update parameters
    window.queue().write_buffer(
        &m.params_buffer,
        0,
        bytemuck::bytes_of(&params),
    );

    window.queue().write_buffer(
        &m.input_buffer,
        0,
        bytemuck::cast_slice(&m.reference_points),
    );

    let output_size = (n_segments
        * points_per_segment
        * std::mem::size_of::<OutputPoint>() as u32)
        as u64;

    // Make sure output_size doesn't exceed our output buffer's capacity
    let max_size =
        (MAX_POINTS * std::mem::size_of::<OutputPoint>() as u32) as u64;
    let output_size = output_size.min(max_size);

    // Create the read buffer
    let read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("read buffer"),
        size: output_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Create and submit compute pass
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder"),
        });

    {
        let mut compute_pass =
            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Sand Line Compute Pass"),
            });
        compute_pass.set_pipeline(&m.compute_pipeline);
        compute_pass.set_bind_group(0, &m.params_bind_group, &[]);
        compute_pass.dispatch_workgroups(
            ((params.n_segments * params.points_per_segment) as f32 / 64.0)
                .ceil() as u32,
            1,
            1,
        );
    }

    // Copy the compute output to our read buffer
    encoder.copy_buffer_to_buffer(
        &m.output_buffer,
        0,
        &read_buffer,
        0,
        output_size,
    );
    window.queue().submit(Some(encoder.finish()));

    // Read the result synchronously
    let slice = read_buffer.slice(..);

    // Create callback
    let computed_points = m.computed_points.clone();
    slice.map_async(wgpu::MapMode::Read, move |_| {});

    // Wait for GPU to finish
    device.poll(wgpu::Maintain::Wait);

    // Read the data
    {
        let data = slice.get_mapped_range();
        if let Ok(mut points) = computed_points.lock() {
            let new_points = bytemuck::cast_slice(&data);
            points[..new_points.len()].copy_from_slice(new_points);
        }
    }

    // Clean up
    read_buffer.unmap();
}

pub fn view(app: &App, m: &Model, frame: Frame) {
    let draw = app.draw();

    draw.rect()
        .x_y(0.0, 0.0)
        .w_h(m.wr.w(), m.wr.h())
        .hsla(0.0, 0.0, 1.0, 1.0);

    let show_ref_line = m.controls.bool("show_ref_line");
    let show_sand_line = m.controls.bool("show_sand_line");

    if show_ref_line {
        for i in 0..m.reference_points.len() - 1 {
            let start = vec2(
                m.reference_points[i].pos[0] * m.wr.w(),
                m.reference_points[i].pos[1] * m.wr.h(),
            );
            let end = vec2(
                m.reference_points[i + 1].pos[0] * m.wr.w(),
                m.reference_points[i + 1].pos[1] * m.wr.h(),
            );
            draw.line()
                .start(start)
                .end(end)
                .color(STEELBLUE)
                .weight(2.0);
        }
    }

    if show_sand_line {
        let num_points = (m.reference_points.len() - 1) as u32
            * m.controls.float("points_per_segment") as u32;

        if let Ok(points) = m.computed_points.lock() {
            for point in points.iter().take(num_points as usize) {
                let pos =
                    vec2(point.pos[0] * m.wr.w(), point.pos[1] * m.wr.h());
                draw.rect().xy(pos).w_h(1.0, 1.0).color(BLACK);
            }
        }
    }

    draw.to_frame(app, &frame).unwrap();
}

fn generate_reference_points(
    segments: usize,
    deviation: f32,
) -> Vec<InputPoint> {
    // Using NDC coordinates
    let start = vec2(-0.5, 0.0);
    let end = vec2(0.5, 0.0);
    let length = end.x - start.x;

    let points: Vec<InputPoint> = (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let x = start.x + length * t;
            let y = if i == 0 || i == segments {
                0.0
            } else {
                random_normal(deviation)
            };

            InputPoint {
                pos: [x, y],
                _padding: [0.0, 0.0],
            }
        })
        .collect();

    // Simple smoothing - average with neighbors
    let smoothed: Vec<InputPoint> = points
        .windows(3)
        .map(|window| {
            let prev = window[0].pos;
            let curr = window[1].pos;
            let next = window[2].pos;
            InputPoint {
                pos: [curr[0], (prev[1] + curr[1] + next[1]) / 3.0],
                _padding: [0.0, 0.0],
            }
        })
        .collect();

    // Add back first and last points
    let mut final_points = vec![points[0]];
    final_points.extend(smoothed);
    final_points.push(*points.last().unwrap());

    final_points
}
