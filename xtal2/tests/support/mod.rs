use std::env;

pub fn gpu_tests_enabled() -> bool {
    matches!(
        env::var("XTAL2_RUN_GPU_TESTS")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}
