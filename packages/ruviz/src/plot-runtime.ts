import type {
  JsPlot as RawJsPlot,
  ObservableVecF64 as RawObservableVecF64,
} from "../generated/raw/ruviz_web_raw.js";
import type {
  BackendPreference,
  NumericReactiveSourceSnapshot,
  PlotSnapshot,
  XSourceSnapshot,
  YSourceSnapshot,
} from "./shared.js";

type RawModule = typeof import("../generated/raw/ruviz_web_raw.js");

export function normalizeBackendPreference(
  backendPreference: BackendPreference | undefined,
): BackendPreference {
  switch (backendPreference) {
    case "cpu":
    case "svg":
    case "gpu":
      return backendPreference;
    default:
      return "auto";
  }
}

export function toRawBackendPreference(
  module: RawModule,
  backendPreference: BackendPreference,
): number {
  switch (backendPreference) {
    case "cpu":
      return module.WebBackendPreference.Cpu;
    case "svg":
      return module.WebBackendPreference.Svg;
    case "gpu":
      return module.WebBackendPreference.Gpu;
    default:
      return module.WebBackendPreference.Auto;
  }
}

function sourceValues(
  source: XSourceSnapshot | YSourceSnapshot | NumericReactiveSourceSnapshot,
): number[] {
  if (source.kind === "sine-signal") {
    throw new Error("sine-signal sources do not expose direct values");
  }

  return [...source.values];
}

function toObservable(
  module: RawModule,
  source: NumericReactiveSourceSnapshot,
): RawObservableVecF64 {
  return new module.ObservableVecF64(Float64Array.from(source.values));
}

/** The plot-level settings of a snapshot, without its series. */
export type PlotSnapshotMetadata = Omit<PlotSnapshot, "series">;

