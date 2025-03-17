use std::str::FromStr;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

pub fn sketch_components_impl(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let attrs = &ast.attrs;
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("SketchComponents only works on structs with named fields"),
    };

    let mut clear_color = None;
    for attr in attrs {
        if attr.path().is_ident("sketch") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("clear_color") {
                    if let Ok(value) = meta.value() {
                        if let Ok(lit_str) = value.parse::<syn::LitStr>() {
                            if let Ok(color_format) =
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

    let has_hub = fields.iter().any(|f| {
        let ident = f.ident.as_ref().unwrap();
        ident == "hub"
    });

    let has_controls = fields.iter().any(|f| {
        let ident = f.ident.as_ref().unwrap();
        ident == "controls"
    });

    let controls_impl = if has_hub {
        quote! { Some(&mut self.hub) }
    } else if has_controls {
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
        impl SketchDerived for #name {
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

impl FromStr for ColorFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('(').collect();
        if parts.len() != 2 {
            return Err("Invalid color format".to_string());
        }

        let format = parts[0].trim();
        let values = parts[1].trim_end_matches(')');
        let numbers: Vec<f32> = values
            .split(',')
            .map(|s| s.trim().parse::<f32>())
            .collect::<Result<_, _>>()
            .map_err(|_| "Invalid numeric values".to_string())?;

        match format {
            "rgba" => {
                if numbers.len() == 4 {
                    Ok(ColorFormat::Rgba(numbers))
                } else {
                    Err("Incorrect number of components for rgba".to_string())
                }
            }
            "hsla" => {
                if numbers.len() == 4 {
                    Ok(ColorFormat::Hsla(numbers))
                } else {
                    Err("Incorrect number of components for hsla".to_string())
                }
            }
            _ => Err(format!(
                "Unsupported color format: '{}'. \
                    Use 'rgba(r,g,b,a)' or 'hsla(h,s,l,a)'",
                format
            )),
        }
    }
}
