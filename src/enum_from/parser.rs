use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;

use crate::idents::{ContainerIdent, FieldIdent, FieldRef, VariantIdent};
use crate::parse::{self, Attribute, Data, DeriveInput, Field, Fields, ParseStream, Variant};

/// A "dumb" parser of the EnumFrom annotations
/// There is no check of consistency between annotations here.
pub struct ParsedEnumFrom {
    pub target_enum: ContainerIdent,
    pub container_annotations: Vec<ContainerAnnotation>,
    pub variants_annotations: HashMap<Variant, VariantAnnotations>,
}

impl ParsedEnumFrom {
    pub fn parse(input: TokenStream) -> parse::Result<ParsedEnumFrom> {
        let derive_input: DeriveInput = parse::parse_derive_input(input)?;

        let data_enum = match derive_input.data {
            Data::Enum(data) => data,
            Data::Other(span) => {
                return Err(parse::Error::new(span, "EnumFrom can only be derived for enums"));
            }
        };

        let target_enum = ContainerIdent(derive_input.ident);
        let container_annotations = extract_container_annotations(&derive_input.attrs)?;
        let variants_annotations = extract_variants_annotations(data_enum.variants)?;

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

fn parse_variant_annotation(stream: &mut ParseStream) -> parse::Result<VariantAnnotation> {
    let span = stream.span();
    let first_ident = stream.parse_ident()?;

    // Check if there's a :: following
    if stream.peek_punct() == Some(':') {
        stream.parse_punct(':')?;
        stream.parse_punct(':')?;
        let variant_ident = stream.parse_ident()?;

        // Check for extra segments (like Enum::Variant::Extra) which are invalid
        if stream.peek_punct() == Some(':') {
            // Consume the rest to get the end span
            stream.parse_punct(':')?;
            stream.parse_punct(':')?;
            let last = stream.parse_ident()?;
            return Err(parse::Error::new_span_range(
                span,
                last.span(),
                "Expected Enum or Enum::Variant",
            ));
        }

        Ok(VariantAnnotation::EnumVariant {
            span,
            enum_ident: ContainerIdent(first_ident),
            variant_ident: VariantIdent(variant_ident),
        })
    } else {
        Ok(VariantAnnotation::EnumOnly {
            span,
            enum_ident: ContainerIdent(first_ident),
        })
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

fn parse_field_annotation(stream: &mut ParseStream) -> parse::Result<FieldAnnotation> {
    let enum_span = stream.span();
    let enum_ident = stream.parse_ident()?;

    stream.parse_punct(':')?;
    stream.parse_punct(':')?;

    let variant_span = stream.span();
    let variant_ident = stream.parse_ident()?;

    // Check for `.` - if we see `::` instead, give a helpful error
    if stream.peek_punct() == Some(':') {
        // Consume the rest to get the end span
        stream.parse_punct(':')?;
        stream.parse_punct(':')?;
        let last = stream.parse_ident()?;
        return Err(parse::Error::new_span_range(
            enum_span,
            last.span(),
            "Expected SourceEnum::SourceVariant.field_name",
        ));
    }
    stream.parse_punct('.')?;

    let field_span = stream.span();
    let source_field = if let Some(ident) = stream.peek_ident() {
        let ident = ident.clone();
        stream.parse_ident()?;
        FieldRef::FieldIdent(FieldIdent(ident))
    } else if stream.peek_literal().is_some() {
        let lit = stream.parse_literal()?;
        let pos = parse::parse_int(&lit)?;
        FieldRef::FieldPos(pos)
    } else {
        return Err(parse::Error::new(
            field_span,
            "Expected either a field identifier or a field position",
        ));
    };

    Ok(FieldAnnotation {
        enum_span,
        variant_span,
        field_span,
        source_enum: ContainerIdent(enum_ident),
        source_variant: VariantIdent(variant_ident),
        source_field,
    })
}

fn extract_container_annotations(
    container_attrs: &[Attribute],
) -> parse::Result<Vec<ContainerAnnotation>> {
    let res = container_attrs
        .iter()
        .filter(|attr| attr.path_is_ident("enum_from"))
        .map(|attr| {
            let build_err = || {
                parse::Error::new(
                    attr.span,
                    "expected a list of source enums, for example #[enum_from(Source1, Source2)]",
                )
            };

            if attr.is_list() {
                let args = attr.parse_args()?;
                let mut stream = ParseStream::new(args);

                if stream.is_empty() {
                    return Err(build_err());
                }

                let idents = stream.parse_terminated(|s| s.parse_ident())?;
                stream.expect_empty()?;

                if idents.is_empty() {
                    return Err(build_err());
                }

                Ok(idents
                    .into_iter()
                    .map(ContainerIdent)
                    .map(ContainerAnnotation)
                    .collect::<Vec<_>>())
            } else {
                Err(build_err())
            }
        })
        .collect::<parse::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    Ok(res)
}

fn extract_variants_annotations(
    variants: Vec<Variant>,
) -> parse::Result<HashMap<Variant, VariantAnnotations>> {
    let res = variants
        .into_iter()
        .map(|variant| {
            extract_variant_annotations(&variant).map(|annotations| (variant, annotations))
        })
        .collect::<parse::Result<Vec<_>>>()?;
    Ok(res.into_iter().collect())
}

fn extract_variant_annotations(variant: &Variant) -> parse::Result<VariantAnnotations> {
    let variant_annotations = variant
        .attrs
        .iter()
        .filter(|attr| attr.path_is_ident("enum_from"))
        .map(|attr| {
            let build_err = || {
                parse::Error::new(
                    attr.span,
                    "expected either #[enum_from] (if there is no ambiguity) or a list of variants, for example #[enum_from(Source1::VariantA, Source2::VariantB)]",
                )
            };

            if attr.is_path() {
                Ok(vec![VariantAnnotation::Nothing { span: attr.span }])
            } else if attr.is_list() {
                let args = attr.parse_args()?;
                let mut stream = ParseStream::new(args);

                if stream.is_empty() {
                    return Err(build_err());
                }

                let annotations = stream.parse_terminated(parse_variant_annotation)?;
                stream.expect_empty()?;

                if annotations.is_empty() {
                    return Err(build_err());
                }

                Ok(annotations)
            } else {
                Err(build_err())
            }
        })
        .collect::<parse::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let fields_annotations = match &variant.fields {
        Fields::Unit => HashMap::new(),
        Fields::Named(fields) => fields
            .iter()
            .map(|field| {
                let annotations = extract_field_annotations(field)?;
                let ident = field
                    .ident
                    .as_ref()
                    .expect("A named field should always have an ident")
                    .clone();
                Ok((FieldRef::FieldIdent(FieldIdent(ident)), annotations))
            })
            .collect::<parse::Result<Vec<_>>>()?
            .into_iter()
            .collect(),
        Fields::Unnamed(fields) => fields
            .iter()
            .enumerate()
            .map(|(pos, field)| {
                let annotations = extract_field_annotations(field)?;
                Ok((FieldRef::FieldPos(pos), annotations))
            })
            .collect::<parse::Result<Vec<_>>>()?
            .into_iter()
            .collect(),
    };

    Ok(VariantAnnotations {
        variant_annotations,
        fields_annotations,
    })
}

fn extract_field_annotations(field: &Field) -> parse::Result<FieldAnnotations> {
    let fields_annotations = field.attrs.iter()
        .filter(|attr| attr.path_is_ident("enum_from"))
        .map(|attr| {
            let build_err = || {
                parse::Error::new(
                    attr.span,
                    "expected a list of field names, for example #[enum_from(Source1::VariantA.field_x, Source2::VariantB.field_y)]",
                )
            };

            if attr.is_path() || attr.is_name_value() {
                Err(build_err())
            } else if attr.is_list() {
                let args = attr.parse_args()?;
                let mut stream = ParseStream::new(args);

                if stream.is_empty() {
                    return Err(build_err());
                }

                let annotations = stream.parse_terminated(parse_field_annotation)?;
                stream.expect_empty()?;

                if annotations.is_empty() {
                    return Err(build_err());
                }

                Ok(annotations)
            } else {
                Err(build_err())
            }
        }).collect::<parse::Result<Vec<Vec<FieldAnnotation>>>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(FieldAnnotations {
        fields_annotations,
        field_span: field.span,
    })
}
