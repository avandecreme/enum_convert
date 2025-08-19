use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into(Target::Stuff)]
    Data {
        #[enum_into(Target::Stuff.nonexistent)] // Invalid field name
        x: i32,
    },
}

enum Target {
    Stuff { x: i64 },
}

fn main() {}
