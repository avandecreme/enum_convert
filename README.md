# enum_convert

Crate to derive [From](https://doc.rust-lang.org/core/convert/trait.From.html) implementations between enums.

## Features

- **EnumFrom**: Derive `From<Source> for AnnotatedEnum`
- **EnumInto**: Derive `From<AnnotatedEnum> for Target`
- Support for multiple source/target enums
- Flexible variant name mapping (one-to-many, many-to-one)
- Field-level mapping for named struct variants
- Automatic type conversion for fields via `.into()`

## Usage

### EnumFrom - Convert from source enums to annotated target enum

```rust
use enum_convert::EnumFrom;

enum Source {
    Unit,
    Tuple(i32, String),
    Struct { x: i32, y: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from]  // Maps from Source::Unit
    Unit,
    #[enum_from]  // Maps from Source::Tuple with type conversion
    Tuple(i64, String),
    #[enum_from]  // Maps from Source::Struct with type conversion
    Struct { x: f64, y: f64 },
    Extra,  // This variant has no mapping
}

// Usage
let source = Source::Tuple(42, "hello".to_string());
let target: Target = source.into();
```

### EnumInto - Convert from annotated source enum to target enums

ℹ️ While the macro is named `EnumInto`, it still implements `From<AnnotatedEnum> for TargetEnum` and thus has an indirect implementation of `Into` as [recommended by the docs](https://doc.rust-lang.org/core/convert/trait.Into.html).
This is similar to `derive_more`'s [Into](https://docs.rs/derive_more/latest/derive_more/derive.Into.html).

```rust
use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    Unit,  // Maps to Target::Unit
    #[enum_into(Target::Different)]  // Maps to Target::Different
    Variant(i32),
}

enum Target {
    Unit,
    Different(i64),
    Extra,
}

// Usage
let source = Source::Variant(42);
let target: Target = source.into();
```

### Advanced Features

#### Multiple source/target enums

```rust
use enum_convert::EnumFrom;

enum FirstSource {
    Unit,
    Data(String),
}

enum SecondSource {
    Empty,
}

#[derive(EnumFrom)]
#[enum_from(FirstSource, SecondSource)]
enum Target {
    #[enum_from(FirstSource, SecondSource::Empty)]
    Unit,
    #[enum_from(FirstSource)]
    Data(String),
}
```

#### Field mapping

```rust
use enum_convert::EnumFrom;

enum Source {
    Record {
        name: String,
        value: i32,
    }
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from]
    Record {
        #[enum_from(Source::Record.name)]  // Maps Source::Record.name to Target::Record.title
        title: String,
        value: i32,
    }
}
```

## Related and similar crates

### derive_more

This crate has some similarities with [derive_more](https://docs.rs/derive_more/latest/derive_more/index.html)'s `From` and `Into` derive macros.

The difference is that with `derive_more` the conversion is field → variant (`From`) and variant → field (`Into`); with this crate it is variant ↔ variant.

```rust
#[derive(derive_more::From, enum_convert::EnumFrom)]
#[enum_from(OtherEnum)]
enum MyEnum {
    #[enum_from]
    Variant1(i32),
}

enum OtherEnum {
    Variant1(i32),
}
```
```rust compile_fail
// `derive_more::From` get expanded to
impl From<i32> for MyEnum {
    fn from(value: i32) -> MyEnum {
        MyEnum::Variant1(value)
    }
}

// `enum_convert::EnumFrom` get expanded to
impl From<OtherEnum> for MyEnum {
    fn from(value: OtherEnum) -> MyEnum {
        match value {
            OtherEnum::Variant1(i) => MyEnum::Variant1(i),
        }
    }
}
```

### enum_to_enum

This crate is very similar to [enum_to_enum](https://docs.rs/enum_to_enum/latest/enum_to_enum/).

At the time of writing (`enum_to_enum` in version 0.1.0) the differences are:
- `enum_convert` does not support [many-to-one conversion with try_into logic ](https://docs.rs/enum_to_enum/latest/enum_to_enum/derive.FromEnum.html#many-to-one-conversion).
- `enum_convert` does not support [effectful conversion](https://docs.rs/enum_to_enum/latest/enum_to_enum/derive.FromEnum.html#effectful-conversion).
- `enum_to_enum` does not support `EnumInto`.
- `enum_to_enum` does not support having variants in the target for which there is no mapping from source.
- `enum_to_enum` does not support fields mapping.

For the common features, here is a comparison of how they are expressed in both crates:

```rust
enum SourceA {
    Unit,
    Tuple(i32, String),
    Struct { x: i32, y: i32 },
}

enum SourceB {
    Unit,
    NoField,
    Tuple(i32, String),
    Struct { x: i32, y: i32 },
}

#[derive(enum_to_enum::FromEnum)]
#[from_enum(SourceA, SourceB)]
enum TargetEnumToEnum {
    #[from_case(SourceB = NoField, Unit)]
    Unit,

    Tuple(i64, String),

    Struct { x: f64, y: f64 },
}

#[derive(enum_convert::EnumFrom)]
#[enum_from(SourceA, SourceB)]
enum TargetEnumConvert {
    #[enum_from(SourceA, SourceB::Unit, SourceB::NoField)]
    Unit,

    #[enum_from(SourceA, SourceB)]
    Tuple(i64, String),

    #[enum_from(SourceA, SourceB)]
    Struct { x: f64, y: f64 },
}
```

### subenum

The [subenum](https://docs.rs/subenum/latest/subenum/) crate allows to implement easily subset of enums with conversion between parent and child.

However, there are cases where it is not desirable or possible to use `subenum`.
For example:
- You don't want to declare the child enum in the same module or crate as the parent enum.
- There already is a child enum coming from another crate and you want to convert from that child enum to your own parent enum.
