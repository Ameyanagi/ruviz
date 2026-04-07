#!/usr/bin/env rust-script
//! Generate curated Rust gallery assets and markdown indexes.
//!
//! Usage: cargo run --bin generate_gallery

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const GALLERY_DOCS_ROOT: &str = "docs/gallery";
const GALLERY_ASSETS_ROOT: &str = "docs/assets/gallery/rust";

#[derive(Clone, Copy)]
struct Category {
    slug: &'static str,
    title: &'static str,
    description: &'static str,
    icon: &'static str,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct ExampleRun {
    name: &'static str,
    features: Option<&'static str>,
}

#[derive(Clone, Copy)]
enum AssetSource {
    Example {
        run: ExampleRun,
        output_rel: &'static str,
    },
    Copy {
        source_rel: &'static str,
    },
}

#[derive(Clone, Copy)]
struct GalleryEntry {
    category: &'static str,
    title: &'static str,
    summary: &'static str,
    asset_name: &'static str,
    source_label: &'static str,
    source: AssetSource,
}

fn categories() -> Vec<Category> {
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

fn entries() -> Vec<GalleryEntry> {
    vec![
        GalleryEntry {
            category: "basic",
            title: "Line Plot",
            summary: "The core line example used across the README and rustdoc.",
            asset_name: "line_plot.png",
            source_label: "`examples/doc_line_plot.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/line_plot.png",
            },
        },
        GalleryEntry {
            category: "basic",
            title: "Histogram Example",
            summary: "Standalone histogram example rendered from the example suite.",
            asset_name: "histogram_example.png",
            source_label: "`examples/histogram_example.rs`",
            source: AssetSource::Example {
                run: ExampleRun {
                    name: "histogram_example",
                    features: None,
                },
                output_rel: "generated/examples/histogram_example.png",
            },
        },
        GalleryEntry {
            category: "basic",
            title: "Boxplot Example",
            summary: "Standalone boxplot example rendered from the example suite.",
            asset_name: "boxplot_example.png",
            source_label: "`examples/boxplot_example.rs`",
            source: AssetSource::Example {
                run: ExampleRun {
                    name: "boxplot_example",
                    features: None,
                },
                output_rel: "generated/examples/boxplot_example.png",
            },
        },
        GalleryEntry {
            category: "statistical",
            title: "Kernel Density Estimate",
            summary: "KDE example copied from the rustdoc image set.",
            asset_name: "kde_plot.png",
            source_label: "`examples/doc_kde.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/kde_plot.png",
            },
        },
        GalleryEntry {
            category: "statistical",
            title: "ECDF",
            summary: "Empirical CDF example copied from the rustdoc image set.",
            asset_name: "ecdf_plot.png",
            source_label: "`examples/doc_ecdf.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/ecdf_plot.png",
            },
        },
        GalleryEntry {
            category: "statistical",
            title: "Violin Plot",
            summary: "Distribution plot with quartile-aware styling.",
            asset_name: "violin_plot.png",
            source_label: "`examples/doc_violin.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/violin_plot.png",
            },
        },
        GalleryEntry {
            category: "statistical",
            title: "Seaborn-Style Boxplot",
            summary: "A style-heavy statistical example generated from the example suite.",
            asset_name: "seaborn_boxplot_example.png",
            source_label: "`examples/seaborn_style_example.rs`",
            source: AssetSource::Example {
                run: ExampleRun {
                    name: "seaborn_style_example",
                    features: None,
                },
                output_rel: "generated/examples/seaborn_boxplot_example.png",
            },
        },
        GalleryEntry {
            category: "publication",
            title: "Scientific Analysis Figure",
            summary: "Multi-panel figure assembled for report-style presentation.",
            asset_name: "scientific_analysis_figure.png",
            source_label: "`examples/scientific_showcase.rs`",
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
            source_label: "`examples/doc_themes.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/theme_publication.png",
            },
        },
        GalleryEntry {
            category: "publication",
            title: "Subplot Layout",
            summary: "A multi-panel subplot layout used for publication-scale figures.",
            asset_name: "subplots.png",
            source_label: "`examples/doc_subplots.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/subplots.png",
            },
        },
        GalleryEntry {
            category: "performance",
            title: "Memory Optimization Demo",
            summary: "Performance-oriented chart generated by the memory optimization example.",
            asset_name: "memory_optimization_demo.png",
            source_label: "`examples/memory_optimization_demo.rs`",
            source: AssetSource::Example {
                run: ExampleRun {
                    name: "memory_optimization_demo",
                    features: None,
                },
                output_rel: "generated/examples/memory_optimization_demo.png",
            },
        },
        GalleryEntry {
            category: "performance",
            title: "Parallel Demo 100k",
            summary: "Parallel rendering example targeting a large 100k-point dataset.",
            asset_name: "parallel_demo_100k.png",
            source_label: "`examples/parallel_demo.rs`",
            source: AssetSource::Example {
                run: ExampleRun {
                    name: "parallel_demo",
                    features: None,
                },
                output_rel: "generated/examples/parallel_demo_100k.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Contour Plot",
            summary: "Contour rendering example with level interpolation.",
            asset_name: "contour_plot.png",
            source_label: "`examples/doc_contour.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/contour_plot.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Radar Chart",
            summary: "Radar chart example demonstrating non-cartesian layout support.",
            asset_name: "radar_chart.png",
            source_label: "`examples/doc_radar.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/radar_chart.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Legend Positions",
            summary: "Reference image covering legend placement options.",
            asset_name: "legend_positions.png",
            source_label: "`examples/doc_legend_positions.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/legend_positions.png",
            },
        },
        GalleryEntry {
            category: "advanced",
            title: "Seaborn-Style Histogram",
            summary: "A styling-heavy histogram variant from the Seaborn example set.",
            asset_name: "seaborn_histogram_example.png",
            source_label: "`examples/seaborn_style_example.rs`",
            source: AssetSource::Example {
                run: ExampleRun {
                    name: "seaborn_style_example",
                    features: None,
                },
                output_rel: "generated/examples/seaborn_histogram_example.png",
            },
        },
        GalleryEntry {
            category: "animation",
            title: "Traveling Sine Wave",
            summary: "Animated sine wave generated with the `record!` macro.",
            asset_name: "animation_sine_wave.gif",
            source_label: "`examples/generate_animation_gallery.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/animation_sine_wave.gif",
            },
        },
        GalleryEntry {
            category: "animation",
            title: "Animated Bars",
            summary: "Animated categorical data example rendered as a GIF.",
            asset_name: "animation_bars.gif",
            source_label: "`examples/generate_animation_gallery.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/animation_bars.gif",
            },
        },
        GalleryEntry {
            category: "animation",
            title: "Wave Interference",
            summary: "Animated wave interference example rendered as a GIF.",
            asset_name: "animation_interference.gif",
            source_label: "`examples/generate_animation_gallery.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/animation_interference.gif",
            },
        },
        GalleryEntry {
            category: "internationalization",
            title: "Japanese Labels",
            summary: "Japanese-language labels rendered with the default browser/document fonts.",
            asset_name: "international_japanese.png",
            source_label: "`examples/doc_international.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/international_japanese.png",
            },
        },
        GalleryEntry {
            category: "internationalization",
            title: "Chinese Labels",
            summary: "Chinese-language bar chart rendering example.",
            asset_name: "international_chinese.png",
            source_label: "`examples/doc_international.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/international_chinese.png",
            },
        },
        GalleryEntry {
            category: "internationalization",
            title: "Korean Labels",
            summary: "Korean-language line chart rendering example.",
            asset_name: "international_korean.png",
            source_label: "`examples/doc_international.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/international_korean.png",
            },
        },
        GalleryEntry {
            category: "internationalization",
            title: "Multi-Language Comparison",
            summary: "A four-panel comparison of CJK and mixed-language labels.",
            asset_name: "international_comparison.png",
            source_label: "`examples/doc_international.rs`",
            source: AssetSource::Copy {
                source_rel: "docs/assets/rustdoc/international_comparison.png",
            },
        },
    ]
}

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
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

