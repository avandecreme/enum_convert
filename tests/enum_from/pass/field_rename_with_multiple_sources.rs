use enum_convert::EnumFrom;

enum Source {
    Struct { x: i32, y: i32 },
    OtherStruct { m: i32, n: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::Struct, Source::OtherStruct)]
    Struct {
        #[enum_from(Source::Struct.x, Source::OtherStruct.m)]
        a: i64,
        #[enum_from(Source::Struct.y, Source::OtherStruct.n)]
        b: i64,
    },
    Extra,
}

fn main() {
    assert!(matches!(
        Target::from(Source::Struct { x: 1, y: 2}),
        Target::Struct { a, b } if a == 1 && b == 2,
    ));
    assert!(matches!(
        Target::from(Source::OtherStruct { m: 1, n: 2}),
        Target::Struct { a, b } if a == 1 && b == 2,
    ));
}
