use enum_convert::EnumFrom;

struct Source {
    Field: i32,
}

#[derive(EnumFrom)]
#[enum_from(Source)] // Source should be an enum
enum Target {
    #[enum_from]
    Field,
}

fn main() {}
