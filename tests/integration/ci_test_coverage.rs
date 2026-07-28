//! CI has to actually run every test target this repository compiles.
//!
//! Cargo discovers one integration test target per `tests/*.rs` file and per
//! `tests/*/main.rs`, but nothing makes `.github/workflows/ci.yml` execute
//! them. For a long time it compiled 45 test targets and ran about 17: no
//! `cargo test` invocation enabled `pdf`, `gpu`, `interactive`,
//! `polars_support` or `nalgebra_support`, so CI never produced a PDF byte,
//! never ingested a polars `DataFrame`, never exercised the interactive event
//! pipeline, and never executed the geometric assertions in
//! `tests/three_d_correctness_test.rs` — the only correctness gate on the 3D
//! feature.
//!
//! Fixing those seventeen commands by hand would not have stopped the next
//! test file from being added and silently never run. This module is the
//! mechanism instead: it reads the workflow, reconstructs which targets each
//! `cargo test` invocation actually executes, and fails when a discovered
//! target is claimed by none of them. Adding `tests/new_thing.rs` without
//! telling CI how to run it is now a test failure.
//!
//! # Why a whole-suite run does not count as coverage
//!
//! `cargo test --all-features --tests` executes every target cargo discovers,
//! so crediting it would make this guard unable to fail by construction: any
//! file added under `tests/` inherits coverage the moment it exists, and the
//! guard degenerates into "the whole-suite run is still in the workflow".
//!
//! It is also the wrong configuration to validate a suite in. `--all-features`
//! is the one feature combination no user ships — it is not the default set,
//! and it silently switches off every `cfg(not(feature = ..))` branch, which is
//! exactly what `tests/typst_feature_gate_ui.rs` exists to assert.
//!
//! So coverage here means *explicit assignment*: some job that runs on a pull
//! request must name the target in a `--test` flag while enabling the features
//! the target's `cfg`s need. The whole-suite run stays in the workflow as a
//! safety net — [`whole_suite_safety_net_still_runs_on_pull_requests`] fails if
//! it is removed — but it is deliberately not what keeps this guard green.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The workflow that must execute every test target the repository compiles.
const WORKFLOW_PATH: &str = ".github/workflows/ci.yml";

/// Environment variable a job sets to declare the rustc version it expects.
const EXPECTED_RUSTC_KEY: &str = "EXPECTED_RUSTC";

/// Flags that consume the following token as their value.
const VALUE_FLAGS: &[&str] = &[
    "--test",
    "--features",
    "--bin",
    "--example",
    "--bench",
    "--package",
    "-p",
    "--target",
    "--manifest-path",
    "--profile",
    "--jobs",
    "-j",
    "--color",
    "--message-format",
];

