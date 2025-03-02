use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

#[proc_macro_derive(LegacySketchComponents, attributes(sketch))]
pub fn legacy_sketch_components(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let attrs = &ast.attrs;
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!(
            "Legacy SketchComponents only works on structs with named fields"
        ),
    };

    let mut clear_color = None;
    for attr in attrs {
        if attr.path().is_ident("sketch") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("clear_color") {
                    if let Ok(value) = meta.value() {
                        if let Ok(lit_str) = value.parse::<syn::LitStr>() {
                            if let Some(color_format) =
                                ColorFormat::from_str(&lit_str.value())
                            {
                                clear_color = Some(color_format);
                            }
                        }
                    }
                }
                Ok(())
            })
            .unwrap_or_else(|_| panic!("failed to parse sketch attribute"));
        }
    }

    let window_rect_impl = {
        let has_window_rect = fields
            .iter()
            .any(|f| f.ident.as_ref().unwrap() == "window_rect");
        let has_wr = fields.iter().any(|f| f.ident.as_ref().unwrap() == "wr");

        if has_window_rect {
            Some(quote! {
                fn window_rect(&mut self) -> Option<&mut WindowRect> {
                    Some(&mut self.window_rect)
                }
            })
        } else if has_wr {
            Some(quote! {
                fn window_rect(&mut self) -> Option<&mut WindowRect> {
                    Some(&mut self.wr)
                }
            })
        } else {
            None
        }
    };

    let controls_impl = if fields
        .iter()
        .any(|f| f.ident.as_ref().unwrap() == "controls")
    {
        Some(quote! {
            fn controls(&mut self) -> Option<&mut dyn ControlProvider> {
                Some(&mut self.controls)
            }
        })
    } else {
        None
    };

    let clear_color_impl = clear_color.map(|color| match color {
        ColorFormat::Rgba(components) => {
            let [r, g, b, a] =
                [components[0], components[1], components[2], components[3]];
            quote! {
                fn clear_color(&self) -> Rgba {
                    Rgba::new(#r, #g, #b, #a)
                }
            }
        }
        ColorFormat::Hsla(components) => {
            let [h, s, l, a] =
                [components[0], components[1], components[2], components[3]];
            quote! {
                fn clear_color(&self) -> Rgba {
                    hsla(#h, #s, #l, #a).into()
                }
            }
        }
    });

    let gen = quote! {
        impl Sketch for #name {
            fn update(&mut self, app: &App, update: Update, ctx: &LatticeContext) {
                panic!(
                    "update() not implemented. \
                    Structs that derive SketchComponents must still provide update \
                    and view functions."
                )
            }

            fn view(&self, app: &App, frame: Frame, ctx: &LatticeContext) {
                panic!(
                    "view() not implemented. \
                    Structs that derive SketchComponents must still provide update \
                    and view functions."
                )
            }
        }

        impl SketchDerived for #name {
            #window_rect_impl
            #controls_impl
            #clear_color_impl
        }
    };

    gen.into()
}

#[proc_macro_derive(SketchComponents, attributes(sketch))]
pub fn sketch_components(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let attrs = &ast.attrs;
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!(
            "Legacy SketchComponents only works on structs with named fields"
        ),
    };

    let mut clear_color = None;
    for attr in attrs {
        if attr.path().is_ident("sketch") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("clear_color") {
                    if let Ok(value) = meta.value() {
                        if let Ok(lit_str) = value.parse::<syn::LitStr>() {
                            if let Some(color_format) =
                                ColorFormat::from_str(&lit_str.value())
                            {
                                clear_color = Some(color_format);
                            }
                        }
                    }
                }
                Ok(())
            })
            .unwrap_or_else(|_| panic!("failed to parse sketch attribute"));
        }
    }

    let controls_impl = if fields
        .iter()
        .any(|f| f.ident.as_ref().unwrap() == "controls")
    {
        quote! { Some(&mut self.controls) }
    } else {
        quote! { None }
    };

    let clear_color_impl = clear_color
        .map(|color| match color {
            ColorFormat::Rgba(components) => {
                let [r, g, b, a] = [
                    components[0],
                    components[1],
                    components[2],
                    components[3],
                ];
                quote! { Rgba::new(#r, #g, #b, #a) }
            }
            ColorFormat::Hsla(components) => {
                let [h, s, l, a] = [
                    components[0],
                    components[1],
                    components[2],
                    components[3],
                ];
                quote! { hsla(#h, #s, #l, #a).into() }
            }
        })
        .unwrap_or_else(|| quote! { Rgba::new(0.0, 0.0, 0.0, 0.0) });

    let gen = quote! {
        impl Sketch for #name {
            fn controls(&mut self) -> Option<&mut dyn ControlProvider> {
                #controls_impl
            }

            fn clear_color(&self) -> Rgba {
                #clear_color_impl
            }
        }
    };

    gen.into()
}

enum ColorFormat {
    Rgba(Vec<f32>),
    Hsla(Vec<f32>),
}

impl ColorFormat {
    fn from_str(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('(').collect();
        if parts.len() != 2 {
            return None;
        }

        let format = parts[0].trim();
        let values = parts[1].trim_end_matches(')');
        let numbers: Vec<f32> = values
            .split(',')
            .map(|s| s.trim().parse::<f32>())
            .collect::<Result<_, _>>()
            .ok()?;

        match format {
            "rgba" => {
                if numbers.len() == 4 {
                    Some(ColorFormat::Rgba(numbers))
                } else {
                    None
                }
            }
            "hsla" => {
                if numbers.len() == 4 {
                    Some(ColorFormat::Hsla(numbers))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
