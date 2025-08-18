use enum_convert::EnumFrom;

enum Source {
    Unit,
}

#[derive(EnumFrom)] // Should only work on enums
#[enum_from(Source)]
struct Target {
    field: i32,
}

fn main() {}
