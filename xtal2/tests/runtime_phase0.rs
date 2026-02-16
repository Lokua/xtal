mod support;

use std::time::Instant;

use xtal2::runtime;
use xtal2::runtime::frame_clock::FrameClock;

#[test]
fn runtime_flavor_includes_xtal2() {
    let flavors = runtime::available_flavors();
    assert!(flavors.contains(&runtime::RuntimeFlavor::Xtal2));
}

#[cfg(all(feature = "xtal2", feature = "legacy_runtime"))]
#[test]
fn runtime_flavor_can_include_legacy_and_xtal2() {
    let flavors = runtime::available_flavors();
    assert!(flavors.contains(&runtime::RuntimeFlavor::Legacy));
    assert!(flavors.contains(&runtime::RuntimeFlavor::Xtal2));
}

#[test]
fn frame_clock_scaffold_ticks() {
    let start = Instant::now();
    let mut clock = FrameClock::with_start(60.0, start);
    let now = start + clock.frame_duration();

    let tick = clock.tick(now);
    assert!(tick.should_render);
    assert_eq!(tick.frames_advanced, 1);
}

#[test]
fn gpu_probe_is_opt_in() {
    if !support::gpu_tests_enabled() {
        eprintln!(
            "Skipping GPU smoke probe. Set XTAL2_RUN_GPU_TESTS=1 to run."
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
