use enum_convert::EnumFrom;

enum Source {
    Data { x: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::Data)]
    Stuff {
        #[enum_from(Source::Data.nonexistent)] // Invalid field name
        x: i64,
    },
}

fn main() {}
