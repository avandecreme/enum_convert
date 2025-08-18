use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(FirstTarget, SecondTarget)]
enum Source {
    Unit,
    Tuple(i32),
    Struct { x: i8, y: i8, s: &'static str },
}

enum FirstTarget {
    Unit,
    Tuple(i32),
    Struct { x: f64, y: f64, s: &'static str },
}

enum SecondTarget {
    Unit,
    Tuple(i32),
    Struct { x: f32, y: f32, s: &'static str },
}

fn main() {
    assert!(matches!(FirstTarget::from(Source::Unit), FirstTarget::Unit));
    assert!(matches!(SecondTarget::from(Source::Unit), SecondTarget::Unit));

    assert!(matches!(FirstTarget::from(Source::Tuple(1)), FirstTarget::Tuple(i) if i == 1));
    assert!(matches!(SecondTarget::from(Source::Tuple(1)), SecondTarget::Tuple(i) if i == 1));

    assert!(matches!(
        FirstTarget::from(Source::Struct { x: 1, y: 2, s: "hello" }),
        FirstTarget::Struct { x, y, s } if x == 1.0 && y == 2.0 && s == "hello",
    ));

    assert!(matches!(
        SecondTarget::from(Source::Struct { x: 1, y: 2, s: "hello" }),
        SecondTarget::Struct { x, y, s } if x == 1.0 && y == 2.0 && s == "hello",
    ));
}
