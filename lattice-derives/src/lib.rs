use proc_macro::TokenStream;

mod sketch_components;
mod uniforms;

/// Saves sketches from the burden of having to manually implement the
/// `SketchDerived` trait which is required to integrate a sketch's controls and
/// animations usages with the UI
#[proc_macro_derive(SketchComponents, attributes(sketch))]
pub fn sketch_components(input: TokenStream) -> TokenStream {
    sketch_components::sketch_components_impl(input)
}

/// **⚠️ Experimental** and **UNSTABLE**
#[proc_macro_attribute]
pub fn uniforms(attr: TokenStream, item: TokenStream) -> TokenStream {
    uniforms::uniforms_impl(attr, item)
}
