#![cfg(feature = "3d")]

#[test]
fn canonical_3d_examples_compile() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/3d/pass/*.rs");
}

#[test]
fn ambiguous_or_malformed_3d_calls_do_not_compile() {
    // Refresh only after confirming that the public contract, rather than just
    // compiler wording, changed:
    // TRYBUILD=overwrite cargo test --no-default-features --features 3d \
    //   --test three_d_api_ui
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/3d/fail/*.rs");
}
