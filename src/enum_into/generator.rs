use std::collections::{BTreeMap, HashMap};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Fields, Variant};

use crate::{
    enum_into::parser::{
        ContainerAnnotation, FieldAnnotation, FieldAnnotations, ParsedEnumInto, VariantAnnotation,
    },
    idents::{ContainerIdent, FieldIdent, FieldRef, VariantIdent},
};

/// A struct holding all the data necessary to generate a TokenStream.
/// Once constructed, the code generation should not fail.
pub struct EnumIntoGenerator {
    target_enums: HashMap<ContainerIdent, VariantsMapping>,
    source_enum: ContainerIdent,
    source_variants: HashMap<VariantIdent, Variant>,
}

struct VariantsMapping(HashMap<VariantIdent, Vec<VariantMapping>>);

enum VariantMapping {
    Unit {
        source_variant: VariantIdent,
    },
    Tuple {
        source_variant: VariantIdent,
        fields_mapping: HashMap<usize, usize>,
    },
    Struct {
        source_variant: VariantIdent,
        fields_mapping: HashMap<FieldIdent, FieldIdent>,
    },
}

impl VariantMapping {
    fn source_variant(&self) -> &VariantIdent {
        match self {
            VariantMapping::Unit { source_variant } => source_variant,
            VariantMapping::Tuple { source_variant, .. } => source_variant,
            VariantMapping::Struct { source_variant, .. } => source_variant,
        }
    }
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
                let source_variant = source_variants.get(variant_mapping.source_variant()).expect(
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
    match (&variant.fields, variant_mapping) {
        (Fields::Unit, VariantMapping::Unit { source_variant }) => quote! {
            #source_enum::#source_variant => #target_enum::#target_variant,
        },
        (
            Fields::Unnamed(fields),
            VariantMapping::Tuple {
                source_variant,
                fields_mapping,
            },
        ) => {
            let (source_fields, target_fields): (Vec<_>, Vec<_>) = (0..fields.unnamed.len())
                .map(|field_source_pos| {
                    let field_target_pos = fields_mapping
                        .get(&field_source_pos)
                        .unwrap_or(&field_source_pos);
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
                source_variant,
                fields_mapping,
            },
        ) => {
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
                    let target_field = &fields_mapping.get(&source_field).unwrap_or(&source_field);
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

        for (source_variant, mut variant_annotations) in variants_annotations {
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

                let fields_annotations = extract_fields_annotations(
                    &mut variant_annotations.fields_annotations,
                    target_enum,
                    &target_variant,
                )?;
                let fields = &source_variant.fields;
                let source_variant = VariantIdent(source_variant.ident.clone());
                let variant_mapping = match fields {
                    Fields::Unit => VariantMapping::Unit { source_variant },
                    Fields::Unnamed(_) => VariantMapping::Tuple {
                        source_variant,
                        fields_mapping: fields_annotations
                            .into_iter()
                            .map(|source_to_target| match source_to_target {
                                (
                                    FieldRef::FieldPos(source_pos),
                                    FieldAnnotation {
                                        target_field: FieldRef::FieldPos(target_pos),
                                        ..
                                    },
                                ) => Ok((source_pos, target_pos)),
                                (_, FieldAnnotation { field_span, .. }) => Err(syn::Error::new(
                                    field_span,
                                    "Unexpected mapping to named field for tuple variant",
                                )),
                            })
                            .collect::<syn::Result<_>>()?,
                    },
                    Fields::Named(_) => VariantMapping::Struct {
                        source_variant,
                        fields_mapping: fields_annotations
                            .into_iter()
                            .map(|source_to_target| match source_to_target {
                                (
                                    FieldRef::FieldIdent(source_ident),
                                    FieldAnnotation {
                                        target_field: FieldRef::FieldIdent(target_ident),
                                        ..
                                    },
                                ) => Ok((source_ident, target_ident)),
                                (_, FieldAnnotation { field_span, .. }) => Err(syn::Error::new(
                                    field_span,
                                    "Unexpected mapping to positional field for struct variant",
                                )),
                            })
                            .collect::<syn::Result<_>>()?,
                    },
                };

                let mut variant_mappings = variants_mapping
                    .remove(&target_variant)
                    .unwrap_or_else(Vec::new);
                variant_mappings.push(variant_mapping);

                variants_mapping.insert(target_variant, variant_mappings);
            }

            check_unused_variants_annotations(target_variants)?;
            check_unused_fields_annotations(&target_enums, variant_annotations.fields_annotations)?;

            source_variants.insert(VariantIdent(source_variant.ident.clone()), source_variant);
        }

        Ok(EnumIntoGenerator {
            target_enums,
            source_enum,
            source_variants,
        })
    }
}

fn check_unused_variants_annotations(
    target_variants: HashMap<ContainerIdent, (VariantIdent, Span)>,
) -> syn::Result<()> {
    for (target_enum, (_, span)) in target_variants {
        Err(syn::Error::new(
            span,
            format!(
                "target enum `{target_enum}` is not specified in this enum's #[enum_into] annotation"
            ),
        ))?
    }
    Ok(())
}

fn check_unused_fields_annotations(
    target_enums: &HashMap<ContainerIdent, VariantsMapping>,
    fields_annotations: HashMap<FieldRef, FieldAnnotations>,
) -> syn::Result<()> {
    for field_annotations in fields_annotations.into_values() {
        for field_annotation in field_annotations.fields_annotations {
            if target_enums.contains_key(&field_annotation.target_enum) {
                Err(syn::Error::new(
                    field_annotation.variant_span,
                    "Field mapping for unexpected enum and variant combination",
                ))?
            } else {
                Err(syn::Error::new(
                    field_annotation.enum_span,
                    "Field mapping for unknown enum",
                ))?
            }
        }
    }

    Ok(())
}

fn extract_fields_annotations(
    fields_annotations: &mut HashMap<FieldRef, FieldAnnotations>,
    target_enum: &ContainerIdent,
    target_variant: &VariantIdent,
) -> syn::Result<BTreeMap<FieldRef, FieldAnnotation>> {
    Ok(fields_annotations
        .iter_mut()
        .filter_map(|(source_field, field_annotations)| {
            let mut annotations = field_annotations
                .fields_annotations
                .extract_if(.., |field_annotation| {
                    field_annotation.target_enum == *target_enum
                        && field_annotation.target_variant == *target_variant
                })
                .collect::<Vec<_>>();
            let annotation = annotations.pop();
            if annotations.pop().is_some() {
                Some(Err(syn::Error::new(
                    field_annotations.field_span,
                    format!("Multiple mapping found for target enum `{target_enum}`"),
                )))
            } else {
                annotation.map(|annotation| Ok((source_field.clone(), annotation)))
            }
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .collect())
}
