use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Meta, Path};

/// Derives `From<T>` for the annotated enum where T is a smaller enum with a subset of variants.
///
/// # Examples
///
/// ```
/// use from_variants::FromVariants;
///
/// enum Smaller {
///     Unit,
///     Tuple(i32, &'static str),
///     Struct { x: i32, y: i32 }
/// }
///
/// #[derive(FromVariants)]
/// #[from_variants(Smaller)]
/// enum Bigger {
///     #[from_variant]
///     Unit,
///     #[from_variant]
///     Tuple(i64, String),
///     #[from_variant]
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
/// let smaller = Smaller::Struct { x: 1, y: 2 };
/// let bigger: Bigger = smaller.into();
/// assert!(matches!(bigger, Bigger::Struct { x, y } if x == 1.0 && y == 2.0));
/// ```
#[proc_macro_derive(FromVariants, attributes(from_variants, from_variant))]
pub fn derive_from_variants(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let target_enum = &input.ident;

    // Extract the source enum name from the attribute
    let source_enum = extract_source_enum(&input.attrs)
        .expect("from_variants attribute with source enum name is required");

    // Only work with enums
    let data = match &input.data {
        Data::Enum(data) => data,
        _ => panic!("FromVariants can only be derived for enums"),
    };

    // Generate match arms only for variants marked with #[from_variant]
    let match_arms = data.variants.iter().filter_map(|variant| {
        // Check if this variant has the #[from_variant] attribute
        let has_from_variant = variant.attrs.iter().any(|attr| {
            attr.path().is_ident("from_variant")
        });

        if !has_from_variant {
            return None;
        }

        let variant_name = &variant.ident;
        let arm = match &variant.fields {
            Fields::Unit => quote! {
                #source_enum::#variant_name => #target_enum::#variant_name,
            },
            Fields::Unnamed(fields) => {
                let field_names: Vec<_> = (0..fields.unnamed.len())
                    .map(|i| quote::format_ident!("field_{}", i))
                    .collect();
                let field_conversions: Vec<_> = field_names.iter()
                    .map(|name| quote! { #name.into() })
                    .collect();
                quote! {
                    #source_enum::#variant_name(#(#field_names),*) => #target_enum::#variant_name(#(#field_conversions),*),
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
                    #source_enum::#variant_name { #(#field_names),* } => #target_enum::#variant_name { #(#field_conversions),* },
                }
            }
        };
        Some(arm)
    });

    let expanded = quote! {
        impl From<#source_enum> for #target_enum {
            fn from(value: #source_enum) -> Self {
                match value {
                    #(#match_arms)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn extract_source_enum(attrs: &[Attribute]) -> Option<Path> {
    for attr in attrs {
        if attr.path().is_ident("from_variants") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(path) = meta_list.parse_args::<Path>() {
                    return Some(path);
                }
            }
        }
    }
    None
}
