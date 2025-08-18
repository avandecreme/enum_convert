use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    Struct {
        // Should be #[enum_into(Target::Struct.a)]
        #[enum_into(Target::Struct.a, Target::Struct.b)]
        x: i32,
    },
}

enum Target {
    Struct { a: i32 },
}

fn main() {}
