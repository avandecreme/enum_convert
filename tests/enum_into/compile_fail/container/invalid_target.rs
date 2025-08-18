use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)] // There is no Target enum
enum Source {
    Unit,
}

// enum Target {
//     Unit,
// }

fn main() {}
