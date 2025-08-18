use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into(Target::Data)]
    Stuff {
        // Missing #[enum_into(Target::a)]
        x: i32,
    },
}

enum Target {
    Data { a: i64 },
}

fn main() {}
