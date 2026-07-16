#!/usr/bin/env rust-script
//! Generate curated Rust gallery assets and markdown indexes.
//!
//! Usage: cargo run --bin generate_gallery

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const GALLERY_DOCS_ROOT: &str = "docs/gallery";
pub const GALLERY_ASSETS_ROOT: &str = "docs/assets/gallery/rust";

#[derive(Clone, Copy)]
pub struct Category {
    pub slug: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExampleRun {
    pub name: &'static str,
    pub features: Option<&'static str>,
}

#[derive(Clone, Copy)]
pub enum AssetSource {
    Example {
        run: ExampleRun,
        output_rel: &'static str,
    },
    Copy {
        source_rel: &'static str,
    },
}

#[derive(Clone, Copy)]
pub struct GalleryEntry {
    pub category: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub asset_name: &'static str,
    pub source_path: &'static str,
    pub guide: Option<(&'static str, &'static str)>,
    pub source: AssetSource,
}

pub fn categories() -> Vec<Category> {
    vec![
        Category {
            slug: "basic",
            title: "Basic Plots",
            description: "Fundamental plot types for everyday visualization and quick starts.",
            icon: "📊",
        },
        Category {
            slug: "statistical",
            title: "Statistical Plots",
            description: "Distribution, density, and uncertainty-focused plot recipes.",
            icon: "📈",
        },
        Category {
            slug: "publication",
            title: "Publication Quality",
            description: "Layouts and themes tuned for papers, reports, and slides.",
            icon: "📄",
        },
        Category {
            slug: "performance",
            title: "Performance",
            description: "Large-dataset examples and optimization-oriented render outputs.",
            icon: "⚡",
        },
        Category {
            slug: "advanced",
            title: "Advanced Techniques",
            description: "Styling, polar/radar, and layout-heavy visualizations.",
            icon: "🎨",
        },
        Category {
            slug: "animation",
            title: "Animation",
            description: "GIF examples generated from the animation helpers and `record!` flows.",
            icon: "🎬",
        },
        Category {
            slug: "internationalization",
            title: "Internationalization",
            description: "Examples covering multilingual text layout and CJK rendering.",
            icon: "🌍",
        },
    ]
}

pub fn entries() -> Vec<GalleryEntry> {
    vec![
        GalleryEntry {
            category: "basic",
            title: "Line Plot",
            summary: "The core line example used across the README and rustdoc.",
            asset_name: "line_plot.png",
            source_path: "examples/doc_line_plot.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/line_plot.png",
            },
        },
        GalleryEntry {
            category: "basic",
            title: "Scatter Plot",
            summary: "A compact point-cloud example for discrete observations.",
            asset_name: "scatter_plot.png",
            source_path: "examples/doc_scatter_plot.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/scatter_plot.png",
            },
        },
        GalleryEntry {
            category: "basic",
            title: "Bar Chart",
            summary: "Categorical values rendered as a simple bar chart.",
            asset_name: "bar_chart.png",
            source_path: "examples/doc_bar_chart.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/bar_chart.png",
            },
        },
        GalleryEntry {
            category: "basic",
            title: "Heatmap",
            summary: "Matrix data shown with a continuous color scale.",
            asset_name: "heatmap.png",
            source_path: "examples/doc_heatmap.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/heatmap.png",
            },
        },
        GalleryEntry {
            category: "statistical",
            title: "Histogram",
            summary: "Distribution counts rendered with the default histogram styling.",
            asset_name: "histogram.png",
            source_path: "examples/doc_histogram.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/histogram.png",
            },
        },
        GalleryEntry {
            category: "statistical",
            title: "Box Plot",
            summary: "Quartiles, whiskers, and outliers in a compact statistical summary.",
            asset_name: "boxplot.png",
            source_path: "examples/doc_boxplot.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/boxplot.png",
            },
        },
        GalleryEntry {
            category: "statistical",
            title: "Kernel Density Estimate",
            summary: "KDE example copied from the rustdoc image set.",
            asset_name: "kde_plot.png",
            source_path: "examples/doc_kde.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/kde_plot.png",
            },
        },
        GalleryEntry {
            category: "statistical",
            title: "ECDF",
            summary: "Empirical CDF example copied from the rustdoc image set.",
            asset_name: "ecdf_plot.png",
            source_path: "examples/doc_ecdf.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/ecdf_plot.png",
            },
        },
        GalleryEntry {
            category: "statistical",
            title: "Violin Plot",
            summary: "Distribution plot with quartile-aware styling.",
            asset_name: "violin_plot.png",
            source_path: "examples/doc_violin.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/violin_plot.png",
            },
        },
        GalleryEntry {
            category: "statistical",
            title: "Error Bars",
            summary: "Uncertainty intervals attached to line and scatter series.",
            asset_name: "errorbar_plot.png",
            source_path: "examples/doc_errorbar.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/errorbar_plot.png",
            },
        },
        GalleryEntry {
            category: "publication",
            title: "Scientific Analysis Figure",
            summary: "Multi-panel figure assembled for report-style presentation.",
            asset_name: "scientific_analysis_figure.png",
            source_path: "examples/scientific_showcase.rs",
            guide: Some(("Subplots & Composition", "../../guide/06_subplots.md")),
            source: AssetSource::Example {
                run: ExampleRun {
                    name: "scientific_showcase",
                    features: None,
                },
                output_rel: "generated/examples/scientific_analysis_figure.png",
            },
        },
        GalleryEntry {
            category: "publication",
            title: "Publication Theme",
            summary: "Publication-oriented theme reference used by docs and comparisons.",
            asset_name: "theme_publication.png",
            source_path: "examples/doc_themes.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/theme_publication.png",
            },
        },
        GalleryEntry {
            category: "publication",
            title: "Mixed Plots in a 2×2 Grid",
            summary: "A composition example combining a line plot, scatter plot, bar chart, and multi-series comparison with a legend in a 2×2 grid.",
            asset_name: "subplots.png",
            source_path: "examples/doc_subplots.rs",
            guide: Some(("Subplots & Composition", "../../guide/06_subplots.md")),
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/subplots.png",
            },
        },
        GalleryEntry {
            category: "publication",
            title: "Typst Labels",
            summary: "Publication text rendered through Typst math mode.",
            asset_name: "typst_text.png",
            source_path: "examples/doc_typst_text.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/typst_text.png",
            },
        },
        GalleryEntry {
            category: "performance",
            title: "Parallel Multi-Series",
            summary: "A large multi-series render produced by the parallel example suite.",
            asset_name: "parallel_multi_series.png",
            source_path: "examples/parallel_demo.rs",
            guide: None,
            source: AssetSource::Example {
                run: ExampleRun {
                    name: "parallel_demo",
                    features: None,
                },
                output_rel: "generated/examples/parallel_multi_series.png",
            },
        },
        GalleryEntry {
            category: "performance",
            title: "Memory-Optimized Signal",
            summary: "A dense signal render generated by the memory optimization example.",
            asset_name: "memory_optimization_demo.png",
            source_path: "examples/memory_optimization_demo.rs",
            guide: None,
            source: AssetSource::Example {
                run: ExampleRun {
                    name: "memory_optimization_demo",
                    features: None,
                },
                output_rel: "generated/examples/memory_optimization_demo.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Contour Plot",
            summary: "Contour rendering example with level interpolation.",
            asset_name: "contour_plot.png",
            source_path: "examples/doc_contour.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/contour_plot.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Radar Chart",
            summary: "Radar chart example demonstrating non-cartesian layout support.",
            asset_name: "radar_chart.png",
            source_path: "examples/doc_radar.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/radar_chart.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Pie Chart",
            summary: "Composition shares with labels and percentages.",
            asset_name: "pie_chart.png",
            source_path: "examples/doc_pie.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/pie_chart.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Donut Chart",
            summary: "A pie chart variant with a central cutout.",
            asset_name: "pie_donut.png",
            source_path: "examples/doc_pie.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/pie_donut.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Polar Rose",
            summary: "A filled polar line plot for non-cartesian data.",
            asset_name: "polar_plot.png",
            source_path: "examples/doc_polar.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/polar_plot.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Color Palette",
            summary: "Default palette reference across multiple line series.",
            asset_name: "colors.png",
            source_path: "examples/doc_colors.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/colors.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Marker Styles",
            summary: "Reference image covering filled and open marker variants.",
            asset_name: "marker_styles.png",
            source_path: "examples/doc_marker_styles.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/marker_styles.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Line Styles",
            summary: "Reference image covering solid, dashed, and dotted lines.",
            asset_name: "line_styles.png",
            source_path: "examples/doc_line_styles.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/line_styles.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Legend Positions",
            summary: "Reference image covering legend placement options.",
            asset_name: "legend_positions.png",
            source_path: "examples/doc_legend_positions.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/legend_positions.png",
            },
        },
        GalleryEntry {
            category: "animation",
            title: "Traveling Sine Wave",
            summary: "Animated sine wave generated with the `record!` macro.",
            asset_name: "animation_sine_wave.gif",
            source_path: "examples/generate_animation_gallery.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/animation_sine_wave.gif",
            },
        },
        GalleryEntry {
            category: "animation",
            title: "Animated Bars",
            summary: "Animated categorical data example rendered as a GIF.",
            asset_name: "animation_bars.gif",
            source_path: "examples/generate_animation_gallery.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/animation_bars.gif",
            },
        },
        GalleryEntry {
            category: "animation",
            title: "Wave Interference",
            summary: "Animated wave interference example rendered as a GIF.",
            asset_name: "animation_interference.gif",
            source_path: "examples/generate_animation_gallery.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/animation_interference.gif",
            },
        },
        GalleryEntry {
            category: "internationalization",
            title: "Japanese Labels",
            summary: "Japanese-language labels rendered with the default browser/document fonts.",
            asset_name: "international_japanese.png",
            source_path: "examples/doc_international.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/international_japanese.png",
            },
        },
        GalleryEntry {
            category: "internationalization",
            title: "Chinese Labels",
            summary: "Chinese-language bar chart rendering example.",
            asset_name: "international_chinese.png",
            source_path: "examples/doc_international.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/international_chinese.png",
            },
        },
        GalleryEntry {
            category: "internationalization",
            title: "Korean Labels",
            summary: "Korean-language line chart rendering example.",
            asset_name: "international_korean.png",
            source_path: "examples/doc_international.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/international_korean.png",
            },
        },
        GalleryEntry {
            category: "internationalization",
            title: "Multi-Language Comparison",
            summary: "A four-panel comparison of Japanese, Chinese, Korean, and English labels with identical sine/cosine content.",
            asset_name: "international_comparison.png",
            source_path: "examples/doc_international.rs",
            guide: None,
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/international_comparison.png",
            },
        },
    ]
}

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