fn write_category_pages(categories: &[Category], entries: &[GalleryEntry]) -> Result<(), String> {
    let mut by_category: BTreeMap<&str, Vec<&GalleryEntry>> = BTreeMap::new();
    for entry in entries {
        by_category.entry(entry.category).or_default().push(entry);
    }

    for category in categories {
        let category_entries = by_category.get(category.slug).cloned().unwrap_or_default();
        let mut content = String::new();
        content.push_str(&format!("# {}\n\n", category.title));
        content.push_str(category.description);
        content.push_str("\n\n## Examples\n\n");

        for entry in category_entries {
            content.push_str(&format!("### {}\n\n", entry.title));
            content.push_str(entry.summary);
            content.push_str("\n\n");
            content.push_str(&format!(
                "![{}](../../assets/gallery/rust/{}/{})\n\n",
                entry.title, entry.category, entry.asset_name
            ));
            content.push_str(&format!("Source: {}\n\n", entry.source_label));
        }

        content.push_str("[← Back to Gallery](../README.md)\n");

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

fn write_gallery_index(categories: &[Category], entries: &[GalleryEntry]) -> Result<(), String> {
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
    content.push_str("Refresh them with:\n\n```bash\ncargo run --bin generate_gallery\n```\n");

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

#[derive(Clone, Copy, Eq, PartialEq)]
enum Mode {
    Full,
    PreviewOnly,
}

fn parse_mode() -> Result<Mode, String> {
    let mut mode = Mode::Full;
    let mut saw_positional = false;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--preview-only" => mode = Mode::PreviewOnly,
            "--help" | "-h" => {
                println!("Usage: cargo run --bin generate_gallery -- [--preview-only]");
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

    run_examples(&entries)?;
    if mode == Mode::PreviewOnly {
        println!("Preview example assets refreshed under generated/examples");
        return Ok(());
    }

    ensure_gallery_layout(&categories)?;
    sync_assets(&entries)?;
    write_category_pages(&categories, &entries)?;
    write_gallery_index(&categories, &entries)?;

    println!("Gallery assets refreshed under {GALLERY_ASSETS_ROOT}");
    println!("Gallery markdown refreshed under {GALLERY_DOCS_ROOT}");
    Ok(())
}
