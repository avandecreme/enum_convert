use enum_convert::{EnumFrom, EnumInto};

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    Unit,
    Tuple(i32, &'static str),
    Struct { x: i32, y: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from]
    Unit,
    #[enum_from]
    Tuple(i64, String),
    #[enum_from]
    Struct { x: i64, y: i64 },
}

fn main() {}
