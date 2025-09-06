use enum_convert::EnumFrom;

enum Source {
    Tuple(i32, i32),
    Struct { x: i32, y: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from]
    Tuple(
        #[enum_from(Source::Tuple.1)] i32,
        #[enum_from(Source::Tuple.0)] i32,
    ),
    #[enum_from]
    Struct {
        #[enum_from(Source::Struct.x)]
        a: i64,
        #[enum_from(Source::Struct.y)]
        b: i64,
    },
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
