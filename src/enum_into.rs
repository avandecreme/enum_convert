use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Field, Fields, Ident, Meta, Path, Token, Variant,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned as _,
};

pub fn derive_enum_into_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    try_derive_enum_into_impl(input).unwrap_or_else(|err| err.into_compile_error().into())
}

fn try_derive_enum_into_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    let source_enum = &input.ident;

    let data = match &input.data {
        Data::Enum(data) => data,
        Data::Struct(_) | Data::Union(_) => panic!("EnumInto can only be derived for enums"),
    };

    let target_enums = extract_target_enums(&input.attrs)?;

    // Generate match arms for all variants (enum_into attribute is optional)
    let match_arms = data
        .variants
        .iter()
        .map(|variant| generate_variant_match_arms(source_enum, &target_enums, variant))
        .collect::<syn::Result<Vec<Vec<_>>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<VariantMatchArm>>();

    // Group match arms by target enum
    let mut arms_by_enum: std::collections::HashMap<String, Vec<_>> =
        std::collections::HashMap::new();

    for VariantMatchArm {
        target_enum,
        match_arm,
    } in &match_arms
    {
        let enum_key = quote!(#target_enum).to_string();
        arms_by_enum
            .entry(enum_key)
            .or_default()
            .push(match_arm.clone());
    }

    // Generate separate impl blocks for each target enum
    let impl_blocks = target_enums.iter().map(|target_enum| {
        let enum_key = quote!(#target_enum).to_string();
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
    target_enum: Path,
    match_arm: proc_macro2::TokenStream,
}

fn generate_variant_match_arms(
    source_enum: &Ident,
    target_enums: &[Path],
    variant: &Variant,
) -> syn::Result<Vec<VariantMatchArm>> {
    let variant_targets = extract_enum_into_targets(target_enums, variant)?;
    let targets = if variant_targets.is_empty() {
        // Default behavior: map to all target enums with same variant name
        target_enums
            .iter()
            .map(|enum_name| (enum_name.clone(), variant.ident.clone()))
            .collect()
    } else {
        variant_targets
    };

    let source_variant_name = &variant.ident;

    targets.into_iter().map(|(target_enum_name, target_variant_name)| {
        let arm = match &variant.fields {
            Fields::Unit => Ok(quote! {
                #source_enum::#source_variant_name => #target_enum_name::#target_variant_name,
            }),
            Fields::Unnamed(fields) => {
                let field_names: Vec<_> = (0..fields.unnamed.len())
                    .map(|i| quote::format_ident!("field_{}", i))
                    .collect();
                let field_conversions: Vec<_> = field_names.iter()
                    .map(|name| quote! { #name.into() })
                    .collect();
                Ok(quote! {
                    #source_enum::#source_variant_name(#(#field_names),*) => #target_enum_name::#target_variant_name(#(#field_conversions),*),
                })
            },
            Fields::Named(fields) => {
                generate_field_mappings(
                    fields,
                    &target_enum_name,
                ).map(|(source_patterns, target_assignments)| {
                    quote! {
                        #source_enum::#source_variant_name { #(#source_patterns),* } => #target_enum_name::#target_variant_name { #(#target_assignments),* },
                    }
                })
            }
        };
        arm.map(|arm| VariantMatchArm { target_enum: target_enum_name, match_arm: arm})
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
    targets: Punctuated<VariantTarget, Token![,]>,
}

impl Parse for VariantAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(VariantAttributeArgs {
            targets: Punctuated::parse_terminated(input)?,
        })
    }
}

struct FieldAttributeArgs {
    fields: Punctuated<FieldMapping, Token![,]>,
}

impl Parse for FieldAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(FieldAttributeArgs {
            fields: Punctuated::parse_terminated(input)?,
        })
    }
}

struct FieldMapping {
    target_enum: Path,
    field_name: Ident,
}

impl Parse for FieldMapping {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        if path.segments.len() == 2 {
            let target_enum = Path {
                leading_colon: None,
                segments: path.segments.iter().take(1).cloned().collect(),
            };
            let field_name = path.segments.last().unwrap().ident.clone();
            Ok(FieldMapping {
                target_enum,
                field_name,
            })
        } else {
            Err(syn::Error::new_spanned(
                path,
                "Expected TargetEnum::field_name",
            ))
        }
    }
}

enum VariantTarget {
    EnumOnly(Path),
    EnumVariant(Path, Ident),
}

impl Parse for VariantTarget {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        if path.segments.len() == 1 {
            Ok(VariantTarget::EnumOnly(path))
        } else if path.segments.len() == 2 {
            let enum_name = Path {
                leading_colon: None,
                segments: path.segments.iter().take(1).cloned().collect(),
            };
            let variant_name = path.segments.last().unwrap().ident.clone();
            Ok(VariantTarget::EnumVariant(enum_name, variant_name))
        } else {
            Err(syn::Error::new_spanned(
                path,
                "Expected Enum or Enum::Variant",
            ))
        }
    }
}

fn extract_target_enums(attrs: &[Attribute]) -> Result<Vec<Path>, syn::Error> {
    let res = attrs
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
        panic!("enum_into attribute with target enum names is required");
    }
    Ok(res)
}

