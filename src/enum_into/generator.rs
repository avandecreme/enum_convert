use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Fields, Variant};

use crate::{
    enum_into::parser::{ContainerAnnotation, FieldAnnotations, ParsedEnumInto, VariantAnnotation},
    idents::{ContainerIdent, FieldIdent, VariantIdent},
};

/// A struct holding all the data necessary to generate a TokenStream.
/// Once constructed, the code generation should not fail.
pub struct EnumIntoGenerator {
    target_enums: HashMap<ContainerIdent, VariantsMapping>,
    source_enum: ContainerIdent,
    source_variants: HashMap<VariantIdent, Variant>,
}

struct VariantsMapping(HashMap<VariantIdent, Vec<VariantMapping>>);

struct VariantMapping {
    source_variant: VariantIdent,
    fields_mapping: HashMap<FieldIdent, FieldIdent>,
}

impl EnumIntoGenerator {
    pub fn generate(self) -> TokenStream {
        let source_enum = &self.source_enum;
        let source_variants = &self.source_variants;

        let impl_blocks = self
            .target_enums
            .into_iter()
            .map(|(target_enum, variants_mapping)| {
                generate_from_impl(target_enum, variants_mapping, source_enum, source_variants)
            })
            .collect::<Vec<_>>();

        quote! {
            #(#impl_blocks)*
        }
    }
}

fn generate_from_impl(
    target_enum: ContainerIdent,
    variants_mapping: VariantsMapping,
    source_enum: &ContainerIdent,
    source_variants: &HashMap<VariantIdent, Variant>,
) -> TokenStream {
    let match_arms = variants_mapping
        .0
        .into_iter()
        .flat_map(|(target_variant, variant_mappings)| {
            variant_mappings.into_iter().map(|variant_mapping| {
                let source_variant = source_variants.get(&variant_mapping.source_variant).expect(
                    "All source variants in variant_mapping should be present in source_variants",
                );
                generate_match_arm(
                    &target_variant,
                    variant_mapping,
                    &target_enum,
                    source_enum,
                    source_variant,
                )
            }).collect::<Vec<_>>()
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
    target_variant: &VariantIdent,
    variant_mapping: VariantMapping,
    target_enum: &ContainerIdent,
    source_enum: &ContainerIdent,
    variant: &Variant,
) -> TokenStream {
    let source_variant = &variant.ident;

    match &variant.fields {
        Fields::Unit => quote! {
            #source_enum::#source_variant => #target_enum::#target_variant,
        },
        Fields::Unnamed(fields) => {
            let field_names: Vec<_> = (0..fields.unnamed.len())
                .map(|i| quote::format_ident!("field_{}", i))
                .collect();
            let field_conversions: Vec<_> = field_names
                .iter()
                .map(|name| quote! { #name.into() })
                .collect();
            quote! {
                #source_enum::#source_variant(#(#field_names),*) =>
                #target_enum::#target_variant(#(#field_conversions),*),
            }
        }
        Fields::Named(fields) => {
            let (source_fields, target_fields): (Vec<_>, Vec<_>) = fields
                .named
                .iter()
                .map(|field| {
                    let source_field = FieldIdent(
                        field
                            .ident
                            .as_ref()
                            .expect("A named field should always have an ident")
                            .clone(),
                    );
                    let target_field = &variant_mapping
                        .fields_mapping
                        .get(&source_field)
                        .unwrap_or(&source_field);
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
    }
}

impl TryFrom<ParsedEnumInto> for EnumIntoGenerator {
    type Error = syn::Error;

    fn try_from(
        ParsedEnumInto {
            source_enum,
            container_annotations,
            variants_annotations,
        }: ParsedEnumInto,
    ) -> Result<Self, Self::Error> {
        if container_annotations.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "enum_into attribute with target enum names is required",
            ));
        }

        let mut source_variants: HashMap<VariantIdent, Variant> = HashMap::new();

        let mut target_enums = container_annotations
            .into_iter()
            .map(|ContainerAnnotation(target_enum)| (target_enum, VariantsMapping(HashMap::new())))
            .collect::<HashMap<_, _>>();

        for (source_variant, variant_annotations) in variants_annotations {
            let mut target_variants = variant_annotations
                .variant_annotations
                .into_iter()
                .filter_map(|variant_annotation| match variant_annotation {
                    VariantAnnotation::Nothing => None,
                    VariantAnnotation::EnumOnly { span, enum_ident } => Some((
                        enum_ident,
                        (VariantIdent(source_variant.ident.clone()), span),
                    )),
                    VariantAnnotation::EnumVariant {
                        span,
                        enum_ident,
                        variant_ident,
                    } => Some((enum_ident, (variant_ident.clone(), span))),
                })
                .collect::<HashMap<_, _>>();
            for (target_enum, VariantsMapping(variants_mapping)) in target_enums.iter_mut() {
                let target_variant = target_variants
                    .remove(target_enum)
                    .map(|(target_variant, _span)| target_variant)
                    .unwrap_or_else(|| VariantIdent(source_variant.ident.clone()));

                let fields_mapping = get_fields_mapping(
                    &variant_annotations.fields_annotations,
                    target_enum,
                    &target_variant,
                )?;

                let mut variant_mappings = variants_mapping
                    .remove(&target_variant)
                    .unwrap_or_else(Vec::new);
                variant_mappings.push(VariantMapping {
                    source_variant: VariantIdent(source_variant.ident.clone()),
                    fields_mapping,
                });

                variants_mapping.insert(target_variant, variant_mappings);
            }
            source_variants.insert(VariantIdent(source_variant.ident.clone()), source_variant);

            for (target_enum, (_, span)) in target_variants {
                Err(syn::Error::new(
                    span,
                    format!(
                        "target enum `{target_enum}` is not specified in this enum's #[enum_into] annotation"
                    ),
                ))?
            }
        }

        Ok(EnumIntoGenerator {
            target_enums,
            source_enum,
            source_variants,
        })
    }
}

fn get_fields_mapping(
    fields_annotations: &HashMap<FieldIdent, FieldAnnotations>,
    target_enum: &ContainerIdent,
    target_variant: &VariantIdent,
) -> syn::Result<HashMap<FieldIdent, FieldIdent>> {
    Ok(fields_annotations
        .iter()
        .map(|(source_field, field_annotations)| {
            let annotations = field_annotations
                .fields_annotations
                .iter()
                .filter(|field_annotation| {
                    field_annotation.target_enum == *target_enum
                        && field_annotation.target_variant == *target_variant
                })
                .collect::<Vec<_>>();
            let target_field = match annotations.len() {
                0 => source_field.clone(),
                1 => annotations[0].target_field.clone(),
                _ => Err(syn::Error::new(
                    field_annotations.field_span,
                    format!("Multiple mapping found for target enum `{target_enum}`"),
                ))?,
            };

            Ok((source_field.clone(), target_field))
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .collect())
}
