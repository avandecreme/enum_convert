use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into(Target, NonExistent)] // Invalid target enum
    Unit,
}

enum Target {
    Unit,
}

fn main() {}
