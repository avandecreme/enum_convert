use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into(Target::Tuple)]
    Struct {
        #[enum_into(Target::Tuple.0)]
        a: i32,
        #[enum_into(Target::Tuple.1)]
        b: i32,
    },
}

enum Target {
    Tuple(i32, i32),
}

fn main() {
    assert!(matches!(
        Target::from(Source::Struct { a: 1, b: 2 }),
        Target::Tuple(a, b) if a == 1 && b == 2,
    ));
}
