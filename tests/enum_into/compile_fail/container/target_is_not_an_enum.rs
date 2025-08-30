use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    Field(i32),
}

// Should be an enum
struct Target {
    Field: i32,
}

fn main() {}
