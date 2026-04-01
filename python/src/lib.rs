use std::path::Path;

use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::PyBytes,
};
use ruviz::{
    core::{IntoPlot, Plot},
    interactive::show_interactive,
    render::Theme,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlotSnapshot {
    size_px: Option<[u32; 2]>,
    theme: Option<String>,
    ticks: Option<bool>,
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    series: Vec<SeriesSnapshot>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SineSignalOptions {
    points: usize,
    domain_start: f64,
    domain_end: f64,
    amplitude: f64,
    cycles: f64,
    phase_velocity: f64,
    phase_offset: f64,
    vertical_offset: f64,
}

impl SineSignalOptions {
    fn values_at(&self, time_seconds: f64) -> Vec<f64> {
        let len = self.points.max(2);
        let span = self.domain_end - self.domain_start;
        let phase = self.phase_offset + self.phase_velocity * time_seconds;
        let denom = (len - 1) as f64;

        (0..len)
            .map(|index| {
                let progress = index as f64 / denom;
                let x = self.domain_start + span * progress;
                self.vertical_offset + self.amplitude * (self.cycles * x + phase).sin()
            })
            .collect()
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum NumericSourceSnapshot {
    Static { values: Vec<f64> },
    Observable { values: Vec<f64> },
}

impl NumericSourceSnapshot {
    fn into_values(self) -> Vec<f64> {
        match self {
            Self::Static { values } | Self::Observable { values } => values,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum YSourceSnapshot {
    Static { values: Vec<f64> },
    Observable { values: Vec<f64> },
    SineSignal { options: SineSignalOptions },
}

impl YSourceSnapshot {
    fn into_values(self) -> Vec<f64> {
        match self {
            Self::Static { values } | Self::Observable { values } => values,
            Self::SineSignal { options } => options.values_at(0.0),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RadarSeriesItem {
    name: Option<String>,
    values: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum SeriesSnapshot {
    Line {
        x: NumericSourceSnapshot,
        y: YSourceSnapshot,
    },
    Scatter {
        x: NumericSourceSnapshot,
        y: YSourceSnapshot,
    },
    Bar {
        categories: Vec<String>,
        values: NumericSourceSnapshot,
    },
    Histogram {
        data: NumericSourceSnapshot,
    },
    Boxplot {
        data: NumericSourceSnapshot,
    },
    Heatmap {
        values: Vec<f64>,
        rows: usize,
        cols: usize,
    },
    ErrorBars {
        x: NumericSourceSnapshot,
        y: NumericSourceSnapshot,
        #[serde(rename = "yErrors")]
        y_errors: NumericSourceSnapshot,
    },
    ErrorBarsXy {
        x: NumericSourceSnapshot,
        y: NumericSourceSnapshot,
        #[serde(rename = "xErrors")]
        x_errors: NumericSourceSnapshot,
        #[serde(rename = "yErrors")]
        y_errors: NumericSourceSnapshot,
    },
    Kde {
        data: Vec<f64>,
    },
    Ecdf {
        data: Vec<f64>,
    },
    Contour {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
    },
    Pie {
        values: Vec<f64>,
        labels: Option<Vec<String>>,
    },
    Radar {
        labels: Vec<String>,
        series: Vec<RadarSeriesItem>,
    },
    Violin {
        data: Vec<f64>,
    },
    PolarLine {
        r: Vec<f64>,
        theta: Vec<f64>,
    },
}

impl PlotSnapshot {
    fn into_plot(self) -> Result<Plot, String> {
        let mut plot = Plot::new();

        if let Some([width, height]) = self.size_px {
            plot = plot.size_px(width, height);
        }

        if let Some(theme) = self.theme {
            plot = match theme.as_str() {
                "light" => plot.theme(Theme::light()),
                "dark" => plot.theme(Theme::dark()),
                other => return Err(format!("unsupported theme: {other}")),
            };
        }

        if let Some(ticks) = self.ticks {
            plot = plot.ticks(ticks);
        }

        if let Some(title) = self.title {
            plot = plot.title(title);
        }

        if let Some(x_label) = self.x_label {
            plot = plot.xlabel(x_label);
        }

        if let Some(y_label) = self.y_label {
            plot = plot.ylabel(y_label);
        }

        for series in self.series {
            plot = apply_series(plot, series)?;
        }

        Ok(plot)
    }
}

fn ensure_same_len(lengths: &[usize], message: &str) -> Result<(), String> {
    let Some((&first, rest)) = lengths.split_first() else {
        return Ok(());
    };

    if rest.iter().all(|len| *len == first) {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

fn apply_series(plot: Plot, series: SeriesSnapshot) -> Result<Plot, String> {
    match series {
        SeriesSnapshot::Line { x, y } => {
            let x = x.into_values();
            let y = y.into_values();
            ensure_same_len(&[x.len(), y.len()], "line x and y must have the same length")?;
            Ok(plot.line(&x, &y).into_plot())
        }
        SeriesSnapshot::Scatter { x, y } => {
            let x = x.into_values();
            let y = y.into_values();
            ensure_same_len(&[x.len(), y.len()], "scatter x and y must have the same length")?;
            Ok(plot.scatter(&x, &y).into_plot())
        }
        SeriesSnapshot::Bar { categories, values } => {
            let values = values.into_values();
            ensure_same_len(
                &[categories.len(), values.len()],
                "bar categories and values must have the same length",
            )?;
            Ok(plot.bar(&categories, &values).into_plot())
        }
        SeriesSnapshot::Histogram { data } => Ok(plot.histogram(&data.into_values(), None).into_plot()),
        SeriesSnapshot::Boxplot { data } => Ok(plot.boxplot(&data.into_values(), None).into_plot()),
        SeriesSnapshot::Heatmap { values, rows, cols } => {
            if rows == 0 || cols == 0 || values.len() != rows.saturating_mul(cols) {
                return Err("heatmap values length must match rows * cols".to_string());
            }
            let matrix: Vec<Vec<f64>> = values.chunks(cols).map(|chunk| chunk.to_vec()).collect();
            Ok(plot.heatmap(&matrix, None).into_plot())
        }
        SeriesSnapshot::ErrorBars { x, y, y_errors } => {
            let x = x.into_values();
            let y = y.into_values();
            let y_errors = y_errors.into_values();
            ensure_same_len(
                &[x.len(), y.len(), y_errors.len()],
                "error bar x, y, and y_errors must have the same length",
            )?;
            Ok(plot.error_bars(&x, &y, &y_errors).into_plot())
        }
        SeriesSnapshot::ErrorBarsXy {
            x,
            y,
            x_errors,
            y_errors,
        } => {
            let x = x.into_values();
            let y = y.into_values();
            let x_errors = x_errors.into_values();
            let y_errors = y_errors.into_values();
            ensure_same_len(
                &[x.len(), y.len(), x_errors.len(), y_errors.len()],
                "error bar x, y, x_errors, and y_errors must have the same length",
            )?;
            Ok(plot.error_bars_xy(&x, &y, &x_errors, &y_errors).into_plot())
        }
        SeriesSnapshot::Kde { data } => Ok(plot.kde(&data).into_plot()),
        SeriesSnapshot::Ecdf { data } => Ok(plot.ecdf(&data).into_plot()),
        SeriesSnapshot::Contour { x, y, z } => {
            if x.is_empty() || y.is_empty() || z.len() != x.len() * y.len() {
                return Err("contour z must contain x.len() * y.len() values".to_string());
            }
            Ok(plot.contour(&x, &y, &z).into_plot())
        }
        SeriesSnapshot::Pie { values, labels } => {
            let builder = plot.pie(&values);
            if let Some(labels) = labels {
                ensure_same_len(
                    &[values.len(), labels.len()],
                    "pie values and labels must have the same length",
                )?;
                Ok(builder.labels(&labels).into_plot())
            } else {
                Ok(builder.into_plot())
            }
        }
        SeriesSnapshot::Radar { labels, series } => {
            if labels.is_empty() {
                return Err("radar labels must not be empty".to_string());
            }

            let mut builder = plot.radar(&labels);
            for item in series {
                ensure_same_len(
                    &[labels.len(), item.values.len()],
                    "radar series values must match labels length",
                )?;
                builder = if let Some(name) = item.name {
                    builder.add_series(name, &item.values)
                } else {
                    builder.series(&item.values)
                };
            }
            Ok(builder.into_plot())
        }
        SeriesSnapshot::Violin { data } => Ok(plot.violin(&data).into_plot()),
        SeriesSnapshot::PolarLine { r, theta } => {
            ensure_same_len(&[r.len(), theta.len()], "polar r and theta must have the same length")?;
            Ok(plot.polar_line(&r, &theta).into_plot())
        }
    }
}

fn parse_plot(snapshot_json: &str) -> PyResult<Plot> {
    let snapshot: PlotSnapshot =
        serde_json::from_str(snapshot_json).map_err(|err| PyValueError::new_err(err.to_string()))?;

    snapshot
        .into_plot()
        .map_err(PyValueError::new_err)
}

#[pyfunction]
fn render_png_bytes<'py>(py: Python<'py>, snapshot_json: &str) -> PyResult<Bound<'py, PyBytes>> {
    let plot = parse_plot(snapshot_json)?;
    let bytes = plot
        .render_png_bytes()
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
    Ok(PyBytes::new(py, &bytes))
}

#[pyfunction]
fn render_svg(snapshot_json: &str) -> PyResult<String> {
    let plot = parse_plot(snapshot_json)?;
    plot.render_to_svg()
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))
}

#[pyfunction]
fn save(snapshot_json: &str, path: &str) -> PyResult<()> {
    let plot = parse_plot(snapshot_json)?;
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "png".to_string());

    match extension.as_str() {
        "svg" => plot
            .export_svg(path)
            .map_err(|err| PyRuntimeError::new_err(err.to_string())),
        "pdf" => plot
            .save_pdf(path)
            .map_err(|err| PyRuntimeError::new_err(err.to_string())),
        _ => plot
            .save(path)
            .map_err(|err| PyRuntimeError::new_err(err.to_string())),
    }
}

#[pyfunction]
fn show_native(snapshot_json: &str) -> PyResult<()> {
    let plot = parse_plot(snapshot_json)?;
    pollster::block_on(show_interactive(plot))
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))
}

#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(render_png_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(render_svg, module)?)?;
    module.add_function(wrap_pyfunction!(save, module)?)?;
    module.add_function(wrap_pyfunction!(show_native, module)?)?;
    module.add_function(wrap_pyfunction!(version, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use image::RgbaImage;
    use serde_json::json;

    fn decode_png(bytes: &[u8]) -> RgbaImage {
        image::load_from_memory(bytes)
            .expect("expected valid PNG bytes")
            .to_rgba8()
    }

    fn assert_pngs_equal(name: &str, actual: &[u8], expected: &[u8]) {
        let actual = decode_png(actual);
        let expected = decode_png(expected);

        assert_eq!(
            actual.dimensions(),
            expected.dimensions(),
            "{name}: PNG dimensions differ"
        );

        let mut diff_pixels = 0usize;
        for (left, right) in actual.pixels().zip(expected.pixels()) {
            if left != right {
                diff_pixels += 1;
            }
        }

        assert_eq!(
            diff_pixels, 0,
            "{name}: rendered PNG differs at {diff_pixels} pixel(s)"
        );
    }

    fn render_snapshot_png(snapshot: serde_json::Value) -> Vec<u8> {
        parse_plot(&snapshot.to_string())
            .expect("snapshot should parse")
            .render_png_bytes()
            .expect("snapshot render should succeed")
    }

    fn render_direct_png(plot: Plot) -> Vec<u8> {
        plot.render_png_bytes()
            .expect("direct render should succeed")
    }

    fn base_plot(title: &str) -> Plot {
        Plot::new()
            .size_px(320, 200)
            .theme(Theme::light())
            .ticks(true)
            .title(title)
            .xlabel("x")
            .ylabel("y")
    }

    #[test]
    fn snapshot_render_matches_direct_plot_for_xy_series() {
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![0.2, 1.1, 0.7, 1.8];
        let scatter_y = vec![0.3, 1.0, 0.9, 1.6];

        let snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "XY parity",
            "xLabel": "x",
            "yLabel": "y",
            "series": [
                {
                    "kind": "line",
                    "x": {"kind": "static", "values": x},
                    "y": {"kind": "static", "values": y},
                },
                {
                    "kind": "scatter",
                    "x": {"kind": "static", "values": [0.0, 1.0, 2.0, 3.0]},
                    "y": {"kind": "static", "values": scatter_y},
                }
            ]
        });

        let expected = base_plot("XY parity")
            .line(&[0.0, 1.0, 2.0, 3.0], &[0.2, 1.1, 0.7, 1.8])
            .scatter(&[0.0, 1.0, 2.0, 3.0], &[0.3, 1.0, 0.9, 1.6])
            .into_plot();

        let actual_png = render_snapshot_png(snapshot);
        let expected_png = render_direct_png(expected);
        assert_pngs_equal("xy_series", &actual_png, &expected_png);
    }

    #[test]
    fn snapshot_render_matches_direct_plot_for_statistical_series() {
        let samples = vec![
            -2.3, -1.9, -1.1, -0.4, 0.2, 0.8, 1.0, 1.4, 1.7, 2.1, 2.5, 2.9,
        ];

        let snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "Histogram parity",
            "xLabel": "value",
            "yLabel": "count",
            "series": [
                {
                    "kind": "histogram",
                    "data": {"kind": "static", "values": samples},
                }
            ]
        });

        let expected = base_plot("Histogram parity")
            .xlabel("value")
            .ylabel("count")
            .histogram(&[-2.3, -1.9, -1.1, -0.4, 0.2, 0.8, 1.0, 1.4, 1.7, 2.1, 2.5, 2.9], None)
            .into_plot();

        let actual_png = render_snapshot_png(snapshot);
        let expected_png = render_direct_png(expected);
        assert_pngs_equal("histogram", &actual_png, &expected_png);
    }

    #[test]
    fn snapshot_render_matches_direct_plot_for_matrix_and_errorbar_series() {
        let heatmap_values = vec![
            vec![0.1, 0.4, 0.8],
            vec![0.3, 0.5, 0.7],
            vec![0.2, 0.6, 0.9],
        ];
        let flat_heatmap = vec![0.1, 0.4, 0.8, 0.3, 0.5, 0.7, 0.2, 0.6, 0.9];

        let heatmap_snapshot = json!({
            "sizePx": [320, 200],
            "theme": "dark",
            "ticks": false,
            "title": "Heatmap parity",
            "series": [
                {
                    "kind": "heatmap",
                    "values": flat_heatmap,
                    "rows": 3,
                    "cols": 3,
                }
            ]
        });

        let heatmap_expected = Plot::new()
            .size_px(320, 200)
            .theme(Theme::dark())
            .ticks(false)
            .title("Heatmap parity")
            .heatmap(&heatmap_values, None)
            .into_plot();

        let actual_png = render_snapshot_png(heatmap_snapshot);
        let expected_png = render_direct_png(heatmap_expected);
        assert_pngs_equal("heatmap", &actual_png, &expected_png);

        let error_snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "Error parity",
            "xLabel": "x",
            "yLabel": "y",
            "series": [
                {
                    "kind": "error-bars-xy",
                    "x": {"kind": "static", "values": [1.0, 2.0, 3.0]},
                    "y": {"kind": "static", "values": [1.2, 1.8, 1.4]},
                    "xErrors": {"kind": "static", "values": [0.1, 0.15, 0.12]},
                    "yErrors": {"kind": "static", "values": [0.2, 0.18, 0.16]},
                }
            ]
        });

        let error_expected = base_plot("Error parity")
            .error_bars_xy(
                &[1.0, 2.0, 3.0],
                &[1.2, 1.8, 1.4],
                &[0.1, 0.15, 0.12],
                &[0.2, 0.18, 0.16],
            )
            .into_plot();

        let actual_png = render_snapshot_png(error_snapshot);
        let expected_png = render_direct_png(error_expected);
        assert_pngs_equal("error_bars_xy", &actual_png, &expected_png);
    }

    #[test]
    fn snapshot_render_matches_direct_plot_for_specialized_series() {
        let contour_snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "Contour parity",
            "series": [
                {
                    "kind": "contour",
                    "x": [-1.0, 0.0, 1.0],
                    "y": [-1.0, 0.0, 1.0],
                    "z": [0.1, 0.2, 0.3, 0.2, 0.6, 0.2, 0.3, 0.2, 0.1],
                }
            ]
        });

        let contour_expected = Plot::new()
            .size_px(320, 200)
            .theme(Theme::light())
            .ticks(true)
            .title("Contour parity")
            .contour(
                &[-1.0, 0.0, 1.0],
                &[-1.0, 0.0, 1.0],
                &[0.1, 0.2, 0.3, 0.2, 0.6, 0.2, 0.3, 0.2, 0.1],
            )
            .into_plot();

        let actual_png = render_snapshot_png(contour_snapshot);
        let expected_png = render_direct_png(contour_expected);
        assert_pngs_equal("contour", &actual_png, &expected_png);

        let pie_snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "Pie parity",
            "series": [
                {
                    "kind": "pie",
                    "values": [30.0, 25.0, 20.0, 25.0],
                    "labels": ["A", "B", "C", "D"],
                }
            ]
        });

        let pie_expected = Plot::new()
            .size_px(320, 200)
            .theme(Theme::light())
            .ticks(true)
            .title("Pie parity")
            .pie(&[30.0, 25.0, 20.0, 25.0])
            .labels(&["A", "B", "C", "D"])
            .into_plot();

        let actual_png = render_snapshot_png(pie_snapshot);
        let expected_png = render_direct_png(pie_expected);
        assert_pngs_equal("pie", &actual_png, &expected_png);

        let radar_snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "Radar parity",
            "series": [
                {
                    "kind": "radar",
                    "labels": ["API", "Docs", "Export", "Interactive", "Scale"],
                    "series": [
                        {"name": "Python", "values": [4.5, 4.7, 4.8, 4.3, 4.0]},
                        {"name": "Web", "values": [4.2, 4.1, 4.0, 4.8, 4.6]},
                    ],
                }
            ]
        });

        let radar_expected = Plot::new()
            .size_px(320, 200)
            .theme(Theme::light())
            .ticks(true)
            .title("Radar parity")
            .radar(&["API", "Docs", "Export", "Interactive", "Scale"])
            .add_series("Python", &[4.5, 4.7, 4.8, 4.3, 4.0])
            .add_series("Web", &[4.2, 4.1, 4.0, 4.8, 4.6])
            .into_plot();

        let actual_png = render_snapshot_png(radar_snapshot);
        let expected_png = render_direct_png(radar_expected);
        assert_pngs_equal("radar", &actual_png, &expected_png);
    }
}
