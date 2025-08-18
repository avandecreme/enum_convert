use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    Struct {
        #[enum_into()] // Should be #[enum_into(Target::a)]
        x: i32
    }
}

enum Target {
    Struct {
        a: i32
    },
}

fn main() {}