fn extract_enum_into_targets(
    target_enums: &[Path],
    source_variant: &Variant,
) -> syn::Result<Vec<(Path, Ident)>> {
    source_variant.attrs.iter().filter(|attr| attr.path().is_ident("enum_into"))
        .map(|attr| {
            match &attr.meta {
                Meta::Path(_) => {
                    // #[enum_into] without arguments - return empty to use default fallback
                    Ok(Vec::new())
                }
                Meta::List(meta_list) => meta_list.parse_args::<VariantAttributeArgs>().and_then(|args| {
                    args
                        .targets
                        .into_iter()
                        .map(|target| {
                            let (enum_name, variant_name) = match target {
                                VariantTarget::EnumOnly(enum_name) => {
                                    (enum_name, source_variant.ident.clone())
                                }
                                VariantTarget::EnumVariant(enum_name, variant_name) => {
                                    (enum_name, variant_name)
                                }
                            };
                            if target_enums.iter().any(|target_enum| {
                                quote!(#target_enum).to_string() ==  quote!(#enum_name).to_string()
                            }) {
                                Ok((enum_name, variant_name))
                            } else {
                                Err(syn::Error::new(
                                    attr.span(),
                                    format!(
                                        "target enum `{}` is not specified in this enum's #[enum_into] annotation",
                                        quote!(#enum_name),
                                    )
                                ))
                            }

                        })
                        .collect()
                }),
                Meta::NameValue(_) => Err(syn::Error::new(
                    attr.span(),
                    "expected a list of variants, for example #[enum_into(Target1::VariantA, Target2::VariantB)].\n\
                    If there is only one target enum and the variant names are identical between source and target, #[enum_into] can be omitted.",
                )),
            }
        }).collect::<syn::Result<Vec<_>>>()
        .map(|vec| vec.into_iter().flatten().collect())
}

fn extract_field_mapping(field: &Field, target_enum: &Path) -> syn::Result<Ident> {
    let mut field_names = field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("enum_into"))
        .map(|attr| {
            let build_err = || {
                syn::Error::new(
                    attr.span(),
                    "expected a list of field names, for example #[enum_into(Target1::field_a, Target2::field_b)]",
                )
            };

            match &attr.meta {
                Meta::Path(_) | Meta::NameValue(_) => Err(build_err()),
                Meta::List(meta_list) => {
                    meta_list.parse_args::<FieldAttributeArgs>().and_then(|args| {
                        if args.fields.empty_or_trailing() {
                            Err(build_err())
                        } else {
                            Ok(args.fields.into_iter().collect::<Vec<FieldMapping>>())
                        }
                    })
                }
            }
        }).collect::<Result<Vec<Vec<FieldMapping>>, syn::Error>>()?
        .into_iter()
        .flatten()
        .filter_map(|FieldMapping { target_enum: enum_name, field_name }| {
            // FIXME: that quote + to_string is suspicious
            if quote!(#enum_name).to_string() == quote!(#target_enum).to_string()
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
                    "Multiple mapping found for target enum `{}`",
                    quote!(#target_enum)
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
    target_enum: &Path,
) -> syn::Result<(Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>)> {
    let mut source_patterns = Vec::new();
    let mut target_assignments = Vec::new();

    for field in &fields.named {
        let source_field_name = field.ident.as_ref().unwrap();

        let target_field_name = extract_field_mapping(field, target_enum)?;
        source_patterns.push(quote! { #source_field_name });
        target_assignments.push(quote! { #target_field_name: #source_field_name.into() });
    }

    Ok((source_patterns, target_assignments))
}