/// Flags that point `cargo test` at targets other than the integration tests,
/// so the invocation cannot stand in for "everything under `tests/`".
const NARROWING_FLAGS: &[&str] = &["--lib", "--doc", "--bins", "--benches", "--examples"];

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_repository_file(relative: &str) -> String {
    let path = repository_root().join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

/// A cargo test target discovered under `tests/`.
#[derive(Debug)]
struct TestTarget {
    /// The `--test <name>` name cargo gives it.
    name: String,
    /// Path relative to the repository root, for failure messages.
    source: String,
    /// Features that must be enabled before the target's assertions run.
    /// A run that does not enable `pdf` cannot honestly claim to cover
    /// `tests/pdf_export_test.rs`, whose every test is `cfg(feature = "pdf")`
    /// and which therefore runs zero assertions.
    required_features: BTreeSet<String>,
}

/// One `(` group while scanning a source file.
#[derive(Clone, Copy)]
struct CfgGroup {
    /// Inside a `cfg(..)` or `cfg_attr(..)` tree.
    conditional: bool,
    /// Inside a `not(..)` subtree of one, where naming a feature demands its
    /// *absence* rather than its presence.
    negated: bool,
}

/// Features a target needs before its assertions execute.
///
/// Only positive positions inside a `cfg(..)`/`cfg_attr(..)` tree count.
/// Scanning the raw text instead used to read prose: `tests/feature_hygiene_test.rs`
/// documents its own subject with `cfg(feature = "...")` in a `//!` comment and
/// so appeared to require features named `parallel`, `window` and `...` — the
/// last of which no run can ever enable. Comments are stripped first and
/// `not(..)` subtrees are skipped.
///
/// The union over the remaining sites is still an over-approximation for
/// `any(..)`, which is deliberate: it can only ask a lane to enable more, never
/// to enable less, so it cannot manufacture false coverage.
fn required_features(source: &Path) -> BTreeSet<String> {
    let text = strip_comments(&fs::read_to_string(source).unwrap_or_default());
    let bytes = text.as_bytes();
    let mut features = BTreeSet::new();
    let mut groups: Vec<CfgGroup> = Vec::new();
    let mut identifier = "";
    let mut identifier_start: Option<usize> = None;
    let mut saw_equals = false;
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_alphanumeric() || byte == b'_' {
            identifier_start.get_or_insert(index);
            index += 1;
            continue;
        }
        if let Some(start) = identifier_start.take() {
            identifier = &text[start..index];
            saw_equals = false;
        }
        match byte {
            b'(' => {
                let parent = groups.last().copied();
                groups.push(CfgGroup {
                    conditional: matches!(identifier, "cfg" | "cfg_attr")
                        || parent.is_some_and(|group| group.conditional),
                    negated: identifier == "not" || parent.is_some_and(|group| group.negated),
                });
                identifier = "";
                index += 1;
            }
            b')' => {
                groups.pop();
                identifier = "";
                index += 1;
            }
            b'=' => {
                saw_equals = true;
                index += 1;
            }
            b'"' => {
                let (literal, end) = read_string_literal(&text, index);
                let group = groups.last().copied();
                let wanted = saw_equals
                    && identifier == "feature"
                    && group.is_some_and(|group| group.conditional && !group.negated);
                if wanted {
                    features.insert(literal.to_string());
                }
                identifier = "";
                saw_equals = false;
                index = end;
            }
            b'\'' => index = skip_character_literal(bytes, index),
            byte if byte.is_ascii_whitespace() => index += 1,
            _ => {
                identifier = "";
                saw_equals = false;
                index += 1;
            }
        }
    }
    features
}

/// The contents of the string literal starting at `start`, and the index just
/// past its closing quote. Escapes are left as written; a feature name never
/// contains one, and every other caller only wants the end index.
fn read_string_literal(text: &str, start: usize) -> (&str, usize) {
    let bytes = text.as_bytes();
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return (&text[start + 1..index], index + 1),
            _ => index += 1,
        }
    }
    (&text[start + 1..], bytes.len())
}

/// Index just past a `'x'` character literal, or just past the quote when the
/// quote opens a lifetime such as `'a` instead.
fn skip_character_literal(bytes: &[u8], start: usize) -> usize {
    if bytes.get(start + 1) == Some(&b'\\') {
        let mut index = start + 2;
        while index < bytes.len() && bytes[index] != b'\'' {
            index += 1;
        }
        return index + 1;
    }
    if bytes.get(start + 2) == Some(&b'\'') {
        return start + 3;
    }
    start + 1
}

/// The source with `//` and `/* */` comments removed, string and character
/// literals left intact.
fn strip_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut stripped = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                let mut depth = 1_usize;
                index += 2;
                while index < bytes.len() && depth > 0 {
                    if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                        depth += 1;
                        index += 2;
                    } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        depth -= 1;
                        index += 2;
                    } else {
                        index += 1;
                    }
                }
            }
            b'"' => {
                let end = read_string_literal(text, index).1;
                stripped.push_str(&text[index..end]);
                index = end;
            }
            b'\'' => {
                let end = skip_character_literal(bytes, index);
                stripped.push_str(&text[index..end.min(bytes.len())]);
                index = end;
            }
            _ => {
                let end = next_character_boundary(text, index);
                stripped.push_str(&text[index..end]);
                index = end;
            }
        }
    }
    stripped
}

