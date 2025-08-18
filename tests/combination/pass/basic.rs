use enum_convert::{EnumFrom, EnumInto};

enum Source {
    Unit,
    Tuple(u8, &'static str),
    Struct { x: i16, y: i16 },
}

#[derive(EnumFrom, EnumInto)]
#[enum_from(Source)]
#[enum_into(Target)]
enum Transit {
    #[enum_from]
    Unit,
    #[enum_from]
    Tuple(i32, &'static str),
    #[enum_from]
    Struct { x: i32, y: i32 },
}

enum Target {
    Unit,
    Tuple(i64, String),
    Struct { x: i64, y: i64 },
}

fn main() {
    assert!(matches!(
        Target::from(Transit::from(Source::Unit)),
        Target::Unit
    ));
    assert!(matches!(
        Target::from(Transit::from(Source::Tuple(42, "hello"))),
        Target::Tuple(42, ref s) if s == "hello",
    ));
    assert!(matches!(
        Target::from(Transit::from(Source::Struct { x: 1, y: 2})),
        Target::Struct { x, y } if x == 1 && y == 2,
    ));
}
