mod from_variants;
mod into_variants;

use proc_macro::TokenStream;

/// Derives `From<Source>` for the annotated Target enum where the source enums have a subset of variants.
///
/// # Examples
///
/// ## Single source enum
/// ```
/// use from_variants::FromVariants;
///
/// enum Source {
///     Unit,
///     OtherUnit,
///     Tuple(i32, &'static str),
///     DifferentName { x: i32, y: i32 }
/// }
///
/// #[derive(FromVariants)]
/// #[from_variants(Source)]
/// enum Target {
///     #[from_variants(Source::Unit, Source::OtherUnit)]
///     Unit,
///     #[from_variants] // If there is only one mapping and it has the same name, there is no need to specify the variant
///     Tuple(i64, String),
///     #[from_variants(Source::DifferentName)]
///     Struct { x: f64, y: f64 },
///     Extra // This variant cannot be built from Source
/// }
///
/// let source = Source::Unit;
/// let target: Target = source.into();
/// assert!(matches!(target, Target::Unit));
///
/// // Target::Unit can also come from Source::OtherUnit
/// let source = Source::OtherUnit;
/// let target: Target = source.into();
/// assert!(matches!(target, Target::Unit));
///
/// let source = Source::Tuple(42, "hello");
/// let target: Target = source.into();
/// assert!(matches!(target, Target::Tuple(42, ref s) if s == "hello"));
///
/// let source = Source::DifferentName { x: 1, y: 2 };
/// let target: Target = source.into();
/// assert!(matches!(target, Target::Struct { x, y } if x == 1.0 && y == 2.0));
/// ```
///
/// ## Multiple source enums
/// ```
/// use from_variants::FromVariants;
///
/// enum FirstSource {
///     Unit,
///     Tuple(i32, &'static str),
///     DifferentName {
///         alpha: f64,
///         y: f64,
///         s: &'static str,
///     },
/// }
///
/// enum SecondSource {
///     Empty,
///     Struct { a: i32, b: i32, s: &'static str },
/// }
///
/// #[derive(FromVariants)]
/// #[from_variants(FirstSource, SecondSource)]
/// enum Target {
///     #[from_variants(FirstSource, SecondSource::Empty)]
///     Unit,
///     #[from_variants(FirstSource)]
///     Tuple(i64, String),
///     #[from_variants(FirstSource::DifferentName, SecondSource)]
///     Struct {
///         #[from_variants(FirstSource::alpha, SecondSource::a)]
///         x: f64,
///         #[from_variants(SecondSource::b)]
///         y: f64,
///         s: &'static str,
///     },
///     Extra
/// }
///
/// let first_source = FirstSource::Unit;
/// let target: Target = first_source.into();
/// assert!(matches!(target, Target::Unit));
///
/// // Target::Unit can also come from SecondSource::Empty
/// let second_source = SecondSource::Empty;
/// let target: Target = second_source.into();
/// assert!(matches!(target, Target::Unit));
///
/// let first_source = FirstSource::Tuple(42, "hello");
/// let target: Target = first_source.into();
/// assert!(matches!(target, Target::Tuple(42, ref s) if s == "hello"));
///
/// let first_source = FirstSource::DifferentName { alpha: 1.0, y: 2.0, s: "hello" };
/// let target: Target = first_source.into();
/// assert!(matches!(target, Target::Struct { x, y, s } if x == 1.0 && y == 2.0 && s == "hello"));
///
/// // Target::Struct can also come from SecondSource::Struct
/// let second_source = SecondSource::Struct { a: 1, b: 2, s: "hello" };
/// let target: Target = second_source.into();
/// assert!(matches!(target, Target::Struct { x, y, s } if x == 1.0 && y == 2.0 && s == "hello"));
/// ```
#[proc_macro_derive(FromVariants, attributes(from_variants))]
pub fn derive_from_variants(input: TokenStream) -> TokenStream {
    from_variants::derive_from_variants_impl(input)
}

/// Derives `From<Source>` for the annotated enum where the target enums have a subset of variants.
///
/// # Examples
///
/// ## Single target enum
/// ```
/// use from_variants::IntoVariants;
///
/// #[derive(IntoVariants)]
/// #[into_variants(Target)]
/// enum Source {
///     Unit,  // Uses same name in target
///     #[into_variants(Target::Unit)]
///     OtherUnit,
///     Tuple(i32, &'static str),  // Uses same name in target
///     #[into_variants(Target::Struct)]  // Maps to different variant name
///     DifferentName { x: i32, y: i32 }
/// }
///
/// enum Target {
///     Unit,
///     Tuple(i64, String),
///     Struct { x: f64, y: f64 },
///     Extra
/// }
///
/// let source = Source::Unit;
/// let target: Target = source.into();
/// assert!(matches!(target, Target::Unit));
///
/// // Target::Unit can also come from Source::OtherUnit
/// let source = Source::OtherUnit;
/// let target: Target = source.into();
/// assert!(matches!(target, Target::Unit));
///
/// let source = Source::Tuple(42, "hello");
/// let target: Target = source.into();
/// assert!(matches!(target, Target::Tuple(42, ref s) if s == "hello"));
///
/// let source = Source::DifferentName { x: 1, y: 2 };
/// let target: Target = source.into();
/// assert!(matches!(target, Target::Struct { x, y } if x == 1.0 && y == 2.0));
/// ```
///
/// ## Multiple target enums with field mapping
/// ```
/// use from_variants::IntoVariants;
///
/// #[derive(IntoVariants)]
/// #[into_variants(FirstTarget, SecondTarget)]
/// enum Source {
///     Unit,  // Goes to both FirstTarget::Unit and SecondTarget::Unit
///     #[into_variants(FirstTarget::Data, SecondTarget::Info)]  // Maps to different variants
///     Record {
///         #[into_variants(FirstTarget::name, SecondTarget::title)]  // Maps fields differently
///         label: String,
///         value: i32
///     }
/// }
///
/// enum FirstTarget {
///     Unit,
///     Data { name: String, value: i64 }
/// }
///
/// enum SecondTarget {
///     Unit,
///     Info { title: String, value: i64 }
/// }
///
/// let source = Source::Unit;
/// let first_target: FirstTarget = source.into();
/// assert!(matches!(first_target, FirstTarget::Unit));
///
/// // Source::Unit can also go to SecondTarget::Unit
/// let source = Source::Unit;
/// let second_target: SecondTarget = source.into();
/// assert!(matches!(second_target, SecondTarget::Unit));
///
/// let source = Source::Record { label: "test".to_string(), value: 42 };
/// let first_target: FirstTarget = source.into();
/// assert!(matches!(first_target, FirstTarget::Data { name, value } if name == "test" && value == 42));
///
/// // Source::Record can also go to SecondTarget::Info with different field mapping
/// let source = Source::Record { label: "test".to_string(), value: 42 };
/// let second_target: SecondTarget = source.into();
/// assert!(matches!(second_target, SecondTarget::Info { title, value } if title == "test" && value == 42));
/// ```
#[proc_macro_derive(IntoVariants, attributes(into_variants))]
pub fn derive_into_variants(input: TokenStream) -> TokenStream {
    into_variants::derive_into_variants_impl(input)
}
