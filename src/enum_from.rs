use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Field, Fields, Ident, Meta, Path, Token, Variant,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
};

pub fn derive_enum_from_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    try_derive_enum_from_impl(input).unwrap_or_else(|err| err.into_compile_error().into())
}

fn try_derive_enum_from_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    let target_enum = &input.ident;

    let data = match &input.data {
        Data::Enum(data) => data,
        Data::Struct(_) | Data::Union(_) => panic!("EnumFrom can only be derived for enums"),
    };

    let source_enums = extract_source_enums(&input.attrs)?;

    // Generate match arms only for variants marked with #[enum_from]
    let match_arms = data
        .variants
        .iter()
        .map(|variant| generate_variant_match_arms(target_enum, &source_enums, variant))
        .collect::<syn::Result<Vec<Vec<_>>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<VariantMatchArm>>();

    // Group match arms by source enum
    let mut arms_by_enum: std::collections::HashMap<String, Vec<_>> =
        std::collections::HashMap::new();

    for VariantMatchArm {
        source_enum,
        match_arm,
    } in &match_arms
    {
        let enum_key = quote!(#source_enum).to_string();
        arms_by_enum
            .entry(enum_key)
            .or_default()
            .push(match_arm.clone());
    }

    // Generate separate impl blocks for each source enum
    let impl_blocks = source_enums.iter().map(|source_enum| {
        let enum_key = quote!(#source_enum).to_string();
        let empty_arms = Vec::new();
        let arms = arms_by_enum.get(&enum_key).unwrap_or(&empty_arms);

        quote! {
            impl From<#source_enum> for #target_enum {
                fn from(value: #source_enum) -> Self {
                    match value {
                        #(#arms)*
                    }
                }
            }
        }
    });

    let expanded = quote! {
        #(#impl_blocks)*
    };

    Ok(TokenStream::from(expanded))
}

struct VariantMatchArm {
    source_enum: Path,
    match_arm: proc_macro2::TokenStream,
}

fn generate_variant_match_arms(
    target_enum: &Ident,
    source_enums: &[Path],
    variant: &Variant,
) -> syn::Result<Vec<VariantMatchArm>> {
    if !has_enum_from_attr(&variant.attrs) {
        return Ok(Vec::new());
    }

    let variant_sources = extract_enum_from_sources(source_enums, variant)?;
    let sources = if variant_sources.is_empty() {
        // Fallback to default behavior
        if source_enums.len() == 1 {
            vec![(source_enums[0].clone(), variant.ident.clone())]
        } else {
            return Err(syn::Error::new(
                variant.span(),
                "When multiple source enums are specified, each variant must specify from which enum to convert with #[enum_from(Enum)] or #[enum_from(Enum::Variant)]",
            ));
        }
    } else {
        variant_sources
    };

    let target_variant_name = &variant.ident;

    sources.into_iter().map(|(source_enum_name, source_variant_name)| {
        let arm = match &variant.fields {
            Fields::Unit => Ok(quote! {
                #source_enum_name::#source_variant_name => #target_enum::#target_variant_name,
            }),
            Fields::Unnamed(fields) => {
                let field_names: Vec<_> = (0..fields.unnamed.len())
                    .map(|i| quote::format_ident!("field_{}", i))
                    .collect();
                let field_conversions: Vec<_> = field_names.iter()
                    .map(|name| quote! { #name.into() })
                    .collect();
                Ok(quote! {
                    #source_enum_name::#source_variant_name(#(#field_names),*) => #target_enum::#target_variant_name(#(#field_conversions),*),
                })
            },
            Fields::Named(fields) => {
                generate_field_mappings(
                    fields,
                    &source_enum_name,
                ).map(|(source_patterns, target_assignments)| {
                    quote! {
                        #source_enum_name::#source_variant_name { #(#source_patterns),* } => #target_enum::#target_variant_name { #(#target_assignments),* },
                    }
                })
            }
        };
        arm.map(|arm| VariantMatchArm { source_enum: source_enum_name, match_arm: arm })
    }).collect::<syn::Result<Vec<_>>>()
}

struct ContainerAttributeArgs {
    enums: Punctuated<Path, Token![,]>,
}

impl Parse for ContainerAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ContainerAttributeArgs {
            enums: Punctuated::parse_terminated(input)?,
        })
    }
}

struct VariantAttributeArgs {
    sources: Punctuated<VariantSource, Token![,]>,
}

impl Parse for VariantAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(VariantAttributeArgs {
            sources: Punctuated::parse_terminated(input)?,
        })
    }
}

struct FieldAttributeArgs(Punctuated<FieldMapping, Token![,]>);

impl Parse for FieldAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(FieldAttributeArgs(Punctuated::parse_terminated(input)?))
    }
}

struct FieldMapping {
    source_enum: Path,
    field_name: Ident,
}

impl Parse for FieldMapping {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        if path.segments.len() == 2 {
            let source_enum = Path {
                leading_colon: None,
                segments: path.segments.iter().take(1).cloned().collect(),
            };
            let field_name = path.segments.last().unwrap().ident.clone();
            Ok(FieldMapping {
                source_enum,
                field_name,
            })
        } else {
            Err(syn::Error::new_spanned(
                path,
                "Expected SourceEnum::field_name",
            ))
        }
    }
}

enum VariantSource {
    EnumOnly(Path),
    EnumVariant(Path, Ident),
}

