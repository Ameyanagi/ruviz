//! Cargo feature hygiene, enforced instead of reviewed.
//!
//! Two failure modes had both reached a shipped release, and neither is visible
//! in a normal build:
//!
//! 1. An optional dependency written as a bare name in a feature list
//!    (`parallel = ["rayon"]`) makes Cargo mint an *implicit* feature named
//!    after the crate. `--features rayon` then compiled rayon while every
//!    `cfg(feature = "parallel")` site stayed switched off — a flag that links a
//!    dependency and enables no code. Writing `dep:rayon` suppresses the
//!    implicit feature.
//!
//! 2. A declared feature that gates nothing. `window` pulled winit, softbuffer,
//!    rfd (GTK3) and arboard, and had zero `cfg(feature = "window")` sites, so
//!    enabling it changed nothing except the build graph.
//!
//! Both checks read the manifest and the sources as text, so they hold for every
//! feature combination without needing to be compiled under one.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Features that legitimately gate no code, with the reason they are exempt.
/// Anything else must have at least one `cfg(feature = "...")` site.
const FEATURES_WITHOUT_CFG_SITES: &[(&str, &str)] = &[
    ("default", "aggregate: selects other features"),
    ("full", "aggregate: selects other features"),
    ("performance", "aggregate: `parallel` + `simd`"),
    ("ndarray", "alias: forwards to `ndarray_support`"),
    ("nalgebra", "alias: forwards to `nalgebra_support`"),
    ("polars", "alias: forwards to `polars_support`"),
    ("window", "alias: forwards to `interactive`"),
    (
        "svg",
        "documented no-op: SVG export is unconditional, flag kept so existing \
         `features = [\"svg\"]` selections keep resolving",
    ),
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn manifest() -> String {
    fs::read_to_string(repo_root().join("Cargo.toml")).expect("Cargo.toml is readable")
}

/// The raw text of the `[features]` table, header excluded.
fn features_table(manifest: &str) -> String {
    let mut lines = manifest
        .lines()
        .skip_while(|line| line.trim() != "[features]");
    assert!(
        lines.next().is_some(),
        "Cargo.toml has no `[features]` table"
    );

    lines
        .take_while(|line| !line.trim_start().starts_with('['))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Feature names declared in the `[features]` table.
fn declared_features(table: &str) -> BTreeSet<String> {
    table
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (name, _) = line.split_once('=')?;
            let name = name.trim().trim_matches('"');
            if name.is_empty() {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Names of every optional dependency, across every `[*dependencies]` table.
fn optional_dependencies(manifest: &str) -> BTreeSet<String> {
    manifest
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.starts_with('#') && line.contains("optional = true")
        })
        .filter_map(|line| {
            let (name, _) = line.trim().split_once('=')?;
            let name = name.trim().trim_matches('"');
            if name.is_empty() {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

fn push_rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            push_rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Every `.rs` file the root crate builds, concatenated.
fn rust_sources() -> String {
    let root = repo_root();
    let mut files = Vec::new();
    for dir in ["src", "tests", "benches", "examples", "gallery", "scripts"] {
        push_rust_sources(&root.join(dir), &mut files);
    }
    assert!(
        files.len() > 100,
        "source scan found only {} .rs files — the directory walk is broken",
        files.len()
    );

    files
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_declared_feature_gates_something() {
    let manifest = manifest();
    let declared = declared_features(&features_table(&manifest));
    assert!(
        declared.contains("parallel") && declared.contains("gpu"),
        "feature parsing produced an implausible set: {declared:?}"
    );

    let sources = rust_sources();
    let exempt: BTreeSet<&str> = FEATURES_WITHOUT_CFG_SITES.iter().map(|(n, _)| *n).collect();

    let inert: Vec<&String> = declared
        .iter()
        .filter(|name| !exempt.contains(name.as_str()))
        .filter(|name| {
            !sources.contains(&format!("feature = \"{name}\""))
                && !sources.contains(&format!("feature=\"{name}\""))
        })
        .collect();

    assert!(
        inert.is_empty(),
        "these cargo features gate no code: {inert:?}\n\
         Either add a `cfg(feature = \"…\")` site, delete the feature, or — if it \
         is an alias or an aggregate — list it in FEATURES_WITHOUT_CFG_SITES with \
         a reason."
    );
}

#[test]
fn exemptions_are_still_declared_features() {
    let manifest = manifest();
    let declared = declared_features(&features_table(&manifest));

    for (name, reason) in FEATURES_WITHOUT_CFG_SITES {
        assert!(
            declared.contains(*name),
            "FEATURES_WITHOUT_CFG_SITES lists `{name}` ({reason}) but Cargo.toml no \
             longer declares it — drop the stale exemption"
        );
    }
}

#[test]
fn optional_dependencies_never_leak_as_implicit_features() {
    let manifest = manifest();
    let table = features_table(&manifest);
    let declared = declared_features(&table);

    let optional = optional_dependencies(&manifest);
    assert!(
        optional.contains("rayon") && optional.contains("winit"),
        "optional-dependency parsing produced an implausible set: {optional:?}"
    );

    let mut problems = Vec::new();
    for dependency in &optional {
        // A dependency that is reachable only as a bare name in some feature
        // list keeps its implicit feature. One `dep:` reference anywhere in the
        // table is enough to suppress it.
        let referenced_as_dep = table.contains(&format!("dep:{dependency}"));
        // An explicit feature of the same name is fine: it shadows the implicit
        // one and is what a user actually gets.
        let shadowed = declared.contains(dependency);

        if !referenced_as_dep && !shadowed {
            problems.push(dependency.clone());
        }
    }

    assert!(
        problems.is_empty(),
        "these optional dependencies leak as implicit cargo features: {problems:?}\n\
         Reference them as `dep:<name>` in the feature that needs them, so that \
         `--features <name>` cannot silently link the crate while leaving the \
         real feature switched off."
    );
}
