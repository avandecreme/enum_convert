use proc_macro::TokenStream;

use crate::enum_from::{generator::EnumFromGenerator, parser::ParsedEnumFrom};

mod generator;
mod parser;

pub fn derive_enum_from_impl(input: TokenStream) -> TokenStream {
    ParsedEnumFrom::parse(input)
        .and_then(EnumFromGenerator::try_from)
        .map(EnumFromGenerator::generate)
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}
