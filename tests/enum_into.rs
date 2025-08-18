#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/enum_into/compile_fail/**/*.rs");
}
