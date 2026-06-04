//! Compile-fail tests proving that the typestate pattern prevents illegal
//! transitions at compile time.

#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
