export type NumericArray = number[] | Float64Array | ArrayLike<number>;
export type BackendPreference = "auto" | "cpu" | "svg" | "gpu";
export type SessionMode = "main-thread" | "worker";
export type PlotTheme = (typeof PLOT_THEME_NAMES)[number];
export type PlotSaveFormat = "png" | "svg";

/**
 * The accepted names, listed in the order the wasm lookup tables use so a
 * rejection here reports exactly what the renderer would have reported.
 */
export const LINE_STYLE_NAMES = ["solid", "dashed", "dotted", "dash-dot", "dash-dot-dot"] as const;

export const MARKER_NAMES = [
  "circle",
  "square",
  "triangle",
  "triangle-down",
  "diamond",
  "plus",
  "cross",
  "star",
  "circle-open",
  "square-open",
  "triangle-open",
  "diamond-open",
] as const;

export const LEGEND_POSITION_NAMES = [
  "best",
  "upper_right",
  "upper_left",
  "lower_left",
  "lower_right",
  "right",
  "center_left",
  "center_right",
  "lower_center",
  "upper_center",
  "center",
  "outside_right",
  "outside_left",
  "outside_upper",
  "outside_lower",
] as const;

export const AXIS_SCALE_NAMES = ["linear", "log", "symlog"] as const;

/** Built-in theme names; `seaborn` reproduces `seaborn.set_theme()`. */
export const PLOT_THEME_NAMES = [
  "light",
  "dark",
  "seaborn",
  "publication",
  "minimal",
  "presentation",
] as const;

/** Line dash pattern accepted by `style.linestyle`. */
export type LineStyleName = (typeof LINE_STYLE_NAMES)[number];

/** Marker shape accepted by `style.marker`. */
export type MarkerName = (typeof MARKER_NAMES)[number];

/** Legend placement; `best` auto-places the legend. */
export type LegendPositionName = (typeof LEGEND_POSITION_NAMES)[number];

/** Axis scale accepted by `xScale`/`yScale`. */
export type AxisScaleName = (typeof AXIS_SCALE_NAMES)[number];

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
  /** Normalize histogram bars to a probability density, so a KDE overlay lines up. */
  density?: boolean;
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
export type HistogramSeriesStyle = CommonSeriesStyle &
  Pick<SeriesStyleSnapshot, "bins" | "density">;
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
export const SNAPSHOT_SCHEMA_VERSION = 2;

