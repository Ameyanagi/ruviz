//! Repository-level integration contracts.
//!
//! Cargo discovers `tests/integration/main.rs` as the `integration` test
//! target, so every module declared here is compiled and executed by a plain
//! `cargo test`. Contracts that are about the repository as a whole — rather
//! than about one plot type — live here.
//!
//! This directory previously held five files (`basic_line.rs`, `data_types.rs`,
//! `large_dataset.rs`, `multi_series.rs`, `publication.rs`, 1 611 lines) with no
//! `main.rs` and no `[[test]]` entry, so cargo never compiled them. They had
//! drifted far enough from the crate that they no longer compiled at all — they
//! called `.scientific_notation(..)` and `.margin(..)` on `PlotBuilder`, which
//! only `Plot` has; they passed `Color::from_hex(..)`, a `Result`, where a
//! `Color` is expected; they passed integer literals to `ThemeBuilder::font_size(f32)`.
//! They were deleted rather than repaired: their subjects are covered by
//! `tests/simple_api_test.rs`, `tests/data_format_compatibility_test.rs`,
//! `tests/ecosystem_data_integration_test.rs`, `tests/export_format_tests.rs`,
//! `tests/full_pipeline_test.rs` and `tests/performance_validation.rs`, all of
//! which do compile and run.

mod ci_test_coverage;
