use enum_convert::EnumFrom;

enum Source {
    Struct { aa: i32, bb: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::Struct)]
    Tuple(
        #[enum_from(Source::Struct.bb)] i32,
        #[enum_from(Source::Struct.aa)] i64,
    ),
}

fn main() {
    assert!(matches!(
        Target::from(Source::Struct { aa: 1, bb: 2 }),
        Target::Tuple(bb, aa) if aa == 1 && bb == 2,
    ));
}