impl Parse for VariantSource {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        if path.segments.len() == 1 {
            Ok(VariantSource::EnumOnly(path))
        } else if path.segments.len() == 2 {
            let enum_name = Path {
                leading_colon: None,
                segments: path.segments.iter().take(1).cloned().collect(),
            };
            let variant_name = path.segments.last().unwrap().ident.clone();
            Ok(VariantSource::EnumVariant(enum_name, variant_name))
        } else {
            Err(syn::Error::new_spanned(
                path,
                "Expected Enum or Enum::Variant",
            ))
        }
    }
}

fn extract_source_enums(attrs: &[Attribute]) -> syn::Result<Vec<Path>> {
    let res = attrs
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
                Meta::List(meta_list) => {
                    meta_list
                        .parse_args::<ContainerAttributeArgs>()
                        .and_then(|args| {
                            if args.enums.empty_or_trailing() {
                                Err(build_err())
                            } else {
                                Ok(args.enums.into_iter().collect::<Vec<Path>>())
                            }
                        })
                }
                Meta::Path(_) | Meta::NameValue(_) => Err(build_err()),
            }
        })
        .collect::<Result<Vec<Vec<Path>>, syn::Error>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    if res.is_empty() {
        // Panicking ensure that the error is on Derive(EnumFrom) instead of on the enum.
        panic!("enum_from attribute with source enum names is required");
    }
    Ok(res)
}

fn has_enum_from_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("enum_from"))
}

fn extract_enum_from_sources(
    source_enums: &[Path],
    target_variant: &Variant,
) -> syn::Result<Vec<(Path, Ident)>> {
    target_variant.attrs.iter().filter(|attr| attr.path().is_ident("enum_from"))
        .map(|attr| {
            match &attr.meta {
                Meta::Path(_) => {
                    // #[enum_from] without arguments - return empty to use default fallback
                    Ok(Vec::new())
                }
                Meta::List(meta_list) => meta_list.parse_args::<VariantAttributeArgs>().and_then(|args| {
                    args.sources
                        .into_iter()
                        .map(|source| {
                            let (enum_name, variant_name) = match source {
                                VariantSource::EnumOnly(enum_name) => {
                                    (enum_name, target_variant.ident.clone())
                                }
                                VariantSource::EnumVariant(enum_name, variant_name) => {
                                    (enum_name, variant_name)
                                }
                            };
                            if source_enums.iter().any(|source_enum| {
                                quote!(#source_enum).to_string() ==  quote!(#enum_name).to_string()
                            }) {
                                Ok((enum_name, variant_name))
                            } else {
                                Err(syn::Error::new(
                                    attr.span(),
                                    format!(
                                        "source enum `{}` is not specified in this enum's #[enum_from] annotation",
                                        quote!(#enum_name),
                                    )
                                ))
                            }
                        })
                        .collect()
                }),
                Meta::NameValue(_) => Err(syn::Error::new(
                    attr.span(),
                    "expected either #[enum_from] (if there is no ambiguity) or a list of variants, for example #[enum_from(Source1::VariantA, Source2::VariantB)]",
                )),
            }
        })
        .collect::<syn::Result<Vec<_>>>()
        .map(|vec| vec.into_iter().flatten().collect())
}

fn extract_field_mapping(field: &Field, source_enum: &Path) -> syn::Result<Ident> {
    let mut field_names = field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("enum_from"))
        .map(|attr| {
            let build_err = || {
                syn::Error::new(
                    attr.span(),
                    "expected a list of field names, for example #[enum_from(Source1::field_a, Source2::field_b)]",
                )
            };

            match &attr.meta {
                Meta::Path(_) | Meta::NameValue(_) => Err(build_err()),
                Meta::List(meta_list) => {
                    meta_list.parse_args::<FieldAttributeArgs>().and_then(|args| {
                        if args.0.empty_or_trailing() {
                            Err(build_err())
                        } else {
                            Ok(args.0.into_iter().collect::<Vec<FieldMapping>>())
                        }
                    })
                }
            }
        }).collect::<Result<Vec<Vec<FieldMapping>>, syn::Error>>()?
        .into_iter()
        .flatten()
        .filter_map(|FieldMapping {source_enum : enum_name, field_name }| {
            // FIXME: that quote + to_string is suspicious
            if quote!(#enum_name).to_string() == quote!(#source_enum).to_string()
            {
                Some(field_name)
            } else {
                None
            }
        });

    if let Some(field_name) = field_names.next() {
        if field_names.next().is_some() {
            Err(syn::Error::new(
                field.span(),
                format!(
                    "Multiple mapping found for source enum `{}`",
                    quote!(#source_enum)
                ),
            ))
        } else {
            Ok(field_name)
        }
    } else {
        // If no mapping found, default to the source field name
        Ok(field.ident.clone().expect(
            "Oops, there is a bug in enum_convert, please report it. Unexpected field without ident.",
        ))
    }
}

fn generate_field_mappings(
    fields: &syn::FieldsNamed,
    source_enum: &Path,
) -> syn::Result<(Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>)> {
    let mut source_patterns = Vec::new();
    let mut target_assignments = Vec::new();

    for field in &fields.named {
        let target_field_name = field.ident.as_ref().unwrap();

        let source_field_name = extract_field_mapping(field, source_enum)?;
        source_patterns.push(quote! { #source_field_name });
        target_assignments.push(quote! { #target_field_name: #source_field_name.into() });
    }

    Ok((source_patterns, target_assignments))
}
