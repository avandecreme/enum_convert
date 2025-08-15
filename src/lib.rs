mod from_variants;
mod into_variants;

use proc_macro::TokenStream;

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
///     DifferentName {
///         alpha: f64,
///         y: f64,
///         s: &'static str,
///     },
/// }
///
/// enum Second {
///     Empty,
///     Struct { a: i32, b: i32, s: &'static str },
/// }
///
/// #[derive(FromVariants)]
/// #[from_variants(First, Second)]
/// enum Bigger {
///     #[from_variants(First, Second::Empty)]
///     Unit,
///     #[from_variants(First)]
///     Tuple(i64, String),
///     #[from_variants(First::DifferentName, Second)]
///     Struct {
///         #[from_variants(First::alpha, Second::a)]
///         x: f64,
///         #[from_variants(Second::b)]
///         y: f64,
///         s: &'static str,
///     },
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
/// let first = First::DifferentName { alpha: 1.0, y: 2.0, s: "hello" };
/// let bigger: Bigger = first.into();
/// assert!(matches!(bigger, Bigger::Struct { x, y, s } if x == 1.0 && y == 2.0 && s == "hello"));
///
/// // Struct can also come from Second
/// let second = Second::Struct { a: 1, b: 2, s: "hello" };
/// let bigger: Bigger = second.into();
/// assert!(matches!(bigger, Bigger::Struct { x, y, s } if x == 1.0 && y == 2.0 && s == "hello"));
/// ```
#[proc_macro_derive(FromVariants, attributes(from_variants))]
pub fn derive_from_variants(input: TokenStream) -> TokenStream {
    from_variants::derive_from_variants_impl(input)
}

/// Derives `From<ThisEnum>` for other enums where this enum is a smaller subset.
///
/// # Examples
///
/// ## Single target enum
/// ```
/// use from_variants::IntoVariants;
///
/// #[derive(IntoVariants)]
/// #[into_variants(Bigger)]
/// enum Smaller {
///     Unit,  // Uses same name in target
///     Tuple(i32, &'static str),  // Uses same name in target
///     #[into_variants(Bigger::Struct)]  // Maps to different variant name
///     DifferentName { x: i32, y: i32 }
/// }
///
/// enum Bigger {
///     Unit,
///     Tuple(i64, String),
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
/// ## Multiple target enums with field mapping
/// ```
/// use from_variants::IntoVariants;
///
/// #[derive(IntoVariants)]
/// #[into_variants(First, Second)]
/// enum Source {
///     Unit,  // Goes to both First::Unit and Second::Unit
///     #[into_variants(First::Data, Second::Info)]  // Maps to different variants
///     Record {
///         #[into_variants(First::name, Second::title)]  // Maps fields differently
///         label: String,
///         value: i32
///     }
/// }
///
/// enum First {
///     Unit,
///     Data { name: String, value: i64 }
/// }
///
/// enum Second {
///     Unit,
///     Info { title: String, value: i64 }
/// }
///
/// let source = Source::Unit;
/// let first: First = source.into();
/// assert!(matches!(first, First::Unit));
///
/// let source = Source::Unit;
/// let second: Second = source.into();
/// assert!(matches!(second, Second::Unit));
///
/// let source = Source::Record { label: "test".to_string(), value: 42 };
/// let first: First = source.into();
/// assert!(matches!(first, First::Data { name, value } if name == "test" && value == 42));
///
/// let source = Source::Record { label: "test".to_string(), value: 42 };
/// let second: Second = source.into();
/// assert!(matches!(second, Second::Info { title, value } if title == "test" && value == 42));
/// ```
#[proc_macro_derive(IntoVariants, attributes(into_variants))]
pub fn derive_into_variants(input: TokenStream) -> TokenStream {
    into_variants::derive_into_variants_impl(input)
}
