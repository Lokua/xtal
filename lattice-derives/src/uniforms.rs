use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, DeriveInput, Ident, LitInt, Token,
};

pub fn uniforms_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let attrs = &input.attrs;
    let vis = &input.vis;
    let args = parse_macro_input!(attr as UniformsArgs);
    let banks = args.banks;
    let field_names = generate_field_names(banks);
    let struct_name = &input.ident;

    let first_bank = field_names.first().cloned();
    let remaining_banks =
        field_names.iter().skip(1).cloned().collect::<Vec<_>>();

    let set_match_arms = field_names.iter().map(|field_name| {
        quote! {
            stringify!(#field_name) => {
                let idx = bank
                    .chars()
                    .nth(1)
                    .unwrap_or('1')
                    .to_digit(10)
                    .unwrap_or(1) as usize - 1;

                if idx < 4 {
                    self.#field_name[idx] = value;
                }
            }
        }
    });

    let expanded_struct = quote! {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
        #(#attrs)*
        #vis struct #struct_name {
            #(#field_names: [f32; 4],)*
        }

        impl Default for #struct_name {
            fn default() -> Self {
                Self {
                    #(#field_names: [0.0; 4],)*
                }
            }
        }

        impl #struct_name {
            pub fn from_hub<T: TimingSource>(hub: &ControlHub<T>) -> Self {
                Self {
                    #(#field_names: [
                        hub.get(&format!("{}{}", stringify!(#field_names), 1)),
                        hub.get(&format!("{}{}", stringify!(#field_names), 2)),
                        hub.get(&format!("{}{}", stringify!(#field_names), 3)),
                        hub.get(&format!("{}{}", stringify!(#field_names), 4)),
                    ],)*
                }
            }

            pub fn set(&mut self, bank: &str, value: f32) {
                if bank.len() < 2 {
                    return;
                }

                let field = &bank[..1];

                match field {
                    #(#set_match_arms)*
                    _ => {}
                }
            }
        }

        impl<T: TimingSource> From<(&WindowRect, &ControlHub<T>)> for #struct_name {
            fn from((window_rect, hub): (&WindowRect, &ControlHub<T>)) -> Self {
                Self {
                    #first_bank: [
                        window_rect.w(),
                        window_rect.h(),
                        hub.get(&format!("{}{}", stringify!(#first_bank), 3)),
                        hub.get(&format!("{}{}", stringify!(#first_bank), 4)),
                    ],
                    #(#remaining_banks: [
                        hub.get(&format!("{}{}", stringify!(#remaining_banks), 1)),
                        hub.get(&format!("{}{}", stringify!(#remaining_banks), 2)),
                        hub.get(&format!("{}{}", stringify!(#remaining_banks), 3)),
                        hub.get(&format!("{}{}", stringify!(#remaining_banks), 4)),
                    ],)*
                }
            }
        }
    };

    expanded_struct.into()
}

struct UniformsArgs {
    banks: usize,
}

impl Parse for UniformsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut banks = 4;

        if !input.is_empty() {
            let name: Ident = input.parse()?;
            if name != "banks" {
                return Err(syn::Error::new(
                    name.span(),
                    "Expected `banks` parameter",
                ));
            }

            input.parse::<Token![=]>()?;
            let value: LitInt = input.parse()?;
            banks = value.base10_parse()?;
        }

        Ok(UniformsArgs { banks })
    }
}

fn generate_field_names(count: usize) -> Vec<syn::Ident> {
    (0..count)
        .map(|i| {
            let c = (b'a' + i as u8) as char;
            format_ident!("{}", c)
        })
        .collect()
}
