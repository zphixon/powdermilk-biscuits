use proc_macro2::{Ident, Span};
use syn::{Data, DeriveInput};

#[proc_macro_derive(Disk, attributes(disk_skip))]
pub fn derive_disk(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item: DeriveInput = syn::parse_macro_input!(input as DeriveInput);

    match item.data {
        Data::Struct(struct_) => {
            let name = item.ident;
            let skip = Ident::new("disk_skip", Span::call_site());

            let mut saved_fields = Vec::new();
            for field in struct_.fields.iter() {
                // if none of the field's attributes are #[disk_skip]
                if field
                    .attrs
                    .iter()
                    .all(|attr| !attr.parse_meta().unwrap().path().is_ident(&skip))
                {
                    saved_fields.push(field.ident.as_ref().unwrap());
                }
            }

            let (impl_, type_, where_) = item.generics.split_for_impl();

            quote::quote! {
                impl #impl_ ::bincode::Encode for #name #type_ #where_ {
                    fn encode<E>(&self, encoder: &mut E) -> std::result::Result<(), ::bincode::error::EncodeError>
                    where
                        E: ::bincode::enc::Encoder,
                    {
                        #(::bincode::Encode::encode(&self.#saved_fields, encoder)?;)*
                        Ok(())
                    }
                }

                impl #impl_ ::bincode::Decode for #name #type_ #where_ {
                    #[allow(clippy::needless_update)]
                    fn decode<D>(decoder: &mut D) -> std::result::Result<Self, ::bincode::error::DecodeError>
                    where
                        D: ::bincode::de::Decoder,
                    {
                        Ok(Self {
                            #(#saved_fields: ::bincode::Decode::decode(decoder)?,)*
                            ..Self::default()
                        })
                    }
                }
            }
            .into()
        }

        _ => panic!("Disk is unimplemented for enums and unions"),
    }
}
