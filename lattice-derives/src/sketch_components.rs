use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Fields, parse_macro_input};

pub fn sketch_components_impl(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("SketchComponents only works on structs with named fields"),
    };

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

    let generated = quote! {
        impl SketchDerived for #name {
            fn hub(&mut self) -> Option<&mut dyn ControlHubProvider> {
                #controls_impl
            }
        }
    };

    generated.into()
}
