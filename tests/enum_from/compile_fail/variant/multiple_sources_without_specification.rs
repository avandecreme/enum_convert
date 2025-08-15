use enum_convert::EnumFrom;

enum First {
    Unit,
}

enum Second {
    Unit,
}

#[derive(EnumFrom)]
#[enum_from(First, Second)]
enum Target {
    #[enum_from] // Ambiguous - should specify which enum when multiple sources
    Unit,
}

fn main() {}
