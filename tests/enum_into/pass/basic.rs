use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    Unit,
    Tuple(i32, &'static str),
    Struct { x: i32, y: i32 },
}

enum Target {
    Unit,
    Tuple(i64, String),
    Struct { x: i64, y: i64 },
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
