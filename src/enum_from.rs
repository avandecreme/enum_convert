use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Field, Fields, Ident, Meta, Path, Token, parse::Parse,
    parse::ParseStream, parse_macro_input, punctuated::Punctuated,
};

pub fn derive_from_variants_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let target_enum = &input.ident;

    // Extract the source enum names from the attribute
    let source_enums = extract_source_enums(&input.attrs)
        .expect("enum_from attribute with source enum names is required");

    // Only work with enums
    let data = match &input.data {
        Data::Enum(data) => data,
        _ => panic!("FromVariants can only be derived for enums"),
    };

    // Generate match arms only for variants marked with #[enum_from]
    let match_arms: Vec<_> = data.variants.iter().flat_map(|variant| {
        // Check if this variant has the #[enum_from] attribute
        if !has_enum_from_attr(&variant.attrs) {
            return Vec::new();
        }

        let variant_sources = extract_enum_from_sources(&variant.attrs, &variant.ident);
        let sources = if variant_sources.is_empty() {
            // Fallback to default behavior
            if source_enums.len() == 1 {
                vec![(source_enums[0].clone(), variant.ident.clone())]
            } else {
                panic!("When multiple source enums are specified, each variant must specify which enum with #[enum_from(Enum)] or #[enum_from(Enum::Variant)]")
            }
        } else {
            variant_sources
        };

        let target_variant_name = &variant.ident;

        sources.into_iter().map(|(source_enum_name, source_variant_name)| {
            let arm = match &variant.fields {
                Fields::Unit => quote! {
                    #source_enum_name::#source_variant_name => #target_enum::#target_variant_name,
                },
                Fields::Unnamed(fields) => {
                    let field_names: Vec<_> = (0..fields.unnamed.len())
                        .map(|i| quote::format_ident!("field_{}", i))
                        .collect();
                    let field_conversions: Vec<_> = field_names.iter()
                        .map(|name| quote! { #name.into() })
                        .collect();
                    quote! {
                        #source_enum_name::#source_variant_name(#(#field_names),*) => #target_enum::#target_variant_name(#(#field_conversions),*),
                    }
                },
                Fields::Named(fields) => {
                    let (source_patterns, target_assignments) = generate_field_mappings(
                        fields,
                        &source_enum_name,
                        &source_variant_name
                    );
                    quote! {
                        #source_enum_name::#source_variant_name { #(#source_patterns),* } => #target_enum::#target_variant_name { #(#target_assignments),* },
                    }
                }
            };
            (source_enum_name, source_variant_name, target_variant_name.clone(), arm)
        }).collect::<Vec<_>>()
    }).collect();

    // Group match arms by source enum
    let mut arms_by_enum: std::collections::HashMap<String, Vec<_>> =
        std::collections::HashMap::new();

    for (source_enum_name, _source_variant_name, _target_variant_name, arm) in &match_arms {
        let enum_key = quote!(#source_enum_name).to_string();
        arms_by_enum.entry(enum_key).or_default().push(arm.clone());
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

    TokenStream::from(expanded)
}

struct ContainerFromVariantsArgs {
    enums: Punctuated<Path, Token![,]>,
}

impl Parse for ContainerFromVariantsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ContainerFromVariantsArgs {
            enums: Punctuated::parse_terminated(input)?,
        })
    }
}

struct VariantFromVariantsArgs {
    sources: Punctuated<VariantSource, Token![,]>,
}

impl Parse for VariantFromVariantsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(VariantFromVariantsArgs {
            sources: Punctuated::parse_terminated(input)?,
        })
    }
}

struct FieldFromVariantsArgs {
    fields: Punctuated<FieldSource, Token![,]>,
}

impl Parse for FieldFromVariantsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(FieldFromVariantsArgs {
            fields: Punctuated::parse_terminated(input)?,
        })
    }
}

enum FieldSource {
    EnumField(Path, Ident),
}

impl Parse for FieldSource {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        if path.segments.len() == 2 {
            let enum_name = Path {
                leading_colon: None,
                segments: path.segments.iter().take(1).cloned().collect(),
            };
            let field_name = path.segments.last().unwrap().ident.clone();
            Ok(FieldSource::EnumField(enum_name, field_name))
        } else {
            Err(syn::Error::new_spanned(path, "Expected Enum::field_name"))
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

fn extract_source_enums(attrs: &[Attribute]) -> Option<Vec<Path>> {
    for attr in attrs {
        if attr.path().is_ident("enum_from")
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(args) = meta_list.parse_args::<ContainerFromVariantsArgs>()
        {
            return Some(args.enums.into_iter().collect());
        }
    }
    None
}

fn has_enum_from_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("enum_from"))
}

fn extract_enum_from_sources(attrs: &[Attribute], target_variant: &Ident) -> Vec<(Path, Ident)> {
    for attr in attrs {
        if attr.path().is_ident("enum_from") {
            match &attr.meta {
                Meta::Path(_) => {
                    // #[enum_from] without arguments - return empty to use default fallback
                    return Vec::new();
                }
                Meta::List(meta_list) => {
                    if let Ok(args) = meta_list.parse_args::<VariantFromVariantsArgs>() {
                        return args
                            .sources
                            .into_iter()
                            .map(|source| match source {
                                VariantSource::EnumOnly(enum_name) => {
                                    (enum_name, target_variant.clone())
                                }
                                VariantSource::EnumVariant(enum_name, variant_name) => {
                                    (enum_name, variant_name)
                                }
                            })
                            .collect();
                    }
                }
                _ => continue,
            }
        }
    }
    Vec::new()
}

fn extract_field_mapping(field: &Field, source_enum: &Path) -> Option<Ident> {
    for attr in &field.attrs {
        if attr.path().is_ident("enum_from")
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(args) = meta_list.parse_args::<FieldFromVariantsArgs>()
        {
            // Find the field mapping for this source enum
            for FieldSource::EnumField(enum_name, field_name) in args.fields {
                if quote!(#enum_name).to_string() == quote!(#source_enum).to_string() {
                    return Some(field_name);
                }
            }
        }
    }
    None
}

fn generate_field_mappings(
    fields: &syn::FieldsNamed,
    source_enum: &Path,
    _source_variant: &Ident,
) -> (Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>) {
    let mut source_patterns = Vec::new();
    let mut target_assignments = Vec::new();

    for field in &fields.named {
        let target_field_name = field.ident.as_ref().unwrap();

        // Check if there's a field mapping for this source enum
        if let Some(source_field_name) = extract_field_mapping(field, source_enum) {
            // Use mapped field name from source
            source_patterns.push(quote! { #source_field_name });
            target_assignments.push(quote! { #target_field_name: #source_field_name.into() });
        } else {
            // Use same field name in both source and target
            source_patterns.push(quote! { #target_field_name });
            target_assignments.push(quote! { #target_field_name: #target_field_name.into() });
        }
    }

    (source_patterns, target_assignments)
}
