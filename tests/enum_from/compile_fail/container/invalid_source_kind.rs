use enum_convert::EnumFrom;

enum Source {
    Unit,
}

#[derive(EnumFrom)]
#[enum_from(Source::Unit)] // Should be #[enum_from(Source)]
enum Target {
    #[enum_from]
    Unit,
}

fn main() {}
