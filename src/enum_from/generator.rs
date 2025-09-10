use std::collections::{BTreeMap, HashMap};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Fields, FieldsNamed, FieldsUnnamed, Variant, spanned::Spanned};

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
    UnitToUnit {
        target_variant: VariantIdent,
    },
    TupleToTuple {
        target_variant: VariantIdent,
        fields_mapping: HashMap<usize, usize>,
    },
    TupleToStruct {
        target_variant: VariantIdent,
        fields_mapping: HashMap<FieldIdent, usize>,
    },
    StructToStruct {
        target_variant: VariantIdent,
        fields_mapping: HashMap<FieldIdent, FieldIdent>,
    },
    StructToTuple {
        target_variant: VariantIdent,
        fields_mapping: HashMap<usize, FieldIdent>,
    },
}

impl VariantMapping {
    fn target_variant(&self) -> &VariantIdent {
        match self {
            VariantMapping::UnitToUnit { target_variant } => target_variant,
            VariantMapping::TupleToTuple { target_variant, .. } => target_variant,
            VariantMapping::TupleToStruct { target_variant, .. } => target_variant,
            VariantMapping::StructToStruct { target_variant, .. } => target_variant,
            VariantMapping::StructToTuple { target_variant, .. } => target_variant,
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
        (Fields::Unit, VariantMapping::UnitToUnit { target_variant }) => {
            quote! { #source_enum::#source_variant => #target_enum::#target_variant, }
        }
        (
            Fields::Unnamed(fields),
            VariantMapping::TupleToTuple {
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
            Fields::Unnamed(fields),
            VariantMapping::StructToTuple {
                target_variant,
                fields_mapping,
            },
        ) => {
            let (source_fields, target_fields): (Vec<_>, Vec<_>) = (0..fields.unnamed.len())
                .map(|field_target_pos| {
                    let source_ident = fields_mapping
                        .get(&field_target_pos)
                        .expect("fields_mapping exhaustiveness should have been checked");
                    (quote! { #source_ident }, quote! { #source_ident.into() })
                })
                .unzip();
            quote! {
                #source_enum::#source_variant { #(#source_fields),* } =>
                #target_enum::#target_variant(#(#target_fields),*),
            }
        }
        (
            Fields::Named(fields),
            VariantMapping::StructToStruct {
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
        (
            Fields::Named(_),
            VariantMapping::TupleToStruct {
                target_variant,
                fields_mapping,
            },
        ) => {
            let (source_fields, target_fields): (Vec<_>, Vec<_>) = fields_mapping
                .into_iter()
                .map(|(target_ident, source_pos)| (source_pos, target_ident))
                .collect::<BTreeMap<usize, FieldIdent>>()
                .into_values()
                .map(|target_ident| {
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

        for (target_variant, mut variant_annotations) in variants_annotations {
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

                let fields_annotations = extract_fields_annotations(
                    &mut variant_annotations.fields_annotations,
                    &source_enum,
                    &source_variant,
                )?;
                let fields = &target_variant.fields;
                let target_variant = VariantIdent(target_variant.ident.clone());
                let variant_mapping = compute_variant_mapping(
                    &source_enum,
                    &source_variant,
                    fields_annotations,
                    fields,
                    target_variant,
                )?;

                variants_mapping.insert(source_variant, variant_mapping);
            }

            check_unused_fields_annotations(&source_enums, variant_annotations.fields_annotations)?;
            target_variants.insert(VariantIdent(target_variant.ident.clone()), target_variant);
        }

        Ok(EnumFromGenerator {
            source_enums,
            target_enum,
            target_variants,
        })
    }
}

fn compute_variant_mapping(
    source_enum: &ContainerIdent,
    source_variant: &VariantIdent,
    fields_annotations: BTreeMap<FieldRef, FieldAnnotation>,
    fields: &Fields,
    target_variant: VariantIdent,
) -> syn::Result<VariantMapping> {
    match (
        fields,
        fields_annotations
            .first_key_value()
            .map(|(_, field_annotation)| &field_annotation.source_field),
    ) {
        (Fields::Unit, None) => Ok(VariantMapping::UnitToUnit { target_variant }),
        (Fields::Unit, Some(_)) => panic!("A unit variant cannot have field annotations"),
        (Fields::Unnamed(_), None) | (Fields::Unnamed(_), Some(FieldRef::FieldPos(_))) => {
            compute_tuple_to_tuple_variant_mapping(fields_annotations, target_variant)
        }
        (Fields::Named(_), None) | (Fields::Named(_), Some(FieldRef::FieldIdent(_))) => {
            compute_struct_to_struct_variant_mapping(fields_annotations, target_variant)
        }
        (Fields::Unnamed(fields), Some(FieldRef::FieldIdent(_))) => {
            compute_struct_to_tuple_variant_mapping(
                source_enum,
                source_variant,
                fields_annotations,
                fields,
                target_variant,
            )
        }
        (Fields::Named(fields), Some(FieldRef::FieldPos(_))) => {
            compute_tuple_to_struct_variant_mapping(
                source_enum,
                source_variant,
                fields_annotations,
                fields,
                target_variant,
            )
        }
    }
}

fn compute_tuple_to_tuple_variant_mapping(
    fields_annotations: BTreeMap<FieldRef, FieldAnnotation>,
    target_variant: VariantIdent,
) -> syn::Result<VariantMapping> {
    let fields_mapping = fields_annotations
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
                "Unexpected mapping to named field while another field mapped to a positional field.",
            )),
        })
        .collect::<syn::Result<_>>()?;

    Ok(VariantMapping::TupleToTuple {
        target_variant,
        fields_mapping,
    })
}

fn compute_struct_to_struct_variant_mapping(
    fields_annotations: BTreeMap<FieldRef, FieldAnnotation>,
    target_variant: VariantIdent,
) -> syn::Result<VariantMapping> {
    let fields_mapping = fields_annotations
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
                "Unexpected mapping to positional field while another field mapped to a named field.",
            )),
        })
        .collect::<syn::Result<_>>()?;

    Ok(VariantMapping::StructToStruct {
        target_variant,
        fields_mapping,
    })
}

fn compute_struct_to_tuple_variant_mapping(
    source_enum: &ContainerIdent,
    source_variant: &VariantIdent,
    fields_annotations: BTreeMap<FieldRef, FieldAnnotation>,
    fields: &FieldsUnnamed,
    target_variant: VariantIdent,
) -> syn::Result<VariantMapping> {
    let fields_mapping = fields_annotations
        .into_iter()
        .map(|target_to_source| match target_to_source {
            (FieldRef::FieldIdent(_), _) => {
                panic!("Target is a tuple variant but got named fields")
            },
            (
                FieldRef::FieldPos(target_pos),
                FieldAnnotation {
                    source_field: FieldRef::FieldIdent(source_ident),
                    ..
                },
            ) => Ok((target_pos, source_ident)),
            (FieldRef::FieldPos(_), FieldAnnotation { source_field: FieldRef::FieldPos(_), field_span, .. }) => {
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
                    "Missing required mapping to named field for {source_enum}::{source_variant}"
                ),
            ))?;
        }
    }

    Ok(VariantMapping::StructToTuple {
        target_variant,
        fields_mapping,
    })
}

