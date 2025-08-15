use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into = "Target::Bar"] // Should be #[enum_into(Target::Bar)]
    Foo,
}

enum Target {
    Bar,
}

fn main() {}
