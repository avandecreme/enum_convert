use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    Field(i32),
}

struct Target { // Should be an enum
    Field: i32,
}

fn main() {}
