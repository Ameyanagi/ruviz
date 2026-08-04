export type NumericArray = number[] | Float64Array | ArrayLike<number>;
export type BackendPreference = "auto" | "cpu" | "svg" | "gpu";
export type SessionMode = "main-thread" | "worker";
export type PlotTheme = "light" | "dark";
export type PlotSaveFormat = "png" | "svg";

/** Line dash pattern accepted by `style.linestyle`. */
export type LineStyleName = "solid" | "dashed" | "dotted" | "dash-dot" | "dash-dot-dot";

/** Marker shape accepted by `style.marker`. */
export type MarkerName =
  | "circle"
  | "square"
  | "triangle"
  | "triangle-down"
  | "diamond"
  | "plus"
  | "cross"
  | "star"
  | "circle-open"
  | "square-open"
  | "triangle-open"
  | "diamond-open";

/** Legend placement; `best` auto-places the legend. */
export type LegendPositionName =
  | "best"
  | "upper_right"
  | "upper_left"
  | "lower_left"
  | "lower_right"
  | "right"
  | "center_left"
  | "center_right"
  | "lower_center"
  | "upper_center"
  | "center"
  | "outside_right"
  | "outside_left"
  | "outside_upper"
  | "outside_lower";

/** Axis scale accepted by `xScale`/`yScale`. */
export type AxisScaleName = "linear" | "log" | "symlog";

/** Serialized axis scale; the trailing threshold applies to `symlog` only. */
export type AxisScaleSnapshot = [scale: AxisScaleName, linthresh?: number];

/**
 * Per-series styling. Keys are camelCase to match the snapshot spelling shared
 * with the Python binding; each maps to one core plot builder setter.
 */
export interface SeriesStyleSnapshot {
  label?: string;
  color?: string;
  alpha?: number;
  width?: number;
  linestyle?: LineStyleName;
  marker?: MarkerName;
  markerSize?: number;
  bins?: number;
  bandwidth?: number;
  levels?: number;
}

/** Styling every series kind shares. */
export type CommonSeriesStyle = Pick<SeriesStyleSnapshot, "label" | "color" | "alpha">;
/** Styling for the kinds drawn with a stroked outline. */
export type StrokedSeriesStyle = CommonSeriesStyle & Pick<SeriesStyleSnapshot, "width">;
export type LineSeriesStyle = StrokedSeriesStyle &
  Pick<SeriesStyleSnapshot, "linestyle" | "marker" | "markerSize">;
export type ScatterSeriesStyle = CommonSeriesStyle &
  Pick<SeriesStyleSnapshot, "marker" | "markerSize">;
export type BoxplotSeriesStyle = StrokedSeriesStyle & Pick<SeriesStyleSnapshot, "linestyle">;
export type HistogramSeriesStyle = CommonSeriesStyle & Pick<SeriesStyleSnapshot, "bins">;
export type KdeSeriesStyle = StrokedSeriesStyle & Pick<SeriesStyleSnapshot, "bandwidth">;
export type ContourSeriesStyle = Pick<SeriesStyleSnapshot, "alpha" | "width" | "levels">;

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
export type YSourceSnapshot = NumericReactiveSourceSnapshot | SignalSourceSnapshot;

export interface LineSeriesSnapshot {
  kind: "line";
  style?: LineSeriesStyle;
  x: XSourceSnapshot;
  y: YSourceSnapshot;
}

export interface ScatterSeriesSnapshot {
  kind: "scatter";
  style?: ScatterSeriesStyle;
  x: XSourceSnapshot;
  y: YSourceSnapshot;
}

export interface BarSeriesSnapshot {
  kind: "bar";
  style?: CommonSeriesStyle;
  categories: string[];
  values: NumericReactiveSourceSnapshot;
}

export interface HistogramSeriesSnapshot {
  kind: "histogram";
  style?: HistogramSeriesStyle;
  data: NumericReactiveSourceSnapshot;
}

export interface BoxplotSeriesSnapshot {
  kind: "boxplot";
  style?: BoxplotSeriesStyle;
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
  style?: StrokedSeriesStyle;
  x: NumericReactiveSourceSnapshot;
  y: NumericReactiveSourceSnapshot;
  yErrors: NumericReactiveSourceSnapshot;
}

export interface ErrorBarsXYSeriesSnapshot {
  kind: "error-bars-xy";
  style?: StrokedSeriesStyle;
  x: NumericReactiveSourceSnapshot;
  y: NumericReactiveSourceSnapshot;
  xErrors: NumericReactiveSourceSnapshot;
  yErrors: NumericReactiveSourceSnapshot;
}

export interface KdeSeriesSnapshot {
  kind: "kde";
  style?: KdeSeriesStyle;
  data: number[];
}

export interface EcdfSeriesSnapshot {
  kind: "ecdf";
  style?: StrokedSeriesStyle;
  data: number[];
}

export interface ContourSeriesSnapshot {
  kind: "contour";
  style?: ContourSeriesStyle;
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
  style?: StrokedSeriesStyle;
  data: number[];
}

export interface PolarLineSeriesSnapshot {
  kind: "polar-line";
  style?: StrokedSeriesStyle;
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

/** Snapshot layout version written by this build; consumers ignore unknown keys. */
export const SNAPSHOT_SCHEMA_VERSION = 1;

export interface PlotSnapshot {
  /** Snapshot layout version; absent on snapshots written before it existed. */
  schemaVersion?: number;
  sizePx?: [number, number];
  theme?: PlotTheme;
  ticks?: boolean;
  title?: string;
  xLabel?: string;
  yLabel?: string;
  legend?: LegendPositionName;
  grid?: boolean;
  xLim?: [number, number];
  yLim?: [number, number];
  xScale?: AxisScaleSnapshot;
  yScale?: AxisScaleSnapshot;
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
  return cloneSnapshotValue(source);
}

/**
 * Deep-copy a snapshot value, keeping fields this build does not know about.
 * Snapshots cross the notebook widget boundary as plain JSON written by other
 * ruviz versions, so cloning structurally keeps newer keys instead of dropping
 * every field the copy does not name.
 */
function cloneSnapshotValue<T>(value: T): T {
  return structuredClone(value);
}

export function cloneSeriesSnapshot(series: PlotSeriesSnapshot): PlotSeriesSnapshot {
  return cloneSnapshotValue(series);
}

export function clonePlotSnapshot(snapshot: PlotSnapshot): PlotSnapshot {
  return cloneSnapshotValue(snapshot);
}
