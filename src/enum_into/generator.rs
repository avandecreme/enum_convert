use std::collections::{BTreeMap, HashMap};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Fields, FieldsNamed, FieldsUnnamed, Variant, spanned::Spanned as _};

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
    UnitToUnit {
        source_variant: VariantIdent,
    },
    TupleToTuple {
        source_variant: VariantIdent,
        fields_mapping: HashMap<usize, usize>,
    },
    TupleToStruct {
        source_variant: VariantIdent,
        fields_mapping: HashMap<usize, FieldIdent>,
    },
    StructToStruct {
        source_variant: VariantIdent,
        fields_mapping: HashMap<FieldIdent, FieldIdent>,
    },
    StructToTuple {
        source_variant: VariantIdent,
        fields_mapping: HashMap<FieldIdent, usize>,
    },
}

impl VariantMapping {
    fn source_variant(&self) -> &VariantIdent {
        match self {
            VariantMapping::UnitToUnit { source_variant } => source_variant,
            VariantMapping::TupleToTuple { source_variant, .. } => source_variant,
            VariantMapping::TupleToStruct { source_variant, .. } => source_variant,
            VariantMapping::StructToStruct { source_variant, .. } => source_variant,
            VariantMapping::StructToTuple { source_variant, .. } => source_variant,
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
        (Fields::Unit, VariantMapping::UnitToUnit { source_variant }) => {
            quote! { #source_enum::#source_variant => #target_enum::#target_variant, }
        }
        (
            Fields::Unnamed(fields),
            VariantMapping::TupleToTuple {
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
            Fields::Unnamed(fields),
            VariantMapping::TupleToStruct {
                source_variant,
                fields_mapping,
            },
        ) => {
            let (source_fields, target_fields): (Vec<_>, Vec<_>) = (0..fields.unnamed.len())
                .map(|field_source_pos| {
                    let target_ident = fields_mapping
                        .get(&field_source_pos)
                        .expect("fields_mapping exhaustiveness should have been checked");
                    (
                        quote! { #target_ident },
                        quote! { #target_ident: #target_ident.into() },
                    )
                })
                .unzip();
            quote! {
                #source_enum::#source_variant(#(#source_fields),*) =>
                #target_enum::#target_variant { #(#target_fields),* },
            }
        }
        (
            Fields::Named(fields),
            VariantMapping::StructToStruct {
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
        (
            Fields::Named(_),
            VariantMapping::StructToTuple {
                source_variant,
                fields_mapping,
            },
        ) => {
            let (source_fields, target_fields): (Vec<_>, Vec<_>) = fields_mapping
                .into_iter()
                .map(|(source_ident, target_pos)| (target_pos, source_ident))
                .collect::<BTreeMap<usize, FieldIdent>>()
                .into_values()
                .map(|source_ident| (quote! { #source_ident }, quote! { #source_ident.into() }))
                .unzip();

            quote! {
                #source_enum::#source_variant { #(#source_fields),* } =>
                #target_enum::#target_variant(#(#target_fields),*),
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
                let variant_mapping = compute_variant_mapping(
                    target_enum,
                    &target_variant,
                    fields_annotations,
                    fields,
                    source_variant,
                )?;

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

fn compute_variant_mapping(
    target_enum: &ContainerIdent,
    target_variant: &VariantIdent,
    fields_annotations: BTreeMap<FieldRef, FieldAnnotation>,
    fields: &Fields,
    source_variant: VariantIdent,
) -> syn::Result<VariantMapping> {
    match (
        fields,
        fields_annotations
            .first_key_value()
            .map(|(_, field_annotation)| &field_annotation.target_field),
    ) {
        (Fields::Unit, None) => Ok(VariantMapping::UnitToUnit { source_variant }),
        (Fields::Unit, Some(_)) => panic!("A unit variant cannot have field annotations"),
        (Fields::Unnamed(_), None) | (Fields::Unnamed(_), Some(FieldRef::FieldPos(_))) => {
            compute_tuple_to_tuple_variant_mapping(fields_annotations, source_variant)
        }
        (Fields::Named(_), None) | (Fields::Named(_), Some(FieldRef::FieldIdent(_))) => {
            compute_struct_to_struct_variant_mapping(fields_annotations, source_variant)
        }
        (Fields::Unnamed(fields), Some(FieldRef::FieldIdent(_))) => {
            compute_tuple_to_struct_variant_mapping(
                target_enum,
                target_variant,
                fields_annotations,
                fields,
                source_variant,
            )
        }
        (Fields::Named(fields), Some(FieldRef::FieldPos(_))) => {
            compute_struct_to_tuple_variant_mapping(
                target_enum,
                target_variant,
                fields_annotations,
                fields,
                source_variant,
            )
        }
    }
}

fn compute_tuple_to_tuple_variant_mapping(
    fields_annotations: BTreeMap<FieldRef, FieldAnnotation>,
    source_variant: VariantIdent,
) -> syn::Result<VariantMapping> {
    let fields_mapping = fields_annotations
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
                "Unexpected mapping to named field while another field mapped to a positional field.",
            )),
        })
        .collect::<syn::Result<_>>()?;

    Ok(VariantMapping::TupleToTuple {
        source_variant,
        fields_mapping,
    })
}

fn compute_struct_to_struct_variant_mapping(
    fields_annotations: BTreeMap<FieldRef, FieldAnnotation>,
    source_variant: VariantIdent,
) -> syn::Result<VariantMapping> {
    let fields_mapping = fields_annotations
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
                "Unexpected mapping to positional field while another field mapped to a named field.",
            )),
        })
        .collect::<syn::Result<_>>()?;

    Ok(VariantMapping::StructToStruct {
        source_variant,
        fields_mapping,
    })
}

fn compute_struct_to_tuple_variant_mapping(
    target_enum: &ContainerIdent,
    target_variant: &VariantIdent,
    fields_annotations: BTreeMap<FieldRef, FieldAnnotation>,
    fields: &FieldsNamed,
    source_variant: VariantIdent,
) -> syn::Result<VariantMapping> {
    let fields_mapping = fields_annotations
        .into_iter()
        .map(|source_to_target| match source_to_target {
            (FieldRef::FieldPos(_), _) => {
                panic!("Source is a struct variant but got positional fields")
            },
            (
                FieldRef::FieldIdent(source_ident),
                FieldAnnotation {
                    target_field: FieldRef::FieldPos(target_pos),
                    ..
                },
            ) => Ok((source_ident, target_pos)),
            (FieldRef::FieldIdent(_), FieldAnnotation { target_field: FieldRef::FieldIdent(_), field_span, .. }) => {
                Err(syn::Error::new(
                    field_span,
                    "Unexpected mapping to named field while another field mapped to a positional field.",
                ))
            },
        })
        .collect::<syn::Result<HashMap<FieldIdent, usize>>>()?;

    for field in fields.named.iter() {
        if !fields_mapping.contains_key(&FieldIdent(
            field.ident.clone().expect("Named fields have idents"),
        )) {
            Err(syn::Error::new(
                field.span(),
                format!(
                    "Missing required mapping to named field for {target_enum}::{target_variant}"
                ),
            ))?;
        }
    }

    Ok(VariantMapping::StructToTuple {
        source_variant,
        fields_mapping,
    })
}

fn compute_tuple_to_struct_variant_mapping(
    target_enum: &ContainerIdent,
    target_variant: &VariantIdent,
    fields_annotations: BTreeMap<FieldRef, FieldAnnotation>,
    fields: &FieldsUnnamed,
    source_variant: VariantIdent,
) -> syn::Result<VariantMapping> {
    let fields_mapping = fields_annotations
        .into_iter()
        .map(|source_to_target| match source_to_target {
            (FieldRef::FieldIdent(_), _) => {
                panic!("Source is a tuple variant but got named fields")
            },
            (
                FieldRef::FieldPos(source_pos),
                FieldAnnotation {
                    target_field: FieldRef::FieldIdent(target_ident),
                    ..
                },
            ) => Ok((source_pos, target_ident)),
            (FieldRef::FieldPos(_), FieldAnnotation { target_field: FieldRef::FieldPos(_), field_span, .. }) => {
                Err(syn::Error::new(
                    field_span,
                    "Unexpected mapping to positional field while another field mapped to a named field.",
                ))
            },
        })
        .collect::<syn::Result<HashMap<usize, FieldIdent>>>()?;

    for (pos, field) in fields.unnamed.iter().enumerate() {
        if !fields_mapping.contains_key(&pos) {
            Err(syn::Error::new(
                field.span(),
                format!(
                    "Missing required mapping to named field for {target_enum}::{target_variant}"
                ),
            ))?;
        }
    }

    Ok(VariantMapping::TupleToStruct {
        source_variant,
        fields_mapping,
    })
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