pub fn gallery_asset_path(repo_root: &Path, entry: &GalleryEntry) -> PathBuf {
    repo_root
        .join(GALLERY_ASSETS_ROOT)
        .join(entry.category)
        .join(entry.asset_name)
}

pub fn copy_source_path(repo_root: &Path, entry: &GalleryEntry) -> Option<PathBuf> {
    match entry.source {
        AssetSource::Copy { source_rel } => Some(repo_root.join(source_rel)),
        AssetSource::Example { .. } => None,
    }
}

pub fn expected_assets_by_category(entries: &[GalleryEntry]) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut expected = BTreeMap::new();
    for entry in entries {
        expected
            .entry(entry.category)
            .or_insert_with(BTreeSet::new)
            .insert(entry.asset_name);
    }
    expected
}

pub fn validate_catalog(
    repo_root: &Path,
    categories: &[Category],
    entries: &[GalleryEntry],
) -> Result<(), Vec<String>> {
    let mut category_slugs = BTreeSet::new();
    let mut destinations = BTreeSet::new();
    let mut errors = Vec::new();

    for category in categories {
        if !category_slugs.insert(category.slug) {
            errors.push(format!(
                "duplicate gallery category slug: {}",
                category.slug
            ));
        }
    }

    for entry in entries {
        if !category_slugs.contains(entry.category) {
            errors.push(format!(
                "catalog entry `{}` references unknown category `{}`",
                entry.title, entry.category
            ));
        }
        if !destinations.insert((entry.category, entry.asset_name)) {
            errors.push(format!(
                "duplicate gallery destination: {}/{}/{}",
                GALLERY_ASSETS_ROOT, entry.category, entry.asset_name
            ));
        }

        let source_path = repo_root.join(entry.source_path);
        if !source_path.is_file() {
            errors.push(format!(
                "catalog source for `{}` does not exist: {}",
                entry.title,
                source_path.display()
            ));
        }

        if let Some((_, guide_path)) = entry.guide {
            let resolved = repo_root
                .join(GALLERY_DOCS_ROOT)
                .join(entry.category)
                .join(guide_path);
            if !resolved.is_file() {
                errors.push(format!(
                    "guide target for `{}` does not exist: {}",
                    entry.title,
                    resolved.display()
                ));
            }
        }

        if let Some(source_path) = copy_source_path(repo_root, entry) {
            if !source_path.is_file() {
                errors.push(format!(
                    "copy source for `{}` does not exist: {}",
                    entry.title,
                    source_path.display()
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn unexpected_managed_assets(
    repo_root: &Path,
    categories: &[Category],
    entries: &[GalleryEntry],
) -> Result<Vec<PathBuf>, String> {
    let expected = expected_assets_by_category(entries);
    let mut unexpected = Vec::new();

    for category in categories {
        let asset_dir = repo_root.join(GALLERY_ASSETS_ROOT).join(category.slug);
        if !asset_dir.exists() {
            continue;
        }
        let keep = expected.get(category.slug).cloned().unwrap_or_default();
        for item in fs::read_dir(&asset_dir)
            .map_err(|err| format!("failed to read {}: {}", asset_dir.display(), err))?
        {
            let item =
                item.map_err(|err| format!("failed to inspect {}: {}", asset_dir.display(), err))?;
            let path = item.path();
            let extension = path.extension().and_then(|value| value.to_str());
            if extension != Some("png") && extension != Some("gif") {
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !keep.contains(name) {
                unexpected.push(path);
            }
        }
    }

    unexpected.sort();
    Ok(unexpected)
}

fn run_examples(entries: &[GalleryEntry]) -> Result<(), String> {
    let mut runs = BTreeSet::new();
    for entry in entries {
        if let AssetSource::Example { run, .. } = entry.source {
            runs.insert(run);
        }
    }

    for run in runs {
        println!("Running example: {}", run.name);
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "--example", run.name, "--release"]);
        if let Some(features) = run.features {
            cmd.args(["--features", features]);
        }

        let output = cmd
            .output()
            .map_err(|err| format!("failed to run example `{}`: {}", run.name, err))?;
        if !output.status.success() {
            return Err(format!(
                "example `{}` failed\nstdout:\n{}\nstderr:\n{}",
                run.name,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    Ok(())
}

fn sync_assets(entries: &[GalleryEntry]) -> Result<(), String> {
    for entry in entries {
        let source = match entry.source {
            AssetSource::Example { output_rel, .. } => repo_path(output_rel),
            AssetSource::Copy { source_rel } => repo_path(source_rel),
        };

        if !source.exists() {
            return Err(format!(
                "missing source asset for `{}`: {}",
                entry.title,
                source.display()
            ));
        }

        let dest = repo_path(GALLERY_ASSETS_ROOT)
            .join(entry.category)
            .join(entry.asset_name);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create gallery asset directory {}: {}",
                    parent.display(),
                    err
                )
            })?;
        }
        fs::copy(&source, &dest).map_err(|err| {
            format!(
                "failed to copy gallery asset {} -> {}: {}",
                source.display(),
                dest.display(),
                err
            )
        })?;
    }

    Ok(())
}

fn sync_copy_assets(entries: &[GalleryEntry]) -> Result<(), String> {
    for entry in entries {
        let AssetSource::Copy { source_rel } = entry.source else {
            continue;
        };
        let source = repo_path(source_rel);
        let dest = gallery_asset_path(Path::new(env!("CARGO_MANIFEST_DIR")), entry);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create gallery asset directory {}: {}",
                    parent.display(),
                    err
                )
            })?;
        }
        fs::copy(&source, &dest).map_err(|err| {
            format!(
                "failed to copy gallery asset {} -> {}: {}",
                source.display(),
                dest.display(),
                err
            )
        })?;
    }
    Ok(())
}

fn prune_stale_assets(categories: &[Category], entries: &[GalleryEntry]) -> Result<(), String> {
    let expected = expected_assets_by_category(entries);

    for category in categories {
        let asset_dir = repo_path(GALLERY_ASSETS_ROOT).join(category.slug);
        let keep = expected
            .get(category.slug)
            .cloned()
            .unwrap_or_else(BTreeSet::new);
        if !asset_dir.exists() {
            continue;
        }

        for item in fs::read_dir(&asset_dir)
            .map_err(|err| format!("failed to read {}: {}", asset_dir.display(), err))?
        {
            let item =
                item.map_err(|err| format!("failed to inspect {}: {}", asset_dir.display(), err))?;
            let path = item.path();
            let extension = path.extension().and_then(|value| value.to_str());
            if extension != Some("png") && extension != Some("gif") {
                continue;
            }

            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !keep.contains(name) {
                fs::remove_file(&path).map_err(|err| {
                    format!("failed to remove stale asset {}: {}", path.display(), err)
                })?;
            }
        }
    }

    Ok(())
}

pub fn render_category_page(category: &Category, entries: &[&GalleryEntry]) -> String {
    let mut content = String::new();
    content.push_str(&format!("# {}\n\n", category.title));
    content.push_str(category.description);
    content.push_str("\n\n## Examples\n\n");

    for entry in entries {
        content.push_str(&format!("### {}\n\n", entry.title));
        content.push_str(entry.summary);
        content.push_str("\n\n");
        content.push_str(&format!(
            "![{}](../../assets/gallery/rust/{}/{})\n\n",
            entry.title, entry.category, entry.asset_name
        ));
        content.push_str(&format!(
            "Source: [{}](../../../{})\n\n",
            entry.source_path, entry.source_path
        ));
        if let Some((label, path)) = entry.guide {
            content.push_str(&format!("Guide: [{label}]({path})\n\n"));
        }
    }

    content.push_str("[← Back to Gallery](../README.md)\n");
    content
}

pub fn entries_for_category<'a>(
    entries: &'a [GalleryEntry],
    category: &str,
) -> Vec<&'a GalleryEntry> {
    entries
        .iter()
        .filter(|entry| entry.category == category)
        .collect()
}

