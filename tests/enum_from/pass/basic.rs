use enum_convert::EnumFrom;

enum Source {
    Unit,
    Tuple(i32, &'static str),
    Struct { x: i32, y: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from]
    Unit,
    #[enum_from]
    Tuple(i64, String),
    #[enum_from]
    Struct {
        x: i64,
        y: i64,
    },
    Extra,
}

fn main() {
    assert!(matches!(Target::from(Source::Unit), Target::Unit));
    assert!(matches!(
        Target::from(Source::Tuple(42, "hello")),
        Target::Tuple(42, ref s) if s == "hello",
    ));
    assert!(matches!(
        Target::from(Source::Struct { x: 1, y: 2}),
        Target::Struct { x, y } if x == 1 && y == 2,
    ));
}
