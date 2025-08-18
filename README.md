# enum_convert

A Rust procedural macro library for deriving automatic conversions between enums variants.

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
    Struct { x: i32, y: i32 }
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
    Extra  // This variant has no mapping
}

// Usage
let source = Source::Tuple(42, "hello".to_string());
let target: Target = source.into();
```

### EnumInto - Convert from annotated source enum to target enums

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
    Extra
}

// Usage
let source = Source::Variant(42);
let target: Target = source.into();
```

### Advanced Features

#### Multiple source/target enums

```rust
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
