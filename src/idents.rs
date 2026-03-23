use std::fmt::Display;

use proc_macro2::{TokenStream, TokenTree};
use syn::Ident;

pub trait ToTokenStream {
    fn to_token_stream(&self) -> TokenStream;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContainerIdent(pub Ident);

impl Display for ContainerIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ToTokenStream for ContainerIdent {
    fn to_token_stream(&self) -> TokenStream {
        TokenStream::from(TokenTree::Ident(self.0.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantIdent(pub Ident);

impl Display for VariantIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ToTokenStream for VariantIdent {
    fn to_token_stream(&self) -> TokenStream {
        TokenStream::from(TokenTree::Ident(self.0.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldIdent(pub Ident);

impl Display for FieldIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ToTokenStream for FieldIdent {
    fn to_token_stream(&self) -> TokenStream {
        TokenStream::from(TokenTree::Ident(self.0.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FieldRef {
    FieldPos(usize),
    FieldIdent(FieldIdent),
}
