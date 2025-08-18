use enum_convert::EnumFrom;

enum Source1 {
    Unit,
}

enum Source2 {
    Unit,
}

#[derive(EnumFrom)]
#[enum_from(Source1; Source2)] // Semi colon instead of comma
enum Target {
    Unit,
}

fn main() {}
