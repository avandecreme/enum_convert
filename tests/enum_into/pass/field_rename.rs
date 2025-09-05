use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    Tuple(
        #[enum_into(Target::Tuple.1)] i32,
        #[enum_into(Target::Tuple.0)] i32,
    ),
    Struct {
        #[enum_into(Target::Struct.a)]
        x: i32,
        #[enum_into(Target::Struct.b)]
        y: i32,
    },
}

enum Target {
    Tuple(i32, i32),
    Struct { a: i64, b: i64 },
    Extra,
}

fn main() {
    assert!(matches!(
        Target::from(Source::Tuple(1, 2)),
        Target::Tuple(a, b) if a == 2 && b == 1,
    ));
    assert!(matches!(
        Target::from(Source::Struct { x: 1, y: 2}),
        Target::Struct { a, b } if a == 1 && b == 2,
    ));
}
