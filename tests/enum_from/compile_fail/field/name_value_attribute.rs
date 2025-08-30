use enum_convert::EnumFrom;

enum Source {
    Struct { x: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from]
    Struct {
        #[enum_from = "Source::Struct.x"] // Should be #[enum_from(Source::Struct.x)]
        a: i32,
    },
}

fn main() {}