fn next_character_boundary(text: &str, index: usize) -> usize {
    let mut end = index + 1;
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    end.min(text.len())
}

/// The integration test targets cargo auto-discovers: every `tests/*.rs` and
/// every `tests/<dir>/main.rs`.
fn discover_test_targets() -> Vec<TestTarget> {
    let root = repository_root();
    let tests_directory = root.join("tests");
    let entries = fs::read_dir(&tests_directory)
        .unwrap_or_else(|error| panic!("failed to list {}: {error}", tests_directory.display()));

    let mut targets = Vec::new();
    for entry in entries {
        let path = entry.expect("a readable tests/ directory entry").path();
        let source = if path.is_dir() {
            path.join("main.rs")
        } else {
            path.clone()
        };
        if !source.is_file() {
            continue;
        }
        if source.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("a UTF-8 test target name")
            .to_string();
        let relative = source.strip_prefix(&root).unwrap_or(source.as_path());
        targets.push(TestTarget {
            name,
            source: relative.to_string_lossy().replace('\\', "/"),
            required_features: required_features(&source),
        });
    }
    targets.sort_by(|left, right| left.name.cmp(&right.name));
    targets
}

/// One `cargo test` invocation found in the workflow.
#[derive(Debug)]
struct CargoTestInvocation {
    /// Targets named with `--test <name>`.
    named_targets: BTreeSet<String>,
    /// Enabled features; `None` means `--all-features`.
    features: Option<BTreeSet<String>>,
    /// True when the invocation runs every integration test target.
    runs_every_target: bool,
    /// False for `--no-run` and for a positional test-name filter, both of
    /// which leave the target's assertions unexecuted.
    executes: bool,
}

impl CargoTestInvocation {
    /// Whether this invocation is an explicit assignment of `target` to a lane.
    ///
    /// Naming the target is required: see the module docs for why a whole-suite
    /// run is deliberately not enough.
    fn covers(&self, target: &TestTarget) -> bool {
        if !self.executes || !self.named_targets.contains(&target.name) {
            return false;
        }
        match &self.features {
            None => true,
            Some(enabled) => {
                let required = &target.required_features;
                required.iter().all(|feature| enabled.contains(feature))
            }
        }
    }

    /// Whether this is the belt-and-braces run of every target under every
    /// feature. It attributes nothing, but losing it would mean a target that
    /// nobody remembered to assign is not merely unassigned but never run.
    fn is_whole_suite_safety_net(&self) -> bool {
        self.executes && self.runs_every_target && self.features.is_none()
    }
}

fn parse_cargo_test(text: &str, default_features: &BTreeSet<String>) -> CargoTestInvocation {
    let mut named_targets = BTreeSet::new();
    let mut explicit_features = BTreeSet::new();
    let mut all_features = false;
    let mut no_default_features = false;
    let mut no_run = false;
    let mut positional_filter = false;
    let mut narrowed = false;
    let mut selects_all_tests = false;

    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        index += 1;
        if token == "--" {
            break;
        }
        let (flag, inline_value) = match token.split_once('=') {
            Some((flag, value)) if flag.starts_with('-') => (flag, Some(value)),
            _ => (token, None),
        };
        let value = match inline_value {
            Some(value) => Some(value),
            None if VALUE_FLAGS.contains(&flag) => {
                let value = tokens.get(index).copied();
                index += 1;
                value
            }
            None => None,
        };
        match flag {
            "--test" => {
                if let Some(value) = value {
                    named_targets.insert(value.trim_matches('"').to_string());
                }
            }
            "--features" => {
                if let Some(value) = value {
                    for name in value.split([',', ' ']).filter(|name| !name.is_empty()) {
                        explicit_features.insert(name.trim_matches('"').to_string());
                    }
                }
            }
            "--all-features" => all_features = true,
            "--no-default-features" => no_default_features = true,
            "--no-run" => no_run = true,
            "--tests" | "--all-targets" => selects_all_tests = true,
            _ if NARROWING_FLAGS.contains(&flag) => narrowed = true,
            _ if flag.starts_with('-') => {}
            _ => positional_filter = true,
        }
    }

    let features = if all_features {
        None
    } else {
        let mut enabled = explicit_features;
        if !no_default_features {
            enabled.extend(default_features.iter().cloned());
        }
        Some(enabled)
    };

    CargoTestInvocation {
        runs_every_target: selects_all_tests || (!narrowed && named_targets.is_empty()),
        named_targets,
        features,
        executes: !no_run && !positional_filter,
    }
}

