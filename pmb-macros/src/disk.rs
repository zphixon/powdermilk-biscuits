use syn::{Data, DeriveInput, Meta, NestedMeta};

trait OptionExt<Orig, Extra> {
    fn also(self, other: Extra) -> Option<(Orig, Extra)>;
}

impl<Orig, Extra> OptionExt<Orig, Extra> for Option<Orig> {
    fn also(self, extra: Extra) -> Option<(Orig, Extra)> {
        self.map(|orig| (orig, extra))
    }
}

pub fn derive_disk(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item: DeriveInput = syn::parse_macro_input!(input as DeriveInput);

    match item.data {
        Data::Struct(struct_) => {
            // fields that don't have #[skip]
            let saved_fields = struct_
                .fields
                .iter()
                .filter(|field| {
                    // skip fields with #[skip]
                    field.attrs.iter().all(|attr| {
                        let name = attr
                            .parse_meta()
                            .unwrap()
                            .path()
                            .segments
                            .first()
                            .unwrap()
                            .ident
                            .to_string();
                        name != "skip" && name != "custom_codec"
                    })
                })
                // extract the field's identifier
                .flat_map(|field| field.ident.as_ref())
                .collect::<Vec<_>>();

            let custom_codec_err = "expected two single-identifier arguments to custom_codec";
            let (custom_enc, custom_dec) = struct_
                .fields
                .iter()
                .flat_map(|field| {
                    // find fields with #[custom_codec(encoder, decoder)]
                    field
                        .attrs
                        .iter()
                        .find(|attr| {
                            attr.parse_meta()
                                .unwrap()
                                .path()
                                .segments
                                .first()
                                .unwrap()
                                .ident
                                == "custom_codec"
                        })
                        .also(field)
                })
                .map(|(attr, field)| {
                    // make sure it's actually a list like (encoder, decoder)
                    if let Meta::List(args) = attr.parse_meta().unwrap() {
                        // extract the identifiers
                        let args = args
                            .nested
                            .iter()
                            .map(|nested| match nested {
                                NestedMeta::Meta(meta) => {
                                    meta.path().segments.first().expect(custom_codec_err)
                                }
                                _ => panic!("{}", custom_codec_err),
                            })
                            .collect::<Vec<_>>();

                        // make sure we have exactly two
                        assert_eq!(2, args.len(), "{}", custom_codec_err);
                        let encoder = &args[0].ident;
                        let decoder = &args[1].ident;

                        let field = field.ident.as_ref().unwrap();
                        (
                            quote::quote!(
                                ::bincode::Encode::encode(&self.#encoder(), encoder)?;
                            ),
                            quote::quote!(
                                #field: #decoder(::bincode::Decode::decode(decoder)?),
                            ),
                        )
                    } else {
                        panic!("{}", custom_codec_err);
                    }
                })
                .unzip::<_, _, Vec<_>, Vec<_>>();

            let name = item.ident;
            let (impl_, type_, where_) = item.generics.split_for_impl();

            quote::quote! {
                impl #impl_ ::bincode::Encode for #name #type_ #where_ {
                    fn encode<E>(&self, encoder: &mut E) -> std::result::Result<(), ::bincode::error::EncodeError>
                    where
                        E: ::bincode::enc::Encoder,
                    {
                        #(::bincode::Encode::encode(&self.#saved_fields, encoder)?;)*
                        #(#custom_enc)*
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
                            #(#custom_dec)*
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
