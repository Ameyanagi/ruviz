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

export function applySnapshotMetadata(rawPlot: RawJsPlot, snapshot: PlotSnapshot): void {
  if (snapshot.sizePx) {
    rawPlot.size_px(snapshot.sizePx[0], snapshot.sizePx[1]);
  }

  if (snapshot.theme === "dark") {
    rawPlot.theme_dark();
  } else if (snapshot.theme === "light") {
    rawPlot.theme_light();
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
            rawPlot.line_signal(xValues, signal);
          } else {
            rawPlot.scatter_signal(xValues, signal);
          }
          break;
        }

        if (series.x.kind === "observable" || series.y.kind === "observable") {
          const xObservable = toObservable(module, series.x);
          const yObservable = toObservable(module, series.y);

          if (series.kind === "line") {
            rawPlot.line_observable(xObservable, yObservable);
          } else {
            rawPlot.scatter_observable(xObservable, yObservable);
          }
          break;
        }

        const xValues = Float64Array.from(sourceValues(series.x));
        const yValues = Float64Array.from(sourceValues(series.y));
        if (series.kind === "line") {
          rawPlot.line(xValues, yValues);
        } else {
          rawPlot.scatter(xValues, yValues);
        }
        break;
      }
      case "bar": {
        if (series.values.kind === "observable") {
          rawPlot.bar_observable(series.categories, toObservable(module, series.values));
        } else {
          rawPlot.bar(series.categories, Float64Array.from(series.values.values));
        }
        break;
      }
      case "histogram": {
        if (series.data.kind === "observable") {
          rawPlot.histogram_observable(toObservable(module, series.data));
        } else {
          rawPlot.histogram(Float64Array.from(series.data.values));
        }
        break;
      }
      case "boxplot": {
        if (series.data.kind === "observable") {
          rawPlot.boxplot_observable(toObservable(module, series.data));
        } else {
          rawPlot.boxplot(Float64Array.from(series.data.values));
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
          );
        } else {
          rawPlot.error_bars(
            Float64Array.from(series.x.values),
            Float64Array.from(series.y.values),
            Float64Array.from(series.yErrors.values),
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
          );
        } else {
          rawPlot.error_bars_xy(
            Float64Array.from(series.x.values),
            Float64Array.from(series.y.values),
            Float64Array.from(series.xErrors.values),
            Float64Array.from(series.yErrors.values),
          );
        }
        break;
      }
      case "kde":
        rawPlot.kde(Float64Array.from(series.data));
        break;
      case "ecdf":
        rawPlot.ecdf(Float64Array.from(series.data));
        break;
      case "contour":
        rawPlot.contour(
          Float64Array.from(series.x),
          Float64Array.from(series.y),
          Float64Array.from(series.z),
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
        rawPlot.violin(Float64Array.from(series.data));
        break;
      case "polar-line":
        rawPlot.polar_line(Float64Array.from(series.r), Float64Array.from(series.theta));
        break;
    }
  }

  return rawPlot;
}