/// A job's top-level `if:` expression, if it has one.
///
/// The body lines keep their workflow indentation, so a job key is exactly
/// four spaces deep; anything deeper belongs to a step, whose condition gates
/// one command rather than the whole job.
fn job_condition(body: &str) -> Option<&str> {
    body.lines()
        .filter(|line| line.starts_with("    ") && !line.starts_with("     "))
        .find_map(|line| line.trim().strip_prefix("if:"))
        .map(str::trim)
}

/// Whether a job runs on a pull request.
///
/// A job gated on `github.event_name` without naming `pull_request` never runs
/// at review time, so anything it executes is not covered when the verdict
/// could still change something. Treating such a job as covering a target is
/// exactly how a suite goes back to being compiled-but-never-run: move the
/// whole-suite invocation into the nightly job and every target looks covered.
/// Conditions that do not switch on the event (`needs.*`, `always()`, …) run on
/// pull requests like any ungated job.
fn runs_on_pull_requests(body: &str) -> bool {
    match job_condition(body) {
        Some(condition) => {
            !condition.contains("github.event_name") || condition.contains("pull_request")
        }
        None => true,
    }
}

/// Every `cargo test` the workflow runs on a pull request.
///
/// A segment only counts when `cargo test` starts the command, so
/// `cd python && uv run cargo test` — which tests the separate `python`
/// package — is correctly not credited to this crate's targets.
fn cargo_test_invocations(
    workflow: &str,
    default_features: &BTreeSet<String>,
) -> Vec<CargoTestInvocation> {
    let mut invocations = Vec::new();
    for (_, body) in workflow_jobs(workflow) {
        if !runs_on_pull_requests(&body) {
            continue;
        }
        for line in body.lines() {
            for segment in line.split("&&") {
                let Some(index) = segment.find("cargo test") else {
                    continue;
                };
                let prefix = segment[..index].trim();
                let starts_command =
                    prefix.is_empty() || prefix.ends_with("run:") || prefix.ends_with("command:");
                if !starts_command {
                    continue;
                }
                let arguments = &segment[index + "cargo test".len()..];
                invocations.push(parse_cargo_test(arguments, default_features));
            }
        }
    }
    invocations
}

/// The crate's default features, as named in `Cargo.toml`.
fn default_features(manifest: &str) -> BTreeSet<String> {
    let list = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("default = ["));
    let Some(list) = list else {
        return BTreeSet::new();
    };
    let mut features = BTreeSet::new();
    for (index, part) in list.split('"').enumerate() {
        if index % 2 == 1 {
            features.insert(part.to_string());
        }
    }
    features
}

/// Split the workflow into `(job id, job body)` pairs.
fn workflow_jobs(workflow: &str) -> Vec<(String, String)> {
    let mut jobs: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    let mut inside_jobs = false;
    for line in workflow.lines() {
        if line.trim_end() == "jobs:" {
            inside_jobs = true;
            continue;
        }
        if !inside_jobs {
            continue;
        }
        let is_job_header = line.starts_with("  ")
            && !line.starts_with("   ")
            && line.trim_end().ends_with(':')
            && !line.trim_start().starts_with('#');
        if is_job_header {
            if let Some((name, body)) = current.take() {
                jobs.push((name, body.join("\n")));
            }
            current = Some((line.trim().trim_end_matches(':').to_string(), Vec::new()));
            continue;
        }
        if let Some((_, body)) = current.as_mut() {
            body.push(line);
        }
    }
    if let Some((name, body)) = current {
        jobs.push((name, body.join("\n")));
    }
    jobs
}

