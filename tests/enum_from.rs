#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/enum_from/compile_fail/**/*.rs");
}
