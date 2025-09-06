use enum_convert::EnumFrom;

enum Source {
    Tuple(i32, i32),
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::Tuple)]
    Struct {
        #[enum_from(Source::Tuple.0)]
        a: i32,
        #[enum_from(Source::Tuple.1)]
        b: i32,
    },
}

fn main() {}
