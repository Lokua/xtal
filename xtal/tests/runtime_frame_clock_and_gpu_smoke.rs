mod support;

use std::time::Instant;

use xtal::time::frame_clock;

#[test]
fn frame_clock_scaffold_ticks() {
    let start = Instant::now();
    frame_clock::set_fps(60.0);
    frame_clock::set_paused(false);
    frame_clock::set_frame_count(0);
    frame_clock::reset_timing(start);
    let now = start + frame_clock::frame_duration();

    let tick = frame_clock::tick(now);
    assert!(tick.should_render);
    assert_eq!(tick.frames_advanced, 1);
}

#[test]
fn gpu_probe_is_opt_in() {
    if !support::gpu_tests_enabled() {
        eprintln!(
            "Skipping GPU smoke probe. Set XTAL_RUN_GPU_TESTS=1 to run."
        );
        return;
    }

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: true,
            compatible_surface: None,
        },
    ))
    .expect("expected a headless adapter for GPU smoke probe");

    let info = adapter.get_info();
    assert!(!info.name.is_empty());
}
