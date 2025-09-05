use std::fmt::Display;

use quote::ToTokens;
use syn::Ident;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContainerIdent(pub Ident);

impl Display for ContainerIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ToTokens for ContainerIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantIdent(pub Ident);

impl ToTokens for VariantIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldIdent(pub Ident);

impl Display for FieldIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ToTokens for FieldIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FieldRef {
    FieldPos(usize),
    FieldIdent(FieldIdent),
}
