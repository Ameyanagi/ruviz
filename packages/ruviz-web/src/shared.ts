export type NumericArray = number[] | Float64Array | ArrayLike<number>;
export type BackendPreference = "auto" | "cpu" | "svg" | "gpu";
export type SessionMode = "main-thread" | "worker";
export type PlotTheme = "light" | "dark";
export type PlotSaveFormat = "png" | "svg";

export interface RuntimeCapabilities {
  offscreenCanvasSupported: boolean;
  workerSupported: boolean;
  webgpuSupported: boolean;
  touchInputSupported: boolean;
  defaultBrowserFontRegistered: boolean;
  gpuCanvasFastPathAvailable: boolean;
}

export interface CanvasSessionOptions {
  backendPreference?: BackendPreference;
  autoResize?: boolean;
  bindInput?: boolean;
  initialTime?: number;
}

export interface WorkerSessionOptions extends CanvasSessionOptions {
  fallbackToMainThread?: boolean;
}

export interface PlotSaveOptions {
  fileName?: string;
  format?: PlotSaveFormat;
}

export interface SineSignalOptions {
  points: number;
  domain?: readonly [number, number];
  amplitude?: number;
  cycles?: number;
  phaseVelocity?: number;
  phaseOffset?: number;
  verticalOffset?: number;
}

export interface NormalizedSineSignalOptions {
  points: number;
  domainStart: number;
  domainEnd: number;
  amplitude: number;
  cycles: number;
  phaseVelocity: number;
  phaseOffset: number;
  verticalOffset: number;
}

export interface StaticSourceSnapshot {
  kind: "static";
  values: number[];
}

export interface ObservableSourceSnapshot {
  kind: "observable";
  values: number[];
}

export interface SignalSourceSnapshot {
  kind: "sine-signal";
  options: NormalizedSineSignalOptions;
}

export type NumericReactiveSourceSnapshot = StaticSourceSnapshot | ObservableSourceSnapshot;
export type XSourceSnapshot = NumericReactiveSourceSnapshot;
export type YSourceSnapshot =
  | NumericReactiveSourceSnapshot
  | SignalSourceSnapshot;

export interface LineSeriesSnapshot {
  kind: "line";
  x: XSourceSnapshot;
  y: YSourceSnapshot;
}

export interface ScatterSeriesSnapshot {
  kind: "scatter";
  x: XSourceSnapshot;
  y: YSourceSnapshot;
}

export interface BarSeriesSnapshot {
  kind: "bar";
  categories: string[];
  values: NumericReactiveSourceSnapshot;
}

export interface HistogramSeriesSnapshot {
  kind: "histogram";
  data: NumericReactiveSourceSnapshot;
}

export interface BoxplotSeriesSnapshot {
  kind: "boxplot";
  data: NumericReactiveSourceSnapshot;
}

export interface HeatmapSeriesSnapshot {
  kind: "heatmap";
  values: number[];
  rows: number;
  cols: number;
}

export interface ErrorBarsSeriesSnapshot {
  kind: "error-bars";
  x: NumericReactiveSourceSnapshot;
  y: NumericReactiveSourceSnapshot;
  yErrors: NumericReactiveSourceSnapshot;
}

export interface ErrorBarsXYSeriesSnapshot {
  kind: "error-bars-xy";
  x: NumericReactiveSourceSnapshot;
  y: NumericReactiveSourceSnapshot;
  xErrors: NumericReactiveSourceSnapshot;
  yErrors: NumericReactiveSourceSnapshot;
}

export interface KdeSeriesSnapshot {
  kind: "kde";
  data: number[];
}

export interface EcdfSeriesSnapshot {
  kind: "ecdf";
  data: number[];
}

export interface ContourSeriesSnapshot {
  kind: "contour";
  x: number[];
  y: number[];
  z: number[];
}

export interface PieSeriesSnapshot {
  kind: "pie";
  values: number[];
  labels?: string[];
}

export interface RadarSeriesItemSnapshot {
  name?: string;
  values: number[];
}

export interface RadarSeriesSnapshot {
  kind: "radar";
  labels: string[];
  series: RadarSeriesItemSnapshot[];
}

export interface ViolinSeriesSnapshot {
  kind: "violin";
  data: number[];
}

export interface PolarLineSeriesSnapshot {
  kind: "polar-line";
  r: number[];
  theta: number[];
}

export type PlotSeriesSnapshot =
  | LineSeriesSnapshot
  | ScatterSeriesSnapshot
  | BarSeriesSnapshot
  | HistogramSeriesSnapshot
  | BoxplotSeriesSnapshot
  | HeatmapSeriesSnapshot
  | ErrorBarsSeriesSnapshot
  | ErrorBarsXYSeriesSnapshot
  | KdeSeriesSnapshot
  | EcdfSeriesSnapshot
  | ContourSeriesSnapshot
  | PieSeriesSnapshot
  | RadarSeriesSnapshot
  | ViolinSeriesSnapshot
  | PolarLineSeriesSnapshot;

