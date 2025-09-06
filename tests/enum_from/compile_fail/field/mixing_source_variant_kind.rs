use enum_convert::EnumFrom;

enum Source {
    Struct { a: i32, b: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::Tuple)]
    Struct {
        #[enum_from(Source::Tuple.0)] // .0 Does not make sense for a Struct source
        a: i32,
        #[enum_from(Source::Tuple.b)]
        b: i32,
    },
}

fn main() {}
