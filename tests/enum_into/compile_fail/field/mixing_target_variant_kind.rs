use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into(Target::Struct)]
    Struct {
        #[enum_into(Target::Struct.0)] // .0 Does not make sense for a Struct target
        a: i32,
        #[enum_into(Target::Struct.b)]
        b: i32,
    },
}

enum Target {
    Struct { a: i32, b: i32 },
}

fn main() {}
