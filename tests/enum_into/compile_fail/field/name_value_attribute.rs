use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    Data {
        #[enum_into = "Target::a"] // Should be #[enum_into(Target::Data.a)]
        x: i32,
    },
}

enum Target {
    Data { a: i64 },
}

fn main() {}