fn write_category_pages(categories: &[Category], entries: &[GalleryEntry]) -> Result<(), String> {
    for category in categories {
        let category_entries = entries_for_category(entries, category.slug);
        let content = render_category_page(category, &category_entries);

        let readme_path = repo_path(GALLERY_DOCS_ROOT)
            .join(category.slug)
            .join("README.md");
        fs::write(&readme_path, content).map_err(|err| {
            format!(
                "failed to write gallery page {}: {}",
                readme_path.display(),
                err
            )
        })?;
    }

    Ok(())
}

pub fn render_gallery_index(categories: &[Category], entries: &[GalleryEntry]) -> String {
    let mut counts = BTreeMap::new();
    for entry in entries {
        *counts.entry(entry.category).or_insert(0usize) += 1;
    }

    let mut content = String::from("# ruviz Gallery\n\n");
    content.push_str(
        "Curated visual showcase of the Rust examples and rustdoc media for `ruviz`.\n\n",
    );
    content.push_str(&format!("**Total Examples**: {}\n\n", entries.len()));
    content.push_str("## Gallery Categories\n\n");

    for category in categories {
        let count = counts.get(category.slug).copied().unwrap_or(0);
        content.push_str(&format!(
            "### {} {} ({} examples)\n\n{}\n\n[View {} →]({}/README.md)\n\n",
            category.icon,
            category.title,
            count,
            category.description,
            category.title,
            category.slug
        ));
    }

    content.push_str("---\n\n");
    content.push_str(
        "Gallery assets are generated from `generated/examples/` and `docs/assets/rustdoc/`.\n",
    );
    content
        .push_str("Refresh them in source-first order with:\n\n```bash\nmake rust-gallery\n```\n");

    content
}