export function applySnapshotMetadata(rawPlot: RawJsPlot, snapshot: PlotSnapshotMetadata): void {
  // Figure size first: DPI, tight layout and max-resolution are all measured
  // against it. `sizeIn` wins over `sizePx` — a caller who asked for inches is
  // targeting a physical output, so honour that over a pixel hint.
  if (snapshot.sizePx) {
    rawPlot.size_px(snapshot.sizePx[0], snapshot.sizePx[1]);
  }

  if (snapshot.sizeIn) {
    rawPlot.size(snapshot.sizeIn[0], snapshot.sizeIn[1]);
  }

  // After the figure size: raising the DPI then scales the exported pixels
  // instead of reshaping the figure.
  if (typeof snapshot.dpi === "number") {
    rawPlot.dpi(snapshot.dpi);
  }

  // Shape-checked rather than `!== undefined`: a foreign snapshot with a null
  // or malformed value should degrade like every other field, not fail replay
  // in the fallible wasm setter. Destructuring turns sparse-array holes into
  // undefined (`.every` would skip them), and the u32 ceiling is what the
  // wasm setter can actually accept.
  if (Array.isArray(snapshot.maxResolution) && snapshot.maxResolution.length === 2) {
    const [maxWidth, maxHeight] = snapshot.maxResolution;
    if (
      [maxWidth, maxHeight].every(
        (value) => Number.isInteger(value) && value > 0 && value <= 4294967295,
      )
    ) {
      rawPlot.max_resolution(maxWidth, maxHeight);
    }
  }

  if (typeof snapshot.scientificNotation === "boolean") {
    rawPlot.scientific_notation(snapshot.scientificNotation);
  }

  if (typeof snapshot.margin === "number") {
    rawPlot.margin(snapshot.margin);
  }

  if (snapshot.theme) {
    // A snapshot may come from a newer ruviz with a theme this runtime does
    // not know; render with the default theme rather than failing the plot.
    try {
      rawPlot.theme(snapshot.theme);
    } catch {
      // unknown theme name: ignored
    }
  }

  // Typography and line width come *after* the theme: applying a theme replaces
  // the typography and line config wholesale (`apply_theme` assigns
  // `config.typography` and `config.lines`), so setting these first would let
  // the theme silently discard an explicitly requested font size or family.
  if (snapshot.fontFamily) {
    rawPlot.font_family(snapshot.fontFamily);
  }

  if (typeof snapshot.fontSize === "number") {
    rawPlot.font_size(snapshot.fontSize);
  }

  // After `fontSize`: the title size is stored internally as a ratio of it.
  if (typeof snapshot.titleSize === "number") {
    rawPlot.title_size(snapshot.titleSize);
  }

  if (typeof snapshot.scaleTypography === "number") {
    rawPlot.scale_typography(snapshot.scaleTypography);
  }

  if (typeof snapshot.lineWidthPt === "number") {
    rawPlot.line_width_pt(snapshot.lineWidthPt);
  }

  if (typeof snapshot.ticks === "boolean") {
    rawPlot.ticks(snapshot.ticks);
  }

  if (snapshot.title) {
    rawPlot.title(snapshot.title);
  }

  if (snapshot.xLabel) {
    rawPlot.xlabel(snapshot.xLabel);
  }

  if (snapshot.yLabel) {
    rawPlot.ylabel(snapshot.yLabel);
  }

  if (snapshot.legend) {
    rawPlot.legend(snapshot.legend);
  }

  if (typeof snapshot.grid === "boolean") {
    rawPlot.grid(snapshot.grid);
  }

  if (snapshot.xLim) {
    rawPlot.xlim(snapshot.xLim[0], snapshot.xLim[1]);
  }

  if (snapshot.yLim) {
    rawPlot.ylim(snapshot.yLim[0], snapshot.yLim[1]);
  }

  if (snapshot.xScale) {
    rawPlot.xscale(snapshot.xScale[0], snapshot.xScale[1]);
  }

  if (snapshot.yScale) {
    rawPlot.yscale(snapshot.yScale[0], snapshot.yScale[1]);
  }

  // Last: tight layout measures the text it packs around, so it has to run once
  // the title and axis labels that occupy those margins are actually set.
  if (typeof snapshot.tightLayoutPad === "number") {
    rawPlot.tight_layout_pad(snapshot.tightLayoutPad);
  }

  // Annotations replay in call order; the core keeps them on a separate layer
  // drawn after the data series, so their position in this apply sequence
  // only fixes their order relative to each other. Foreign snapshots degrade
  // rather than fail: the coordinate checks stop a missing or null value
  // from silently becoming 0, and the try/catch stops a malformed style from
  // throwing wasm-side and blanking the whole plot — one bad annotation is
  // skipped like an unknown kind.
  // A foreign snapshot may carry a non-array here; degrade like a missing
  // field rather than throwing before the per-entry guard can help.
  const annotations = Array.isArray(snapshot.annotations) ? snapshot.annotations : [];
  for (const annotation of annotations) {
    try {
      switch (annotation.kind) {
        case "vline":
          if (Number.isFinite(annotation.x)) {
            rawPlot.vline(annotation.x, annotation.style);
          }
          break;
        case "hline":
          if (Number.isFinite(annotation.y)) {
            rawPlot.hline(annotation.y, annotation.style);
          }
          break;
        case "text":
          if (
            Number.isFinite(annotation.x) &&
            Number.isFinite(annotation.y) &&
            typeof annotation.text === "string" &&
            annotation.text !== ""
          ) {
            rawPlot.annotate_text(annotation.x, annotation.y, annotation.text, annotation.style);
          }
          break;
        default:
          // A snapshot from a newer build may carry kinds this runtime does
          // not know; skip them rather than failing the whole plot.
          break;
      }
    } catch {
      // malformed style value: skip this annotation
    }
  }
}

