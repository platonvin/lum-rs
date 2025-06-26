//! bytemuck alternative for casting struct as slice of bytes

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// bytemuck alternative for casting struct as slice of bytes
#[proc_macro_derive(AsU8Slice)]
pub fn as_u8_slice_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the name of the struct
    let name = &input.ident;

    // Generate the implementation
    let expanded = quote! {
        impl #name {
            pub fn as_u8_slice(&self) -> &[u8] {
                unsafe {
                    std::slice::from_raw_parts(
                        (self as *const #name) as *const u8,
                        std::mem::size_of::<#name>(),
                    )
                }
            }
        }
    };

    // Convert the expanded code into a TokenStream
    TokenStream::from(expanded)
}
