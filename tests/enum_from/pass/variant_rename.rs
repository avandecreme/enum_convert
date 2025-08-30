use enum_convert::EnumFrom;

enum Source {
    Unit,
    Extra,
    Tuple(i32, &'static str),
    Struct { x: i32, y: i32 },
}

#[derive(EnumFrom)]
#[enum_from(Source)]
enum Target {
    #[enum_from(Source::Unit, Source::Extra)]
    UnitRenamed,
    #[enum_from(Source::Tuple)]
    TupleRenamed(i64, String),
    #[enum_from(Source::Struct)]
    StructRenamed { x: i64, y: i64 },
}

fn main() {
    assert!(matches!(Target::from(Source::Unit), Target::UnitRenamed));
    assert!(matches!(Target::from(Source::Extra), Target::UnitRenamed));
    assert!(matches!(
        Target::from(Source::Tuple(42, "hello")),
        Target::TupleRenamed(42, ref s) if s == "hello",
    ));
    assert!(matches!(
        Target::from(Source::Struct { x: 1, y: 2}),
        Target::StructRenamed { x, y } if x == 1 && y == 2,
    ));
}
