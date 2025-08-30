use enum_convert::EnumInto;

#[derive(EnumInto)]
// Missing #[enum_into(Target)] attribute
enum Source {
    Unit,
}

enum Target {
    Unit,
}

fn main() {}
