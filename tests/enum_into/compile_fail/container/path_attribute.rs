use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into]
enum Source {
    Unit,
}

enum Target {
    Unit,
}

fn main() {}