fn describe_uncovered(target: &TestTarget) -> String {
    let required = &target.required_features;
    let features = if required.is_empty() {
        "no features".to_string()
    } else {
        let names: Vec<&str> = required.iter().map(String::as_str).collect();
        format!("features {}", names.join(", "))
    };
    format!(
        "  --test {} ({}, needs {})",
        target.name, target.source, features
    )
}

#[test]
fn ci_executes_every_test_target_cargo_compiles() {
    let workflow = read_repository_file(WORKFLOW_PATH);
    let manifest = read_repository_file("Cargo.toml");
    let invocations = cargo_test_invocations(&workflow, &default_features(&manifest));

    assert!(
        invocations.iter().any(|invocation| invocation.executes),
        "{WORKFLOW_PATH} contains no `cargo test` invocation that executes anything, \
         so this guard would pass vacuously"
    );

    let targets = discover_test_targets();
    assert!(
        targets.len() >= 20,
        "only discovered {} test targets under tests/, which means discovery broke \
         rather than that the suite shrank",
        targets.len()
    );

    let mut uncovered = Vec::new();
    for target in &targets {
        let covered = invocations.iter().any(|entry| entry.covers(target));
        if !covered {
            uncovered.push(describe_uncovered(target));
        }
    }

    assert!(
        uncovered.is_empty(),
        "{WORKFLOW_PATH} compiles these test targets but no job that runs on a pull request \
         assigns them to a lane:\n{}\n\n\
         Fix this in the workflow, not here: name each target in a `--test` flag of a job that \
         enables the features it needs. A `--no-run` compile and a run with a positional \
         test-name filter both leave the target's assertions unexecuted, so neither counts, and \
         neither does the whole-suite `cargo test --all-features --tests` — it executes every \
         target by construction, so crediting it would make this guard incapable of failing, and \
         `--all-features` is in any case the one configuration no user ships.",
        uncovered.join("\n")
    );
}

/// The whole-suite run attributes no coverage, but it must still exist.
///
/// Explicit assignment says every target has a lane that runs it in a
/// configuration chosen for it. This says that a target somebody forgets to
/// assign is still executed somewhere while the guard is red, rather than being
/// silently absent from CI until the assignment lands.
#[test]
fn whole_suite_safety_net_still_runs_on_pull_requests() {
    let workflow = read_repository_file(WORKFLOW_PATH);
    let manifest = read_repository_file("Cargo.toml");
    let invocations = cargo_test_invocations(&workflow, &default_features(&manifest));

    assert!(
        invocations
            .iter()
            .any(CargoTestInvocation::is_whole_suite_safety_net),
        "no job in {WORKFLOW_PATH} that runs on a pull request executes \
         `cargo test --all-features --tests`. Explicit `--test` assignment is what this guard \
         checks, but the whole-suite run is what makes a missed assignment merely unattributed \
         instead of unrun."
    );
}

#[test]
fn every_job_pinning_a_toolchain_asserts_the_rustc_it_actually_got() {
    let workflow = read_repository_file(WORKFLOW_PATH);
    let mut pinned_jobs = 0;

    for (job, body) in workflow_jobs(&workflow) {
        let pinned: BTreeSet<String> = body
            .lines()
            .filter_map(|line| line.trim().strip_prefix("toolchain:"))
            .map(|value| value.trim().trim_matches('"').to_string())
            .filter(|value| value.starts_with(|c: char| c.is_ascii_digit()))
            .collect();

        for version in &pinned {
            pinned_jobs += 1;
            assert!(
                body.contains(&format!("{EXPECTED_RUSTC_KEY}: \"{version}\"")),
                "job `{job}` pins toolchain {version} but never asserts the rustc it got. \
                 A tracked rust-toolchain.toml outranks some toolchain selections, so the job \
                 can silently compile with a different compiler and still go green. Add a step \
                 with `env: {EXPECTED_RUSTC_KEY}: \"{version}\"` that compares it against \
                 `rustc --version`."
            );
        }
    }

    assert!(
        pinned_jobs > 0,
        "no job in {WORKFLOW_PATH} pins a toolchain version any more, so this guard would pass \
         vacuously"
    );
}