export interface PlotSnapshot {
  sizePx?: [number, number];
  theme?: PlotTheme;
  ticks?: boolean;
  title?: string;
  xLabel?: string;
  yLabel?: string;
  series: PlotSeriesSnapshot[];
}

export function toNumberArray(values: NumericArray): number[] {
  return Array.from(values, (value) => Number(value));
}

function finiteNumber(value: number | undefined, fallback: number): number {
  return Number.isFinite(value) ? Number(value) : fallback;
}

export function normalizeSineSignalOptions(
  options: SineSignalOptions,
): NormalizedSineSignalOptions {
  const points = Math.max(2, Math.floor(finiteNumber(options.points, 2)));
  const domainStart = finiteNumber(options.domain?.[0], 0);
  const defaultDomainEnd = domainStart + Math.PI * 2;
  const rawDomainEnd = finiteNumber(options.domain?.[1], defaultDomainEnd);
  const domainEnd = rawDomainEnd === domainStart ? defaultDomainEnd : rawDomainEnd;

  return {
    points,
    domainStart,
    domainEnd,
    amplitude: finiteNumber(options.amplitude, 1),
    cycles: finiteNumber(options.cycles, 1),
    phaseVelocity: finiteNumber(options.phaseVelocity, 0),
    phaseOffset: finiteNumber(options.phaseOffset, 0),
    verticalOffset: finiteNumber(options.verticalOffset, 0),
  };
}

export function cloneSourceSnapshot<T extends XSourceSnapshot | YSourceSnapshot>(source: T): T {
  if (source.kind === "sine-signal") {
    return {
      kind: "sine-signal",
      options: { ...source.options },
    } as T;
  }

  return {
    kind: source.kind,
    values: [...source.values],
  } as T;
}

function cloneSeriesSnapshot(series: PlotSeriesSnapshot): PlotSeriesSnapshot {
  switch (series.kind) {
    case "line":
    case "scatter":
      return {
        kind: series.kind,
        x: cloneSourceSnapshot(series.x),
        y: cloneSourceSnapshot(series.y),
      };
    case "bar":
      return {
        kind: "bar",
        categories: [...series.categories],
        values: cloneSourceSnapshot(series.values),
      };
    case "histogram":
      return {
        kind: "histogram",
        data: cloneSourceSnapshot(series.data),
      };
    case "boxplot":
      return {
        kind: "boxplot",
        data: cloneSourceSnapshot(series.data),
      };
    case "heatmap":
      return {
        kind: "heatmap",
        values: [...series.values],
        rows: series.rows,
        cols: series.cols,
      };
    case "error-bars":
      return {
        kind: "error-bars",
        x: cloneSourceSnapshot(series.x),
        y: cloneSourceSnapshot(series.y),
        yErrors: cloneSourceSnapshot(series.yErrors),
      };
    case "error-bars-xy":
      return {
        kind: "error-bars-xy",
        x: cloneSourceSnapshot(series.x),
        y: cloneSourceSnapshot(series.y),
        xErrors: cloneSourceSnapshot(series.xErrors),
        yErrors: cloneSourceSnapshot(series.yErrors),
      };
    case "kde":
      return { kind: "kde", data: [...series.data] };
    case "ecdf":
      return { kind: "ecdf", data: [...series.data] };
    case "contour":
      return {
        kind: "contour",
        x: [...series.x],
        y: [...series.y],
        z: [...series.z],
      };
    case "pie":
      return {
        kind: "pie",
        values: [...series.values],
        labels: series.labels ? [...series.labels] : undefined,
      };
    case "radar":
      return {
        kind: "radar",
        labels: [...series.labels],
        series: series.series.map((item) => ({
          name: item.name,
          values: [...item.values],
        })),
      };
    case "violin":
      return { kind: "violin", data: [...series.data] };
    case "polar-line":
      return {
        kind: "polar-line",
        r: [...series.r],
        theta: [...series.theta],
      };
  }
}

export function clonePlotSnapshot(snapshot: PlotSnapshot): PlotSnapshot {
  return {
    sizePx: snapshot.sizePx ? ([...snapshot.sizePx] as [number, number]) : undefined,
    theme: snapshot.theme,
    ticks: snapshot.ticks,
    title: snapshot.title,
    xLabel: snapshot.xLabel,
    yLabel: snapshot.yLabel,
    series: snapshot.series.map(cloneSeriesSnapshot),
  };
}
