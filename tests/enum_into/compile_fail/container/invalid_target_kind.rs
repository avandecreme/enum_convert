use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target::Unit)] // Should be #[enum_into(Target)]
enum Source {
    Unit,
}

enum Target {
    Unit,
}

fn main() {}
