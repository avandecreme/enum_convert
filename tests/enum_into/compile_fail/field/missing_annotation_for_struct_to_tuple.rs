use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into(Target::Tuple)]
    Struct {
        // #[enum_into(Target::Tuple.1)] missing
        aa: i32,
        #[enum_into(Target::Tuple.0)]
        bb: i32,
    },
}

enum Target {
    Tuple(i64),
}

fn main() {}
