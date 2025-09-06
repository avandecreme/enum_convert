use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{
    Attribute, Data, DataEnum, DeriveInput, Field, Ident, LitInt, Meta, Path, Token, Variant,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

use crate::idents::{ContainerIdent, FieldIdent, FieldRef, VariantIdent};

/// A "dumb" parser of the EnumFrom annotations
/// There is no check of consistency between annotations here.
pub struct ParsedEnumFrom {
    pub target_enum: ContainerIdent,
    pub container_annotations: Vec<ContainerAnnotation>,
    pub variants_annotations: HashMap<Variant, VariantAnnotations>,
}

impl ParsedEnumFrom {
    pub fn parse(input: TokenStream) -> syn::Result<ParsedEnumFrom> {
        let derive_input: DeriveInput = syn::parse(input)?;

        let data_enum = match derive_input.data {
            Data::Enum(data) => data,
            Data::Struct(_) | Data::Union(_) => Err(syn::Error::new(
                Span::call_site(),
                "EnumFrom can only be derived for enums",
            ))?,
        };

        let target_enum = ContainerIdent(derive_input.ident);
        let container_annotations = extract_container_annotations(&derive_input.attrs)?;
        let variants_annotations = extract_variants_annotations(data_enum)?;

        Ok(ParsedEnumFrom {
            target_enum,
            container_annotations,
            variants_annotations,
        })
    }
}

pub struct ContainerAnnotation(pub ContainerIdent);

pub struct VariantAnnotations {
    pub variant_annotations: Vec<VariantAnnotation>,
    pub fields_annotations: HashMap<FieldRef, FieldAnnotations>,
}

pub enum VariantAnnotation {
    Nothing {
        span: Span,
    },
    EnumOnly {
        span: Span,
        enum_ident: ContainerIdent,
    },
    EnumVariant {
        span: Span,
        enum_ident: ContainerIdent,
        variant_ident: VariantIdent,
    },
}

impl Parse for VariantAnnotation {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let path: Path = input.parse()?;
        if path.segments.len() == 1 {
            Ok(Self::EnumOnly {
                span,
                enum_ident: ContainerIdent(path.segments[0].ident.clone()),
            })
        } else if path.segments.len() == 2 {
            Ok(Self::EnumVariant {
                span,
                enum_ident: ContainerIdent(path.segments[0].ident.clone()),
                variant_ident: VariantIdent(path.segments[1].ident.clone()),
            })
        } else {
            Err(syn::Error::new_spanned(
                path,
                "Expected Enum or Enum::Variant",
            ))
        }
    }
}

pub struct FieldAnnotations {
    pub fields_annotations: Vec<FieldAnnotation>,
    pub field_span: Span,
}

#[derive(Clone)]
pub struct FieldAnnotation {
    pub source_enum: ContainerIdent,
    pub source_variant: VariantIdent,
    pub source_field: FieldRef,
    pub enum_span: Span,
    pub variant_span: Span,
    pub field_span: Span,
}

impl Parse for FieldAnnotation {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut path: Path = input.parse()?;
        if path.segments.len() == 2 {
            input.parse::<Token![.]>()?;
            let field_span = input.span();
            let source_field = if let Ok(ident) = input.parse::<Ident>() {
                FieldRef::FieldIdent(FieldIdent(ident))
            } else if let Ok(lit) = input.parse::<LitInt>() {
                FieldRef::FieldPos(lit.base10_parse()?)
            } else {
                Err(syn::Error::new(
                    field_span,
                    "Expected either a field identifier or a field position",
                ))?
            };
            let variant_segment = path.segments.pop().unwrap().into_value();
            let enum_segment = path.segments.pop().unwrap().into_value();
            Ok(FieldAnnotation {
                enum_span: enum_segment.span(),
                variant_span: variant_segment.span(),
                field_span,
                source_enum: ContainerIdent(enum_segment.ident),
                source_variant: VariantIdent(variant_segment.ident),
                source_field,
            })
        } else {
            Err(syn::Error::new_spanned(
                path,
                "Expected SourceEnum::SourceVariant.field_name",
            ))
        }
    }
}

