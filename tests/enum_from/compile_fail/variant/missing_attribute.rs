use enum_convert::EnumFrom;

enum Source {
    Foo,
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    // Missing #[enum_from(Source::Foo)]
    Bar,
}

fn main() {}
