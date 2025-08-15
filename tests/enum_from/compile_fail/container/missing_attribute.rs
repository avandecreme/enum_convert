use enum_convert::EnumFrom;

enum Source {
    Unit,
}

#[derive(EnumFrom)]
// Missing #[enum_from(Source)] attribute
enum Target {
    #[enum_from]
    Unit,
}

fn main() {}
