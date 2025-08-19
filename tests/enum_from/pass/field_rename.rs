use enum_convert::EnumFrom;

enum Source {
    Struct { x: i32, y: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
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
        Target::from(Source::Struct { x: 1, y: 2}),
        Target::Struct { a, b } if a == 1 && b == 2,
    ));
}
