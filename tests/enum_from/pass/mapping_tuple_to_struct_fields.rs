use enum_convert::EnumFrom;

enum Source {
    Tuple(i32, i32),
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::Tuple)]
    Struct {
        #[enum_from(Source::Tuple.1)]
        a: i64,
        #[enum_from(Source::Tuple.0)]
        b: i32,
    },
}

fn main() {
    assert!(matches!(
        Target::from(Source::Tuple(1, 2)),
        Target::Struct { a, b } if a == 2 && b == 1,
    ));
}