fn write_gallery_index(categories: &[Category], entries: &[GalleryEntry]) -> Result<(), String> {
    let content = render_gallery_index(categories, entries);

    let index_path = repo_path(GALLERY_DOCS_ROOT).join("README.md");
    fs::write(&index_path, content).map_err(|err| {
        format!(
            "failed to write gallery index {}: {}",
            index_path.display(),
            err
        )
    })?;
    Ok(())
}

fn ensure_gallery_layout(categories: &[Category]) -> Result<(), String> {
    for category in categories {
        fs::create_dir_all(repo_path(GALLERY_DOCS_ROOT).join(category.slug)).map_err(|err| {
            format!(
                "failed to create gallery docs directory for {}: {}",
                category.slug, err
            )
        })?;
        fs::create_dir_all(repo_path(GALLERY_ASSETS_ROOT).join(category.slug)).map_err(|err| {
            format!(
                "failed to create gallery asset directory for {}: {}",
                category.slug, err
            )
        })?;
    }

    Ok(())
}

fn stage_copy_backed_preview(entries: &[GalleryEntry]) -> Result<(), String> {
    let subplot = entries
        .iter()
        .find(|entry| entry.asset_name == "subplots.png")
        .ok_or_else(|| "catalog is missing the canonical subplot entry".to_string())?;
    let source = copy_source_path(Path::new(env!("CARGO_MANIFEST_DIR")), subplot)
        .ok_or_else(|| "canonical subplot entry must be Copy-backed".to_string())?;
    let destination = repo_path("generated/examples/copy-backed/subplots.png");
    let parent = destination
        .parent()
        .ok_or_else(|| "preview subplot destination has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|err| {
        format!(
            "failed to create preview directory {}: {}",
            parent.display(),
            err
        )
    })?;
    fs::copy(&source, &destination).map_err(|err| {
        format!(
            "failed to stage Copy-backed preview {} -> {}: {}",
            source.display(),
            destination.display(),
            err
        )
    })?;
    Ok(())
}

