use enum_convert::EnumFrom;

enum Source {
    Stuff { x: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::Stuff)]
    Data {
        // Missing #[enum_from(Source::x)]
        a: i64,
    },
}

fn main() {}
