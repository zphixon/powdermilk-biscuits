use quote::quote;
use syn::{Data, DeriveInput};

#[proc_macro_derive(EnumVariantCount)]
pub fn derive_enum_variant_count(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let syn_item: DeriveInput = syn::parse(input).unwrap();
    let len = match syn_item.data {
        Data::Enum(item) => item.variants.len(),
        _ => panic!("EnumVariantCount may only be applied to enums"),
    };
    let name = syn_item.ident;

    let expanded = quote! {
        impl #name {
            pub const NUM_VARIANTS: usize = #len;
        }
    };

    expanded.into()
}
