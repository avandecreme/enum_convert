use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target1; Target2)] // semi colon instead of comma
enum Source {
    Unit,
}

enum Target1 {
    Unit,
}

enum Target2 {
    Unit,
}

fn main() {}
