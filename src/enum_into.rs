use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Field, Fields, Ident, Meta, Path, Token, parse::Parse,
    parse::ParseStream, parse_macro_input, punctuated::Punctuated,
};

pub fn derive_into_variants_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let source_enum = &input.ident;

    // Extract the target enum names from the attribute
    let target_enums = extract_target_enums(&input.attrs)
        .expect("enum_into attribute with target enum names is required");

    // Only work with enums
    let data = match &input.data {
        Data::Enum(data) => data,
        _ => panic!("IntoVariants can only be derived for enums"),
    };

    // Generate match arms for all variants (enum_into attribute is optional)
    let match_arms: Vec<_> = data.variants.iter().flat_map(|variant| {

        let variant_targets = extract_enum_into_targets(&variant.attrs, &variant.ident);
        let targets = if variant_targets.is_empty() {
            // Default behavior: map to all target enums with same variant name
            target_enums.iter().map(|enum_name| (enum_name.clone(), variant.ident.clone())).collect()
        } else {
            variant_targets
        };

        let source_variant_name = &variant.ident;

        targets.into_iter().map(|(target_enum_name, target_variant_name)| {
            let arm = match &variant.fields {
                Fields::Unit => quote! {
                    #source_enum::#source_variant_name => #target_enum_name::#target_variant_name,
                },
                Fields::Unnamed(fields) => {
                    let field_names: Vec<_> = (0..fields.unnamed.len())
                        .map(|i| quote::format_ident!("field_{}", i))
                        .collect();
                    let field_conversions: Vec<_> = field_names.iter()
                        .map(|name| quote! { #name.into() })
                        .collect();
                    quote! {
                        #source_enum::#source_variant_name(#(#field_names),*) => #target_enum_name::#target_variant_name(#(#field_conversions),*),
                    }
                },
                Fields::Named(fields) => {
                    let (source_patterns, target_assignments) = generate_field_mappings(
                        fields,
                        &target_enum_name,
                        &target_variant_name,
                        source_variant_name
                    );
                    quote! {
                        #source_enum::#source_variant_name { #(#source_patterns),* } => #target_enum_name::#target_variant_name { #(#target_assignments),* },
                    }
                }
            };
            (target_enum_name, target_variant_name, source_variant_name.clone(), arm)
        }).collect::<Vec<_>>()
    }).collect();

    // Group match arms by target enum
    let mut arms_by_enum: std::collections::HashMap<String, Vec<_>> =
        std::collections::HashMap::new();

    for (target_enum_name, _target_variant_name, _source_variant_name, arm) in &match_arms {
        let enum_key = quote!(#target_enum_name).to_string();
        arms_by_enum.entry(enum_key).or_default().push(arm.clone());
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

    TokenStream::from(expanded)
}

struct ContainerIntoVariantsArgs {
    enums: Punctuated<Path, Token![,]>,
}

impl Parse for ContainerIntoVariantsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ContainerIntoVariantsArgs {
            enums: Punctuated::parse_terminated(input)?,
        })
    }
}

struct VariantIntoVariantsArgs {
    targets: Punctuated<VariantTarget, Token![,]>,
}

impl Parse for VariantIntoVariantsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(VariantIntoVariantsArgs {
            targets: Punctuated::parse_terminated(input)?,
        })
    }
}

struct FieldIntoVariantsArgs {
    fields: Punctuated<FieldTarget, Token![,]>,
}

impl Parse for FieldIntoVariantsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(FieldIntoVariantsArgs {
            fields: Punctuated::parse_terminated(input)?,
        })
    }
}

enum FieldTarget {
    EnumField(Path, Ident),
}

impl Parse for FieldTarget {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        if path.segments.len() == 2 {
            let enum_name = Path {
                leading_colon: None,
                segments: path.segments.iter().take(1).cloned().collect(),
            };
            let field_name = path.segments.last().unwrap().ident.clone();
            Ok(FieldTarget::EnumField(enum_name, field_name))
        } else {
            Err(syn::Error::new_spanned(path, "Expected Enum::field_name"))
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

fn extract_target_enums(attrs: &[Attribute]) -> Option<Vec<Path>> {
    for attr in attrs {
        if attr.path().is_ident("enum_into") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(args) = meta_list.parse_args::<ContainerIntoVariantsArgs>() {
                    return Some(args.enums.into_iter().collect());
                }
            }
        }
    }
    None
}


fn extract_enum_into_targets(
    attrs: &[Attribute],
    source_variant: &Ident,
) -> Vec<(Path, Ident)> {
    for attr in attrs {
        if attr.path().is_ident("enum_into") {
            match &attr.meta {
                Meta::Path(_) => {
                    // #[enum_into] without arguments - return empty to use default fallback
                    return Vec::new();
                }
                Meta::List(meta_list) => {
                    if let Ok(args) = meta_list.parse_args::<VariantIntoVariantsArgs>() {
                        return args
                            .targets
                            .into_iter()
                            .map(|target| match target {
                                VariantTarget::EnumOnly(enum_name) => {
                                    (enum_name, source_variant.clone())
                                }
                                VariantTarget::EnumVariant(enum_name, variant_name) => {
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

fn extract_field_mapping(field: &Field, target_enum: &Path) -> Option<Ident> {
    for attr in &field.attrs {
        if attr.path().is_ident("enum_into") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(args) = meta_list.parse_args::<FieldIntoVariantsArgs>() {
                    // Find the field mapping for this target enum
                    for FieldTarget::EnumField(enum_name, field_name) in args.fields {
                        if quote!(#enum_name).to_string() == quote!(#target_enum).to_string() {
                            return Some(field_name);
                        }
                    }
                }
            }
        }
    }
    None
}

fn generate_field_mappings(
    fields: &syn::FieldsNamed,
    target_enum: &Path,
    _target_variant: &Ident,
    _source_variant: &Ident,
) -> (Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>) {
    let mut source_patterns = Vec::new();
    let mut target_assignments = Vec::new();

    for field in &fields.named {
        let source_field_name = field.ident.as_ref().unwrap();

        // Check if there's a field mapping for this target enum
        if let Some(target_field_name) = extract_field_mapping(field, target_enum) {
            // Use mapped field name for target
            source_patterns.push(quote! { #source_field_name });
            target_assignments.push(quote! { #target_field_name: #source_field_name.into() });
        } else {
            // Use same field name in both source and target
            source_patterns.push(quote! { #source_field_name });
            target_assignments.push(quote! { #source_field_name: #source_field_name.into() });
        }
    }

    (source_patterns, target_assignments)
}