use enum_convert::EnumFrom;

enum Source {
    Unit,
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::Unit::Extra)] // Invalid syntax - too many segments
    Unit,
}

fn main() {}
