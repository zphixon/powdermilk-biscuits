mod disk;
mod loop_;

#[proc_macro_derive(Disk, attributes(skip, custom_codec))]
pub fn derive_disk(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    disk::derive_disk(input)
}

#[proc_macro]
pub fn pmb_loop(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    loop_::pmb_loop(input)
}