fn compute_tuple_to_struct_variant_mapping(
    source_enum: &ContainerIdent,
    source_variant: &VariantIdent,
    fields_annotations: BTreeMap<FieldRef, FieldAnnotation>,
    fields: &FieldsNamed,
    target_variant: VariantIdent,
) -> syn::Result<VariantMapping> {
    let fields_mapping = fields_annotations
        .into_iter()
        .map(|target_to_source| match target_to_source {
            (FieldRef::FieldPos(_), _) => {
                panic!("Target is a struct variant but got positional fields")
            },
            (
                FieldRef::FieldIdent(target_ident),
                FieldAnnotation {
                    source_field: FieldRef::FieldPos(source_pos),
                    ..
                },
            ) => Ok((target_ident, source_pos)),
            (FieldRef::FieldIdent(_), FieldAnnotation { source_field: FieldRef::FieldIdent(_), field_span, .. }) => Err(syn::Error::new(
                field_span,
                "Unexpected mapping to named field while another field mapped to a positional field.",
            )),
        })
        .collect::<syn::Result<HashMap<FieldIdent, usize>>>()?;

    for field in fields.named.iter() {
        if !fields_mapping.contains_key(&FieldIdent(
            field.ident.clone().expect("Named fields have idents"),
        )) {
            Err(syn::Error::new(
                field.span(),
                format!(
                    "Missing required mapping to named field for {source_enum}::{source_variant}"
                ),
            ))?;
        }
    }

    Ok(VariantMapping::TupleToStruct {
        target_variant,
        fields_mapping,
    })
}

fn check_unused_fields_annotations(
    source_enums: &HashMap<ContainerIdent, VariantsMapping>,
    fields_annotations: HashMap<FieldRef, FieldAnnotations>,
) -> syn::Result<()> {
    for field_annotations in fields_annotations.into_values() {
        for field_annotation in field_annotations.fields_annotations {
            if source_enums.contains_key(&field_annotation.source_enum) {
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
    source_enum: &ContainerIdent,
    source_variant: &VariantIdent,
) -> syn::Result<BTreeMap<FieldRef, FieldAnnotation>> {
    Ok(fields_annotations
        .iter_mut()
        .filter_map(|(target_field, field_annotations)| {
            let mut annotations = field_annotations
                .fields_annotations
                .extract_if(.., |field_annotation| {
                    field_annotation.source_enum == *source_enum
                        && field_annotation.source_variant == *source_variant
                })
                .collect::<Vec<_>>();
            let annotation = annotations.pop();
            if annotations.pop().is_some() {
                Some(Err(syn::Error::new(
                    field_annotations.field_span,
                    format!("Multiple mapping found for source enum `{source_enum}`"),
                )))
            } else {
                annotation.map(|annotation| Ok((target_field.clone(), annotation)))
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
