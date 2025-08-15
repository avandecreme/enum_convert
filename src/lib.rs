use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Meta, Path, Ident, punctuated::Punctuated, Token, parse::Parse, parse::ParseStream};

/// Derives `From<T>` for the annotated enum where T is a smaller enum with a subset of variants.
///
/// # Examples
///
/// ## Single source enum (source enum name optional in from_variants)
/// ```
/// use from_variants::FromVariants;
///
/// enum Smaller {
///     Unit,
///     Tuple(i32, &'static str),
///     DifferentName { x: i32, y: i32 }
/// }
///
/// #[derive(FromVariants)]
/// #[from_variants(Smaller)]
/// enum Bigger {
///     #[from_variants]
///     Unit,
///     #[from_variants]
///     Tuple(i64, String),
///     #[from_variants(Smaller::DifferentName)]
///     Struct { x: f64, y: f64 },
///     Extra
/// }
///
/// let smaller = Smaller::Unit;
/// let bigger: Bigger = smaller.into();
/// assert!(matches!(bigger, Bigger::Unit));
///
/// let smaller = Smaller::Tuple(42, "hello");
/// let bigger: Bigger = smaller.into();
/// assert!(matches!(bigger, Bigger::Tuple(42, ref s) if s == "hello"));
///
/// let smaller = Smaller::DifferentName { x: 1, y: 2 };
/// let bigger: Bigger = smaller.into();
/// assert!(matches!(bigger, Bigger::Struct { x, y } if x == 1.0 && y == 2.0));
/// ```
///
/// ## Multiple source enums (source enum name required in from_variants)
/// ```
/// use from_variants::FromVariants;
///
/// enum First {
///     Unit,
///     Tuple(i32, &'static str),
/// }
///
/// enum Second {
///     Empty,
///     Struct { x: i32, y: i32 },
/// }
///
/// #[derive(FromVariants)]
/// #[from_variants(First, Second)]
/// enum Bigger {
///     #[from_variants(First, Second::Empty)]
///     Unit,
///     #[from_variants(First)]
///     Tuple(i64, String),
///     #[from_variants(Second)]
///     Struct { x: f64, y: f64 },
///     Extra
/// }
///
/// let first = First::Unit;
/// let bigger: Bigger = first.into();
/// assert!(matches!(bigger, Bigger::Unit));
///
/// // Unit can also come from Second
/// let second = Second::Empty;
/// let bigger: Bigger = second.into();
/// assert!(matches!(bigger, Bigger::Unit));
///
/// let first = First::Tuple(42, "hello");
/// let bigger: Bigger = first.into();
/// assert!(matches!(bigger, Bigger::Tuple(42, ref s) if s == "hello"));
///
/// let second = Second::Struct { x: 1, y: 2 };
/// let bigger: Bigger = second.into();
/// assert!(matches!(bigger, Bigger::Struct { x, y } if x == 1.0 && y == 2.0));
/// ```
#[proc_macro_derive(FromVariants, attributes(from_variants))]
pub fn derive_from_variants(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let target_enum = &input.ident;

    // Extract the source enum names from the attribute
    let source_enums = extract_source_enums(&input.attrs)
        .expect("from_variants attribute with source enum names is required");

    // Only work with enums
    let data = match &input.data {
        Data::Enum(data) => data,
        _ => panic!("FromVariants can only be derived for enums"),
    };

    // Generate match arms only for variants marked with #[from_variants]
    let match_arms: Vec<_> = data.variants.iter().flat_map(|variant| {
        // Check if this variant has the #[from_variants] attribute
        if !has_from_variants_attr(&variant.attrs) {
            return Vec::new();
        }

        let variant_sources = extract_from_variants_sources(&variant.attrs, &variant.ident);
        let sources = if variant_sources.is_empty() {
            // Fallback to default behavior
            if source_enums.len() == 1 {
                vec![(source_enums[0].clone(), variant.ident.clone())]
            } else {
                panic!("When multiple source enums are specified, each variant must specify which enum with #[from_variants(Enum)] or #[from_variants(Enum::Variant)]")
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
                    let field_names: Vec<_> = fields.named.iter()
                        .map(|f| &f.ident)
                        .collect();
                    let field_conversions: Vec<_> = field_names.iter()
                        .map(|name| quote! { #name: #name.into() })
                        .collect();
                    quote! {
                        #source_enum_name::#source_variant_name { #(#field_names),* } => #target_enum::#target_variant_name { #(#field_conversions),* },
                    }
                }
            };
            (source_enum_name, source_variant_name, target_variant_name.clone(), arm)
        }).collect::<Vec<_>>()
    }).collect();

    // Group match arms by source enum
    let mut arms_by_enum: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();

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
            Err(syn::Error::new_spanned(path, "Expected Enum or Enum::Variant"))
        }
    }
}

fn extract_source_enums(attrs: &[Attribute]) -> Option<Vec<Path>> {
    for attr in attrs {
        if attr.path().is_ident("from_variants") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(args) = meta_list.parse_args::<ContainerFromVariantsArgs>() {
                    return Some(args.enums.into_iter().collect());
                }
            }
        }
    }
    None
}

fn has_from_variants_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("from_variants"))
}

fn extract_from_variants_sources(attrs: &[Attribute], target_variant: &Ident) -> Vec<(Path, Ident)> {
    for attr in attrs {
        if attr.path().is_ident("from_variants") {
            match &attr.meta {
                Meta::Path(_) => {
                    // #[from_variants] without arguments - return empty to use default fallback
                    return Vec::new();
                },
                Meta::List(meta_list) => {
                    if let Ok(args) = meta_list.parse_args::<VariantFromVariantsArgs>() {
                        return args.sources.into_iter().map(|source| {
                            match source {
                                VariantSource::EnumOnly(enum_name) => (enum_name, target_variant.clone()),
                                VariantSource::EnumVariant(enum_name, variant_name) => (enum_name, variant_name),
                            }
                        }).collect();
                    }
                },
                _ => continue,
            }
        }
    }
    Vec::new()
}
