#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/enum_from/compile_fail/**/*.rs");
}

#[test]
fn pass_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/enum_from/pass/**/*.rs");
}
