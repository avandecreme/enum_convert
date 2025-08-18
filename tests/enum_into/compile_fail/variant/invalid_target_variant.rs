use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into(Target::NonExistent)] // Invalid variant name
    Unit,
}

enum Target {
    Unit,
}

fn main() {}
