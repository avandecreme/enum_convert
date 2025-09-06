use std::collections::{BTreeMap, HashMap};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Fields, Variant};

use crate::{
    enum_from::parser::{
        ContainerAnnotation, FieldAnnotation, FieldAnnotations, ParsedEnumFrom, VariantAnnotation,
    },
    idents::{ContainerIdent, FieldIdent, FieldRef, VariantIdent},
};

/// A struct holding all the data necessary to generate a TokenStream.
/// Once constructed, the code generation should not fail.
pub struct EnumFromGenerator {
    source_enums: HashMap<ContainerIdent, VariantsMapping>,
    target_enum: ContainerIdent,
    target_variants: HashMap<VariantIdent, Variant>,
}

struct VariantsMapping(HashMap<VariantIdent, VariantMapping>);

enum VariantMapping {
    Unit {
        target_variant: VariantIdent,
    },
    Tuple {
        target_variant: VariantIdent,
        fields_mapping: HashMap<usize, usize>,
    },
    Struct {
        target_variant: VariantIdent,
        fields_mapping: HashMap<FieldIdent, FieldIdent>,
    },
}

impl VariantMapping {
    fn target_variant(&self) -> &VariantIdent {
        match self {
            VariantMapping::Unit { target_variant } => target_variant,
            VariantMapping::Tuple { target_variant, .. } => target_variant,
            VariantMapping::Struct { target_variant, .. } => target_variant,
        }
    }
}

impl EnumFromGenerator {
    pub fn generate(self) -> TokenStream {
        let target_enum = &self.target_enum;
        let target_variants = &self.target_variants;

        let impl_blocks = self
            .source_enums
            .into_iter()
            .map(|(source_enum, variants_mapping)| {
                generate_from_impl(source_enum, variants_mapping, target_enum, target_variants)
            })
            .collect::<Vec<_>>();

        quote! {
            #(#impl_blocks)*
        }
    }
}

fn generate_from_impl(
    source_enum: ContainerIdent,
    variants_mapping: VariantsMapping,
    target_enum: &ContainerIdent,
    target_variants: &HashMap<VariantIdent, Variant>,
) -> TokenStream {
    let match_arms =
        variants_mapping
            .0
            .into_iter()
            .map(|(source_variant, variant_mapping)| {
                let target_variant = target_variants.get(variant_mapping.target_variant()).expect(
                    "All target variants in variant_mapping should be present in target_variants",
                );
                generate_match_arm(
                    source_variant,
                    variant_mapping,
                    &source_enum,
                    target_enum,
                    target_variant,
                )
            })
            .collect::<Vec<_>>();

    quote! {
        impl From<#source_enum> for #target_enum {
            fn from(value: #source_enum) -> Self {
                match value {
                    #(#match_arms)*
                }
            }
        }
    }
}

