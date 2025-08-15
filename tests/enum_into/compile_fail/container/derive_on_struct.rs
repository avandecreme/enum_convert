use enum_convert::EnumInto;

#[derive(EnumInto)] // Should only work on enums
#[enum_into(Target)]
struct Source {
    field: i32,
}

enum Target {
    Unit,
}

fn main() {}