fn extract_container_annotations(
    container_attrs: &[Attribute],
) -> syn::Result<Vec<ContainerAnnotation>> {
    let res = container_attrs
        .iter()
        .filter(|attr| attr.path().is_ident("enum_from"))
        .map(|attr| {
            let build_err = || {
                syn::Error::new(
                    attr.span(),
                    "expected a list of source enums, for example #[enum_from(Source1, Source2)]",
                )
            };

            match &attr.meta {
                Meta::List(meta_list) => meta_list
                    .parse_args_with(|input: ParseStream| {
                        Punctuated::<Ident, Token![,]>::parse_terminated(input)
                    })
                    .and_then(|idents| {
                        if idents.empty_or_trailing() {
                            Err(build_err())
                        } else {
                            Ok(idents
                                .into_iter()
                                .map(ContainerIdent)
                                .map(ContainerAnnotation)
                                .collect::<Vec<_>>())
                        }
                    }),
                Meta::Path(_) | Meta::NameValue(_) => Err(build_err()),
            }
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    Ok(res)
}

fn extract_variants_annotations(
    data_enum: DataEnum,
) -> syn::Result<HashMap<Variant, VariantAnnotations>> {
    let res = data_enum
        .variants
        .into_iter()
        .map(|variant| {
            extract_variant_annotations(&variant).map(|annotations| (variant, annotations))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(res.into_iter().collect())
}

fn extract_variant_annotations(variant: &Variant) -> syn::Result<VariantAnnotations> {
    let variant_annotations = variant
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("enum_from"))
        .map(|attr| {
            let build_err = || {
                syn::Error::new(
                    attr.span(),
                    "expected either #[enum_from] (if there is no ambiguity) or a list of variants, for example #[enum_from(Source1::VariantA, Source2::VariantB)]",
                )
            };
            match &attr.meta {
                Meta::Path(_) => Ok(vec![VariantAnnotation::Nothing { span: attr.span() }]),
                Meta::List(meta_list) => {
                    meta_list.parse_args_with(|input: ParseStream| {
                        Punctuated::<VariantAnnotation, Token![,]>::parse_terminated(input)
                            .and_then(|annotations| {
                                if annotations.empty_or_trailing() {
                                    Err(build_err())
                                } else {
                                    Ok(annotations.into_iter().collect())
                                }
                            })
                    })
                },
                Meta::NameValue(_) => Err(build_err()),
            }
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let fields_annotations = variant
        .fields
        .iter()
        .enumerate()
        .map(|(pos, field)| {
            let annotations = extract_field_annotations(field);
            match &field.ident {
                Some(field_ident) => annotations.map(|field_annotations| {
                    (
                        FieldRef::FieldIdent(FieldIdent(field_ident.clone())),
                        field_annotations,
                    )
                }),
                None => annotations
                    .map(|field_annotations| (FieldRef::FieldPos(pos), field_annotations)),
            }
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .collect();

    Ok(VariantAnnotations {
        variant_annotations,
        fields_annotations,
    })
}

fn extract_field_annotations(field: &Field) -> syn::Result<FieldAnnotations> {
    let fields_annotations = field.attrs.iter()
        .filter(|attr| attr.path().is_ident("enum_from"))
        .map(|attr| {
            let build_err = || {
                syn::Error::new(
                    attr.span(),
                    "expected a list of field names, for example #[enum_from(Source1::VariantA.field_x, Source2::VariantB.field_y)]",
                )
            };

            match &attr.meta {
                Meta::Path(_) | Meta::NameValue(_) => Err(build_err()),
                Meta::List(meta_list) => {
                    meta_list.parse_args_with(|input: ParseStream| {
                        Punctuated::<FieldAnnotation, Token![,]>::parse_terminated(input)
                            .and_then(|annotations| {
                                if annotations.empty_or_trailing() {
                                    Err(build_err())
                                } else {
                                    Ok(annotations.into_iter().collect())
                                }
                            })
                    })
                }
            }
        }).collect::<Result<Vec<Vec<FieldAnnotation>>, syn::Error>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(FieldAnnotations {
        fields_annotations,
        field_span: field.span(),
    })
}
