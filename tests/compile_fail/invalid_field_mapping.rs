use enum_convert::EnumFrom;

enum Source {
    Data { x: i32 }
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from]
    Data {
        #[enum_from(Source::nonexistent)]  // Invalid field name
        x: i64,
    }
}

fn main() {}