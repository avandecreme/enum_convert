use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Source)]
enum Source {
    #[enum_into(Target::Unit::Extra)] // Invalid syntax - too many segments
    Unit,
}

enum Target {
    Unit,
}

fn main() {}