export function buildRawPlotFromSnapshot(snapshot: PlotSnapshot, module: RawModule): RawJsPlot {
  const rawPlot = new module.JsPlot();
  applySnapshotMetadata(rawPlot, snapshot);

  for (const series of snapshot.series) {
    switch (series.kind) {
      case "line":
      case "scatter": {
        if (series.y.kind === "sine-signal") {
          const signal = module.SignalVecF64.sineWave(
            series.y.options.points,
            series.y.options.domainStart,
            series.y.options.domainEnd,
            series.y.options.amplitude,
            series.y.options.cycles,
            series.y.options.phaseVelocity,
            series.y.options.phaseOffset,
            series.y.options.verticalOffset,
          );
          const xValues = Float64Array.from(sourceValues(series.x));

          if (series.kind === "line") {
            rawPlot.line_signal(xValues, signal, series.style);
          } else {
            rawPlot.scatter_signal(xValues, signal, series.style);
          }
          break;
        }

        if (series.x.kind === "observable" || series.y.kind === "observable") {
          const xObservable = toObservable(module, series.x);
          const yObservable = toObservable(module, series.y);

          if (series.kind === "line") {
            rawPlot.line_observable(xObservable, yObservable, series.style);
          } else {
            rawPlot.scatter_observable(xObservable, yObservable, series.style);
          }
          break;
        }

        const xValues = Float64Array.from(sourceValues(series.x));
        const yValues = Float64Array.from(sourceValues(series.y));
        if (series.kind === "line") {
          rawPlot.line(xValues, yValues, series.style);
        } else {
          rawPlot.scatter(xValues, yValues, series.style);
        }
        break;
      }
      case "bar": {
        if (series.values.kind === "observable") {
          rawPlot.bar_observable(
            series.categories,
            toObservable(module, series.values),
            series.style,
          );
        } else {
          rawPlot.bar(series.categories, Float64Array.from(series.values.values), series.style);
        }
        break;
      }
      case "histogram": {
        if (series.data.kind === "observable") {
          rawPlot.histogram_observable(toObservable(module, series.data), series.style);
        } else {
          rawPlot.histogram(Float64Array.from(series.data.values), series.style);
        }
        break;
      }
      case "boxplot": {
        if (series.data.kind === "observable") {
          rawPlot.boxplot_observable(toObservable(module, series.data), series.style);
        } else {
          rawPlot.boxplot(Float64Array.from(series.data.values), series.style);
        }
        break;
      }
      case "heatmap":
        rawPlot.heatmap(Float64Array.from(series.values), series.rows, series.cols);
        break;
      case "error-bars": {
        if (
          series.x.kind === "observable" ||
          series.y.kind === "observable" ||
          series.yErrors.kind === "observable"
        ) {
          rawPlot.error_bars_observable(
            toObservable(module, series.x),
            toObservable(module, series.y),
            toObservable(module, series.yErrors),
            series.style,
          );
        } else {
          rawPlot.error_bars(
            Float64Array.from(series.x.values),
            Float64Array.from(series.y.values),
            Float64Array.from(series.yErrors.values),
            series.style,
          );
        }
        break;
      }
      case "error-bars-xy": {
        if (
          series.x.kind === "observable" ||
          series.y.kind === "observable" ||
          series.xErrors.kind === "observable" ||
          series.yErrors.kind === "observable"
        ) {
          rawPlot.error_bars_xy_observable(
            toObservable(module, series.x),
            toObservable(module, series.y),
            toObservable(module, series.xErrors),
            toObservable(module, series.yErrors),
            series.style,
          );
        } else {
          rawPlot.error_bars_xy(
            Float64Array.from(series.x.values),
            Float64Array.from(series.y.values),
            Float64Array.from(series.xErrors.values),
            Float64Array.from(series.yErrors.values),
            series.style,
          );
        }
        break;
      }
      case "kde":
        rawPlot.kde(Float64Array.from(series.data), series.style);
        break;
      case "ecdf":
        rawPlot.ecdf(Float64Array.from(series.data), series.style);
        break;
      case "contour":
        rawPlot.contour(
          Float64Array.from(series.x),
          Float64Array.from(series.y),
          Float64Array.from(series.z),
          series.style,
        );
        break;
      case "pie":
        if (series.labels && series.labels.length > 0) {
          rawPlot.pie_with_labels(Float64Array.from(series.values), series.labels);
        } else {
          rawPlot.pie(Float64Array.from(series.values));
        }
        break;
      case "radar": {
        const flattened: number[] = [];
        const names: string[] = [];
        for (const item of series.series) {
          flattened.push(...item.values);
          names.push(item.name ?? "");
        }
        rawPlot.radar(series.labels, names, Float64Array.from(flattened));
        break;
      }
      case "violin":
        rawPlot.violin(Float64Array.from(series.data), series.style);
        break;
      case "polar-line":
        rawPlot.polar_line(
          Float64Array.from(series.r),
          Float64Array.from(series.theta),
          series.style,
        );
        break;
    }
  }

  return rawPlot;
}
