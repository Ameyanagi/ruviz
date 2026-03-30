#[cfg(not(feature = "typst-math"))]
#[test]
fn typst_requires_feature() {
    // `trybuild` snapshots are compiler-version sensitive. Refresh them with
    // `TRYBUILD=overwrite cargo test --test typst_feature_gate_ui --no-default-features`
    // when the toolchain changes and the diagnostics legitimately shift.
    let test_cases = trybuild::TestCases::new();
    test_cases.compile_fail("tests/ui/typst_requires_feature/*.rs");
}

#[cfg(feature = "typst-math")]
#[test]
fn typst_with_feature_compiles() {
    let test_cases = trybuild::TestCases::new();
    test_cases.pass("tests/ui/typst_with_feature/*.rs");
}