#[test]
fn workflow_parsing_recognises_the_shapes_ci_actually_uses() {
    let defaults = BTreeSet::from(["ndarray_support".to_string(), "parallel".to_string()]);
    let workflow = "\
jobs:
  example:
    steps:
      - name: Compile only
        run: cargo test --tests --no-run --verbose
      - name: Filtered
        run: cargo test --test alpha some_test_name -- --exact
      - name: Named with features
        run: cargo test --no-default-features --features 3d,gpu --test beta
      - name: Everything
        run: cargo test --all-features --tests
      - name: Another package
        run: cd python && uv run cargo test
";
    let invocations = cargo_test_invocations(workflow, &defaults);
    assert_eq!(
        invocations.len(),
        4,
        "the separate `python` package run must not be credited to this crate"
    );

    let compile_only = &invocations[0];
    assert!(!compile_only.executes, "--no-run executes nothing");

    let filtered = &invocations[1];
    assert!(!filtered.executes, "a positional filter skips most tests");

    let named = &invocations[2];
    assert!(named.executes);
    assert!(!named.runs_every_target);
    assert_eq!(
        named.features.as_ref().expect("an explicit feature set"),
        &BTreeSet::from(["3d".to_string(), "gpu".to_string()]),
        "--no-default-features must drop the default features"
    );

    let everything = &invocations[3];
    assert!(everything.executes && everything.runs_every_target);
    assert!(everything.features.is_none(), "--all-features enables all");
    assert!(everything.is_whole_suite_safety_net());

    let beta = TestTarget {
        name: "beta".to_string(),
        source: "tests/beta.rs".to_string(),
        required_features: BTreeSet::from(["3d".to_string()]),
    };
    assert!(named.covers(&beta));
    assert!(!compile_only.covers(&beta));
    assert!(
        !everything.covers(&beta),
        "a whole-suite run assigns no target to a lane, or a newly added test \
         file would inherit coverage the moment it existed"
    );

    let pdf = TestTarget {
        name: "pdf_export_test".to_string(),
        source: "tests/pdf_export_test.rs".to_string(),
        required_features: BTreeSet::from(["pdf".to_string()]),
    };
    assert!(!named.covers(&pdf), "a run without `pdf` misses pdf tests");
    assert!(!everything.covers(&pdf));
}

/// The property the whole thing exists for: a test file nobody wired into a
/// lane is a failure, even with the whole-suite run present.
#[test]
fn a_test_file_no_lane_names_is_uncovered() {
    let defaults = BTreeSet::new();
    let workflow = "\
jobs:
  matrix:
    steps:
      - run: cargo test --all-features --tests
      - run: cargo test --test alpha
";
    let invocations = cargo_test_invocations(workflow, &defaults);
    let target = |name: &str| TestTarget {
        name: name.to_string(),
        source: format!("tests/{name}.rs"),
        required_features: BTreeSet::new(),
    };
    assert!(
        invocations
            .iter()
            .any(|invocation| invocation.covers(&target("alpha")))
    );
    assert!(
        !invocations
            .iter()
            .any(|invocation| invocation.covers(&target("zz_dummy_probe"))),
        "the whole-suite run must not stand in for assigning a new file to a lane"
    );
    assert!(
        invocations
            .iter()
            .any(CargoTestInvocation::is_whole_suite_safety_net),
        "the safety net is still recognised, it just attributes nothing"
    );
}