export interface PlotSnapshot {
  /** Snapshot layout version; absent on snapshots written before it existed. */
  schemaVersion?: number;
  sizePx?: [number, number];
  /**
   * Figure size in inches — the unit journals specify (single column is
   * typically 3.25, double column 6.5). Applied before `dpi`, which then fixes
   * the exported pixel dimensions. Takes precedence over `sizePx`.
   */
  sizeIn?: [number, number];
  /** Output DPI; applied after the figure size, so raising it scales the exported pixels. */
  dpi?: number;
  /** Base text size in points; every other text size derives from it. */
  fontSize?: number;
  /** Title size in points, absolute rather than relative to `fontSize`. */
  titleSize?: number;
  /** `serif` | `sans-serif` | `monospace` | `cursive` | `fantasy`, or a registered family name. */
  fontFamily?: string;
  /** Scales every text size at once, preserving the typographic hierarchy. */
  scaleTypography?: number;
  /** Data line width in points. */
  lineWidthPt?: number;
  /** Plot margin as a fraction of the figure, 0.0–0.5. */
  margin?: number;
  /** Shrink margins to fit text, leaving this many points of slack. */
  tightLayoutPad?: number;
  /** Scientific notation on axis tick labels. */
  scientificNotation?: boolean;
  /** Caps exported pixel dimensions while preserving aspect ratio. */
  maxResolution?: [number, number];
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

/**
 * Color names the core renderer resolves, mirroring `Color::named`. Keeping the
 * list here lets the builder reject a bad color at the call instead of at the
 * next async render, with the message the renderer would have produced.
 */
const COLOR_NAMES = new Set([
  "red",
  "green",
  "blue",
  "yellow",
  "orange",
  "purple",
  "cyan",
  "magenta",
  "black",
  "white",
  "gray",
  "grey",
  "lightgray",
  "lightgrey",
  "light_gray",
  "light_grey",
  "darkgray",
  "darkgrey",
  "dark_gray",
  "dark_grey",
  "pink",
  "brown",
  "lime",
  "navy",
  "teal",
  "olive",
  "maroon",
  "aqua",
  "fuchsia",
  "silver",
  "coral",
  "salmon",
  "gold",
  "indigo",
  "violet",
  "crimson",
]);

/** The names `Color::suggest_named` searches, in its order. */
const SUGGESTED_COLOR_NAMES = [
  "red",
  "green",
  "blue",
  "yellow",
  "orange",
  "purple",
  "cyan",
  "magenta",
  "black",
  "white",
  "gray",
  "grey",
  "pink",
  "brown",
  "lime",
  "navy",
  "teal",
  "olive",
  "maroon",
  "aqua",
  "fuchsia",
  "silver",
  "coral",
  "salmon",
  "gold",
  "indigo",
  "violet",
  "crimson",
];

const HEX_COLOR = /^#?(?:[0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i;

/** Port of `Color::suggest_named`, so a typo gets the same hint everywhere. */
function suggestColorName(name: string): string | undefined {
  const lowered = name.toLowerCase();
  for (const candidate of SUGGESTED_COLOR_NAMES) {
    if (candidate.startsWith(lowered) && candidate.length <= lowered.length + 2) {
      return candidate;
    }
    if (lowered.startsWith(candidate) && lowered.length <= candidate.length + 2) {
      return candidate;
    }
    if (lowered.length === candidate.length) {
      let differences = 0;
      for (let index = 0; index < candidate.length; index += 1) {
        if (lowered[index] !== candidate[index]) {
          differences += 1;
        }
      }
      if (differences <= 1) {
        return candidate;
      }
    }
  }
  return undefined;
}

/** Reject a color the renderer could not resolve, with the renderer's message. */
export function validateColor(value: string): void {
  if (COLOR_NAMES.has(value.toLowerCase()) || HEX_COLOR.test(value)) {
    return;
  }

  const suggestion = suggestColorName(value);
  throw new RangeError(
    `unsupported color '${value}'; expected a hex string like '#2563eb' ` +
      "or a named color such as red, green, blue, orange, purple, black, white, gray" +
      (suggestion ? ` (did you mean '${suggestion}'?)` : ""),
  );
}

/** Reject a name absent from a lookup table, with the renderer's message. */
export function validateName(table: readonly string[], kind: string, name: string): void {
  if (!table.includes(name)) {
    throw new RangeError(`unsupported ${kind} '${name}'; expected one of: ${table.join(", ")}`);
  }
}

/**
 * Figure-level presentation settings, applied together by `PlotBuilder.figure`.
 *
 * These are grouped rather than exposed as individual chained setters because
 * they are chosen as a set — a journal's column width, body point size and rule
 * weight are one decision, not eight. Every field is optional; omitting one
 * leaves the current value untouched.
 */
export interface FigureOptions {
  /** Figure size in inches, `[width, height]` — the unit journals specify. */
  size?: [number, number];
  /** Output DPI, applied after `size` to fix the exported pixel dimensions. */
  dpi?: number;
  /** Base text size in points; other text sizes derive from it. */
  fontSize?: number;
  /** Title size in points, absolute rather than relative to `fontSize`. */
  titleSize?: number;
  /** `serif` | `sans-serif` | `monospace` | `cursive` | `fantasy`, or a registered family. */
  fontFamily?: string;
  /** Scales every text size at once, preserving the typographic hierarchy. */
  scaleTypography?: number;
  /** Data line width in points. */
  lineWidthPt?: number;
  /** Plot margin as a fraction of the figure, 0.0–0.5. */
  margin?: number;
  /** Shrink margins to fit the text, leaving this many points of slack. */
  tightLayoutPad?: number;
  /** Scientific notation on axis tick labels. */
  scientificNotation?: boolean;
  /** Cap exported pixel dimensions while preserving aspect ratio. */
  maxResolution?: [number, number];
}

/**
 * Reject values the core builders would silently clamp to a default. Clamping
 * is right for Rust call sites, but from JS an `undefined` arrives as NaN and
 * would otherwise produce a wrong figure with no diagnostic.
 */
export function assertFinitePositive(value: number, label: string): void {
  // Bounded to positive finite f32 values: the wasm layer stores these as
  // f32, and a value that overflows to Infinity (or rounds to zero) there
  // would be rejected only later, at rebuild time.
  if (!Number.isFinite(value) || value < 1.1754943508222875e-38 || value > 3.4028234663852886e38) {
    throw new RangeError(`${label} must be a finite number greater than zero`);
  }
}

/** Reject a `[width, height]` pair that is not actually a two-element array. */
export function assertPair(value: unknown, label: string): asserts value is [number, number] {
  if (!Array.isArray(value) || value.length !== 2) {
    throw new TypeError(`${label} must be a [width, height] pair`);
  }
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
