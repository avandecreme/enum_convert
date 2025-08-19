use enum_convert::EnumInto;

#[derive(EnumInto)]
#[enum_into(Target)]
enum Source {
    #[enum_into(Target::UnitRenamed)]
    Unit,
    #[enum_into(Target::UnitRenamed)]
    Extra,
    #[enum_into(Target::TupleRenamed)]
    Tuple(i32, &'static str),
    #[enum_into(Target::StructRenamed)]
    Struct { x: i32, y: i32 },
}

enum Target {
    UnitRenamed,
    TupleRenamed(i64, String),
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
