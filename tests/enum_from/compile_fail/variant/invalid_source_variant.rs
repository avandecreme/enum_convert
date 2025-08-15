use enum_convert::EnumFrom;

enum Source {
    Unit,
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::NonExistent)] // Invalid variant name
    Unit,
}

fn main() {}
