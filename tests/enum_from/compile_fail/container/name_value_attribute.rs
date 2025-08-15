use enum_convert::EnumFrom;

enum Source {
    Unit,
}

#[derive(EnumFrom)]
#[enum_from = "Source"] // Should be #[enum_from(Source)]
enum Target {
    Unit,
}

fn main() {}
