use enum_convert::EnumFrom;

enum Source {
    Struct { aa: i32, bb: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::Struct)]
    Tuple(
        #[enum_from(Source::Struct.aa)] i32,
        i64, // Annotation #[enum_from(Source::Struct.aa)] is missing
    ),
}

fn main() {}
