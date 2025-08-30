use proc_macro::TokenStream;

use crate::enum_into::{generator::EnumIntoGenerator, parser::ParsedEnumInto};

mod generator;
mod parser;

pub fn derive_enum_into_impl(input: TokenStream) -> TokenStream {
    ParsedEnumInto::parse(input)
        .and_then(EnumIntoGenerator::try_from)
        .map(EnumIntoGenerator::generate)
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}