fn generate_match_arm(
    source_variant: VariantIdent,
    variant_mapping: VariantMapping,
    source_enum: &ContainerIdent,
    target_enum: &ContainerIdent,
    variant: &Variant,
) -> TokenStream {
    match (&variant.fields, variant_mapping) {
        (Fields::Unit, VariantMapping::Unit { target_variant }) => {
            quote! { #source_enum::#source_variant => #target_enum::#target_variant, }
        }
        (
            Fields::Unnamed(fields),
            VariantMapping::Tuple {
                target_variant,
                fields_mapping,
            },
        ) => {
            let (source_fields, target_fields): (Vec<_>, Vec<_>) = (0..fields.unnamed.len())
                .map(|field_target_pos| {
                    let field_source_pos = fields_mapping
                        .get(&field_target_pos)
                        .unwrap_or(&field_target_pos);
                    let target_field_name = quote::format_ident!("field_{field_target_pos}");
                    (
                        quote::format_ident!("field_{field_source_pos}"),
                        quote! { #target_field_name.into() },
                    )
                })
                .unzip();
            quote! {
                #source_enum::#source_variant(#(#source_fields),*) =>
                #target_enum::#target_variant(#(#target_fields),*),
            }
        }
        (
            Fields::Named(fields),
            VariantMapping::Struct {
                target_variant,
                fields_mapping,
            },
        ) => {
            let (source_fields, target_fields): (Vec<_>, Vec<_>) = fields
                .named
                .iter()
                .map(|field| {
                    let target_field = FieldIdent(
                        field
                            .ident
                            .as_ref()
                            .expect("A named field should always have an ident")
                            .clone(),
                    );
                    let source_field = &fields_mapping.get(&target_field).unwrap_or(&target_field);
                    (
                        quote! { #source_field },
                        quote! { #target_field: #source_field.into() },
                    )
                })
                .unzip();

            quote! {
                #source_enum::#source_variant { #(#source_fields),* } =>
                #target_enum::#target_variant { #(#target_fields),* },
            }
        }
        (_, _) => panic!("Unexpected mixing of variant types"),
    }
}

impl TryFrom<ParsedEnumFrom> for EnumFromGenerator {
    type Error = syn::Error;

    fn try_from(
        ParsedEnumFrom {
            target_enum,
            container_annotations,
            variants_annotations,
        }: ParsedEnumFrom,
    ) -> Result<Self, Self::Error> {
        let single_source_enum = match &container_annotations[..] {
            [] => Err(syn::Error::new(
                Span::call_site(),
                "enum_from attribute with source enum names is required",
            ))?,
            [source_enum] => Some(source_enum.0.clone()),
            _ => None,
        };

        let mut target_variants: HashMap<VariantIdent, Variant> = HashMap::new();

        let mut source_enums = container_annotations
            .into_iter()
            .map(|ContainerAnnotation(source_enum)| {
                (
                    source_enum,
                    VariantsMapping(HashMap::<VariantIdent, VariantMapping>::new()),
                )
            })
            .collect::<HashMap<_, _>>();

        for (target_variant, variant_annotations) in variants_annotations {
            for variant_annotation in variant_annotations.variant_annotations {
                let (source_enum, source_variant, span) = get_source_enum_and_variant(
                    &target_variant,
                    single_source_enum.as_ref(),
                    variant_annotation,
                )?;

                let VariantsMapping(variants_mapping) = source_enums.get_mut(&source_enum).ok_or_else(|| {
                    syn::Error::new(
                        span,
                        format!(
                            "source enum `{source_enum}` is not specified in this enum's #[enum_from] annotation"
                        )
                    )
                })?;

                let fields_mapping = get_fields_mapping(
                    &variant_annotations.fields_annotations,
                    &source_enum,
                    &source_variant,
                )?;
                let fields = &target_variant.fields;
                let target_variant = VariantIdent(target_variant.ident.clone());
                let variant_mapping = match fields {
                    Fields::Unit => VariantMapping::Unit { target_variant },
                    Fields::Unnamed(_) => VariantMapping::Tuple {
                        target_variant,
                        fields_mapping: fields_mapping
                            .into_iter()
                            .map(|target_to_source| match target_to_source {
                                (
                                    FieldRef::FieldPos(target_pos),
                                    FieldAnnotation {
                                        source_field: FieldRef::FieldPos(source_pos),
                                        ..
                                    },
                                ) => Ok((target_pos, source_pos)),
                                (_, FieldAnnotation { field_span, .. }) => Err(syn::Error::new(
                                    field_span,
                                    "Unexpected mapping to named field for tuple variant",
                                )),
                            })
                            .collect::<syn::Result<_>>()?,
                    },
                    Fields::Named(_) => VariantMapping::Struct {
                        target_variant,
                        fields_mapping: fields_mapping
                            .into_iter()
                            .map(|target_to_source| match target_to_source {
                                (
                                    FieldRef::FieldIdent(target_ident),
                                    FieldAnnotation {
                                        source_field: FieldRef::FieldIdent(source_ident),
                                        ..
                                    },
                                ) => Ok((target_ident, source_ident)),
                                (_, FieldAnnotation { field_span, .. }) => Err(syn::Error::new(
                                    field_span,
                                    "Unexpected mapping to positional field for struct variant",
                                )),
                            })
                            .collect::<syn::Result<_>>()?,
                    },
                };

                variants_mapping.insert(source_variant, variant_mapping);
            }
            target_variants.insert(VariantIdent(target_variant.ident.clone()), target_variant);
        }

        Ok(EnumFromGenerator {
            source_enums,
            target_enum,
            target_variants,
        })
    }
}

fn get_fields_mapping(
    fields_annotations: &HashMap<FieldRef, FieldAnnotations>,
    source_enum: &ContainerIdent,
    source_variant: &VariantIdent,
) -> syn::Result<BTreeMap<FieldRef, FieldAnnotation>> {
    Ok(fields_annotations
        .iter()
        .filter_map(|(target_field, field_annotations)| {
            let annotations = field_annotations
                .fields_annotations
                .iter()
                .filter(|field_annotation| {
                    field_annotation.source_enum == *source_enum
                        && field_annotation.source_variant == *source_variant
                })
                .collect::<Vec<_>>();
            match annotations.len() {
                0 => None,
                1 => Some(Ok((target_field.clone(), annotations[0].clone()))),
                _ => Some(Err(syn::Error::new(
                    field_annotations.field_span,
                    format!("Multiple mapping found for source enum `{source_enum}`"),
                ))),
            }
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .collect())
}

/// Returns the source enum and variant for the given variant annotation.
fn get_source_enum_and_variant(
    target_variant: &Variant,
    single_source_enum: Option<&ContainerIdent>,
    variant_annotation: VariantAnnotation,
) -> syn::Result<(ContainerIdent, VariantIdent, Span)> {
    match variant_annotation {
        VariantAnnotation::Nothing { span } => {
            if let Some(source_enum) = single_source_enum {
                Ok((
                    source_enum.clone(),
                    VariantIdent(target_variant.ident.clone()),
                    span,
                ))
            } else {
                Err(syn::Error::new(
                    span,
                    "When multiple source enums are specified, each variant must specify from which enum to convert with #[enum_from(Enum)] or #[enum_from(Enum::Variant)]",
                ))
            }
        }
        VariantAnnotation::EnumOnly { span, enum_ident } => {
            Ok((enum_ident, VariantIdent(target_variant.ident.clone()), span))
        }
        VariantAnnotation::EnumVariant {
            span,
            enum_ident,
            variant_ident,
        } => Ok((enum_ident, variant_ident, span)),
    }
}
