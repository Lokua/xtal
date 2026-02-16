#[macro_export]
macro_rules! register_sketches {
    (
        $(
            {
                title: $title:expr,
                enabled: $enabled:expr,
                sketches: [$($module:ident),* $(,)?]
            }
        ),+ $(,)?
    ) => {{
        (|| -> Result<$crate::runtime::registry::RuntimeRegistry, String> {
            let mut __registry =
                $crate::runtime::registry::RuntimeRegistry::new();

            $(
                let mut __category_sketches = Vec::new();
                $(
                    __registry.register(
                        &$module::SKETCH_CONFIG,
                        || Box::new($module::init()),
                    )?;
                    __category_sketches
                        .push($module::SKETCH_CONFIG.name.to_string());
                )*

                __registry.define_category(
                    $title,
                    $enabled,
                    __category_sketches,
                )?;
            )+

            Ok(__registry)
        })()
    }};
}
