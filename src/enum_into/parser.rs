use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{
    Attribute, Data, DataEnum, DeriveInput, Field, Ident, Meta, Path, Token, Variant,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

use crate::idents::{ContainerIdent, FieldIdent, VariantIdent};

/// A "dumb" parser of the EnumInto annotations
/// There is no check of consistency between annotations here.
pub struct ParsedEnumInto {
    pub source_enum: ContainerIdent,
    pub container_annotations: Vec<ContainerAnnotation>,
    pub variants_annotations: HashMap<Variant, VariantAnnotations>,
}

impl ParsedEnumInto {
    pub fn parse(input: TokenStream) -> syn::Result<ParsedEnumInto> {
        let derive_input: DeriveInput = syn::parse(input)?;

        let data_enum = match derive_input.data {
            Data::Enum(data) => data,
            Data::Struct(_) | Data::Union(_) => Err(syn::Error::new(
                Span::call_site(),
                "EnumInto can only be derived for enums",
            ))?,
        };

        let source_enum = ContainerIdent(derive_input.ident);
        let container_annotations = extract_container_annotations(&derive_input.attrs)?;
        let variants_annotations = extract_variants_annotations(data_enum)?;

        Ok(ParsedEnumInto {
            source_enum,
            container_annotations,
            variants_annotations,
        })
    }
}

pub struct ContainerAnnotation(pub ContainerIdent);

pub struct VariantAnnotations {
    pub variant_annotations: Vec<VariantAnnotation>,
    pub fields_annotations: HashMap<FieldIdent, FieldAnnotations>,
}

pub enum VariantAnnotation {
    Nothing,
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

pub struct FieldAnnotation {
    pub target_enum: ContainerIdent,
    pub target_variant: VariantIdent,
    pub target_field: FieldIdent,
}

impl Parse for FieldAnnotation {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        if path.segments.len() == 2 {
            let target_enum = ContainerIdent(path.segments[0].ident.clone());
            let target_variant = VariantIdent(path.segments[1].ident.clone());
            input.parse::<Token![.]>()?;
            let target_field = FieldIdent(input.parse()?);
            Ok(FieldAnnotation {
                target_enum,
                target_variant,
                target_field,
            })
        } else {
            Err(syn::Error::new_spanned(
                path,
                "Expected TargetEnum::TargetVariant.field_name",
            ))
        }
    }
}

fn extract_container_annotations(
    container_attrs: &[Attribute],
) -> syn::Result<Vec<ContainerAnnotation>> {
    let res = container_attrs
        .iter()
        .filter(|attr| attr.path().is_ident("enum_into"))
        .map(|attr| {
            let build_err = || {
                syn::Error::new(
                    attr.span(),
                    "expected a list of target enums, for example #[enum_into(Target1, Target2)]",
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
        .filter(|attr| attr.path().is_ident("enum_into"))
        .map(|attr| {
            let build_err = || {
                syn::Error::new(
                    attr.span(),
                    "expected a list of variants, for example #[enum_into(Target1::VariantA, Target2::VariantB)].\n\
                    If there is only one target enum and the variant names are identical between source and target, #[enum_into] can be omitted.",
                )
            };
            match &attr.meta {
                Meta::Path(_) => Ok(vec![VariantAnnotation::Nothing]),
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
        .filter_map(|field| {
            field.ident.as_ref().map(|field_ident| {
                extract_field_annotations(field)
                    .map(|field_annotations| (FieldIdent(field_ident.clone()), field_annotations))
            })
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
        .filter(|attr| attr.path().is_ident("enum_into"))
        .map(|attr| {
            let build_err = || {
                syn::Error::new(
                    attr.span(),
                    "expected a list of field names, for example #[enum_into(Target1::VariantA.field_x, Target2::VariantB.field_y)]",
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
