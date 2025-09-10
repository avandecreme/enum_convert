use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into(Target::Struct)]
    Tuple(
        #[enum_into(Target::Struct.bb)] i32,
        #[enum_into(Target::Struct.aa)] i32,
    ),
}

enum Target {
    Struct { aa: i32, bb: i32 },
}

fn main() {
    assert!(matches!(
        Target::from(Source::Tuple(1, 2)),
        Target::Struct { aa, bb } if aa == 2 && bb == 1,
    ));
}
