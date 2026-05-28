mod mapped_data;

use quote::ToTokens;

#[proc_macro_attribute]
pub fn mapped(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    mapped_data::Mapped::parse(attr.into(), input.into())
        .map_or_else(
            |err| err.to_compile_error(),
            |data| data.into_token_stream(),
        )
        .into()
}
