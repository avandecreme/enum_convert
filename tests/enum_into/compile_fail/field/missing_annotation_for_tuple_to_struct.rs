use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into(Target::Struct)]
    Tuple(
        // #[enum_into(Target::Struct.b)] missing
        i32,
        #[enum_into(Target::Struct.a)] i32,
    ),
}

enum Target {
    Struct { a: i64, b: i32 },
}

fn main() {}