pub fn check_gallery(
    repo_root: &Path,
    categories: &[Category],
    entries: &[GalleryEntry],
) -> Vec<String> {
    let mut errors = validate_catalog(repo_root, categories, entries)
        .err()
        .unwrap_or_default();

    for category in categories {
        let expected =
            render_category_page(category, &entries_for_category(entries, category.slug));
        let path = repo_root
            .join(GALLERY_DOCS_ROOT)
            .join(category.slug)
            .join("README.md");
        match fs::read_to_string(&path) {
            Ok(actual) if actual == expected => {}
            Ok(_) => errors.push(format!(
                "generated Markdown is stale: {} (run `make rust-gallery`)",
                path.display()
            )),
            Err(err) => errors.push(format!("failed to read {}: {}", path.display(), err)),
        }
    }

    let index_path = repo_root.join(GALLERY_DOCS_ROOT).join("README.md");
    match fs::read_to_string(&index_path) {
        Ok(actual) if actual == render_gallery_index(categories, entries) => {}
        Ok(_) => errors.push(format!(
            "generated Markdown is stale: {} (run `make rust-gallery`)",
            index_path.display()
        )),
        Err(err) => errors.push(format!("failed to read {}: {}", index_path.display(), err)),
    }

    for entry in entries {
        let destination = gallery_asset_path(repo_root, entry);
        match copy_source_path(repo_root, entry) {
            Some(source) => match (fs::read(&source), fs::read(&destination)) {
                (Ok(source_bytes), Ok(destination_bytes)) if source_bytes == destination_bytes => {}
                (Ok(_), Ok(_)) => errors.push(format!(
                    "Copy-backed gallery asset is stale: {} differs from {}",
                    destination.display(),
                    source.display()
                )),
                (Err(err), _) => {
                    errors.push(format!("failed to read {}: {}", source.display(), err))
                }
                (_, Err(err)) => errors.push(format!(
                    "failed to read gallery asset {}: {}",
                    destination.display(),
                    err
                )),
            },
            None if !destination.is_file() => errors.push(format!(
                "Example-backed gallery asset is missing: {}",
                destination.display()
            )),
            None => {}
        }
    }

    match unexpected_managed_assets(repo_root, categories, entries) {
        Ok(paths) => errors.extend(
            paths
                .into_iter()
                .map(|path| format!("unexpected managed gallery asset: {}", path.display())),
        ),
        Err(err) => errors.push(err),
    }

    errors
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Mode {
    Full,
    PreviewOnly,
    Check,
    DeterministicOnly,
}

fn parse_mode() -> Result<Mode, String> {
    let mut mode = Mode::Full;
    let mut saw_positional = false;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--preview-only" => mode = Mode::PreviewOnly,
            "--check" => mode = Mode::Check,
            "--deterministic-only" => mode = Mode::DeterministicOnly,
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --bin generate_gallery -- [--preview-only|--check|--deterministic-only]"
                );
                println!("  --check               verify committed gallery files without writing");
                println!("  --preview-only        refresh generated preview outputs only");
                println!(
                    "  --deterministic-only  refresh Markdown and Copy-backed assets without examples"
                );
                std::process::exit(0);
            }
            _ if arg.starts_with('-') => {
                return Err(format!("unsupported flag `{arg}`"));
            }
            _ => {
                saw_positional = true;
                break;
            }
        }
    }

    if saw_positional {
        return Err("generate_gallery does not accept positional arguments".to_string());
    }

    Ok(mode)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = parse_mode()?;
    let categories = categories();
    let entries = entries();

    if mode == Mode::Check {
        let errors = check_gallery(Path::new(env!("CARGO_MANIFEST_DIR")), &categories, &entries);
        if errors.is_empty() {
            println!("Rust gallery freshness check passed");
            return Ok(());
        }
        for error in &errors {
            eprintln!("gallery freshness error: {error}");
        }
        return Err(format!(
            "Rust gallery freshness check failed with {} error(s)",
            errors.len()
        )
        .into());
    }

    if mode == Mode::DeterministicOnly {
        ensure_gallery_layout(&categories)?;
        sync_copy_assets(&entries)?;
        prune_stale_assets(&categories, &entries)?;
        write_category_pages(&categories, &entries)?;
        write_gallery_index(&categories, &entries)?;
        println!("Deterministic gallery outputs refreshed");
        return Ok(());
    }

    run_examples(&entries)?;
    if mode == Mode::PreviewOnly {
        stage_copy_backed_preview(&entries)?;
        println!("Preview example assets refreshed under generated/examples");
        return Ok(());
    }

    ensure_gallery_layout(&categories)?;
    sync_assets(&entries)?;
    prune_stale_assets(&categories, &entries)?;
    write_category_pages(&categories, &entries)?;
    write_gallery_index(&categories, &entries)?;

    println!("Gallery assets refreshed under {GALLERY_ASSETS_ROOT}");
    println!("Gallery markdown refreshed under {GALLERY_DOCS_ROOT}");
    Ok(())
}