/// Prose about `cfg` is not a `cfg`.
#[test]
fn required_features_reads_cfg_trees_and_not_comments() {
    let directory = repository_root().join("target/ci_test_coverage_fixtures");
    fs::create_dir_all(&directory).expect("a writable fixture directory");
    let path = directory.join("required_features_fixture.rs");
    fs::write(
        &path,
        r#"
//! Documents `cfg(feature = "documented_only")` in prose.
/* and cfg(feature = "block_comment_only") in a block comment */
#[cfg(feature = "needed")]
mod gated;

#[cfg(all(feature = "also_needed", unix))]
fn both() {}

#[cfg(not(feature = "absent"))]
fn fallback() {}

#[cfg(any(feature = "either_a", feature = "either_b"))]
fn either() {}

fn message() -> &'static str {
    "add a cfg(feature = \"quoted_only\") site"
}
"#,
    )
    .expect("the fixture is writable");

    let found = required_features(&path);
    assert!(found.contains("needed"), "a bare cfg names a requirement");
    assert!(found.contains("also_needed"), "all(..) names requirements");
    assert!(
        !found.contains("documented_only") && !found.contains("block_comment_only"),
        "comments are prose, not configuration: {found:?}"
    );
    assert!(
        !found.contains("absent"),
        "not(..) asks for a feature to be OFF, so it is not a requirement"
    );
    assert!(
        !found.contains("quoted_only"),
        "a string literal outside a cfg tree is not a cfg: {found:?}"
    );
    // any(..) is over-approximated rather than dropped: demanding more can only
    // ask a lane to enable extra features, never manufacture false coverage.
    assert!(found.contains("either_a") && found.contains("either_b"));

    fs::remove_file(&path).ok();
}

/// The real suite must not demand features that cannot exist.
#[test]
fn every_required_feature_is_declared_in_the_manifest() {
    let manifest = read_repository_file("Cargo.toml");
    let declared: BTreeSet<String> = manifest
        .lines()
        .skip_while(|line| line.trim() != "[features]")
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .filter_map(|line| line.split_once('='))
        .map(|(name, _)| name.trim().trim_matches('"').to_string())
        .filter(|name| !name.is_empty() && !name.starts_with('#'))
        .collect();
    assert!(
        declared.contains("pdf"),
        "failed to parse the [features] table, so this guard would pass vacuously"
    );

    for target in discover_test_targets() {
        for feature in &target.required_features {
            assert!(
                declared.contains(feature),
                "{} is read as requiring a feature `{feature}` that Cargo.toml does not \
                 declare, so no lane could ever be written to cover it",
                target.source
            );
        }
    }
}

/// A run that only happens on a schedule does not cover anything on a PR.
///
/// This is the hole the guard used to have: it read `run:` lines without
/// asking whether the job containing them runs at review time, so moving the
/// whole-suite `cargo test --all-features --tests` into the nightly job would
/// have left every target looking covered while no PR ever ran them again.
#[test]
fn schedule_gated_jobs_do_not_cover_anything_on_a_pull_request() {
    let defaults = BTreeSet::new();
    let workflow = "\
jobs:
  nightly:
    if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
    steps:
      - run: cargo test --all-features --tests
  pr-only:
    if: github.event_name == 'pull_request'
    steps:
      - run: cargo test --test alpha
  gated-on-another-job:
    if: needs.preview-scope.outputs.run_preview == 'true'
    steps:
      - run: cargo test --test beta
  ungated:
    steps:
      - name: skip a step-level condition
        if: github.event_name == 'schedule'
        run: cargo test --test gamma
";
    let invocations = cargo_test_invocations(workflow, &defaults);
    assert_eq!(
        invocations.len(),
        3,
        "the schedule-gated job must contribute no coverage"
    );
    assert!(
        invocations
            .iter()
            .all(|invocation| !invocation.runs_every_target),
        "the only whole-suite run lives in the schedule-gated job"
    );

    let target = |name: &str| TestTarget {
        name: name.to_string(),
        source: format!("tests/{name}.rs"),
        required_features: BTreeSet::new(),
    };
    for name in ["alpha", "beta", "gamma"] {
        assert!(
            invocations
                .iter()
                .any(|invocation| invocation.covers(&target(name))),
            "`{name}` is named by a job that does run on pull requests"
        );
    }
    assert!(
        !invocations
            .iter()
            .any(|invocation| invocation.covers(&target("delta"))),
        "nothing outside the schedule-gated whole-suite run covers `delta`"
    );
}

#[test]
fn default_features_are_read_from_the_manifest() {
    let manifest = read_repository_file("Cargo.toml");
    assert!(
        !default_features(&manifest).is_empty(),
        "failed to parse `default = [..]` out of Cargo.toml, so every invocation would look \
         like it enables no features at all"
    );
}
