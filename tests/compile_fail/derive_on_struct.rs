use enum_convert::EnumFrom;

#[derive(EnumFrom)]
#[enum_from(SomeEnum)]
struct NotAnEnum {  // Should only work on enums
    field: i32,
}

enum SomeEnum {
    Unit,
}

fn main() {}