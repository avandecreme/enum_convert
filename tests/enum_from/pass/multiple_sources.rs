use enum_convert::EnumFrom;

enum FirstSource {
    Unit,
    Tuple(i32, &'static str),
    Struct { x: f64, y: f64, s: &'static str },
}

enum SecondSource {
    Unit,
    Struct { x: i32, y: i32, s: &'static str },
}

#[derive(EnumFrom)]
#[enum_from(FirstSource, SecondSource)]
enum Target {
    #[enum_from(FirstSource)]
    #[enum_from(SecondSource)]
    Unit,

    #[enum_from(FirstSource)]
    Tuple(i64, String),

    #[enum_from(FirstSource, SecondSource)]
    Struct {
        x: f64,
        y: f64,
        s: &'static str,
    },

    Extra,
}

fn main() {
    assert!(matches!(Target::from(FirstSource::Unit), Target::Unit));
    assert!(matches!(Target::from(SecondSource::Unit), Target::Unit));

    assert!(matches!(
        Target::from(FirstSource::Tuple(42, "hello")),
        Target::Tuple(42, ref s) if s == "hello",
    ));

    assert!(matches!(
        Target::from(FirstSource::Struct { x: 1.0, y: 2.0, s: "hello" }),
        Target::Struct { x, y, s } if x == 1.0 && y == 2.0 && s == "hello",
    ));

    assert!(matches!(
        Target::from(SecondSource::Struct { x: 1, y: 2, s: "hello" }),
        Target::Struct { x, y, s } if x == 1.0 && y == 2.0 && s == "hello",
    ));
}
