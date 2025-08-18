use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    // Missing #[enum_into(Target::Bar)]
    Foo,
}

enum Target {
    Bar,
}

fn main() {}
