use enum_convert::EnumFrom;

// enum Source {
//     Unit,
// }

#[derive(EnumFrom)]
#[enum_from(Source)] // There is no Source enum
enum Target {
    Unit,
}

fn main() {}
