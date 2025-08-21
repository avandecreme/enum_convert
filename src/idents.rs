use std::fmt::Display;

use quote::ToTokens;
use syn::Ident;

#[derive(Clone)]
pub struct ContainerIdent(pub Ident);

impl Display for ContainerIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for ContainerIdent {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ContainerIdent {}

impl std::hash::Hash for ContainerIdent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl ToTokens for ContainerIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}

#[derive(Clone)]
pub struct VariantIdent(pub Ident);

impl PartialEq for VariantIdent {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for VariantIdent {}

impl std::hash::Hash for VariantIdent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl ToTokens for VariantIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}

#[derive(Clone)]
pub struct FieldIdent(pub Ident);

impl PartialEq for FieldIdent {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for FieldIdent {}

impl std::hash::Hash for FieldIdent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl ToTokens for FieldIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}
