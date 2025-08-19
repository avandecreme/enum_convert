use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    Struct {
        #[enum_into(Target::Struct.a)]
        x: i32,
        #[enum_into(Target::Struct.b)]
        y: i32,
    },
}

enum Target {
    Struct { a: i64, b: i64 },
    Extra,
}

fn main() {
    assert!(matches!(
        Target::from(Source::Struct { x: 1, y: 2}),
        Target::Struct { a, b } if a == 1 && b == 2,
    ));
}
