import initRaw, * as raw from "../generated/raw/ruviz_web_raw.js";
import type {
  JsPlot as RawJsPlot,
  ObservableVecF64 as RawObservableVecF64,
  SignalVecF64 as RawSignalVecF64,
  WebCanvasSession as RawWebCanvasSession,
} from "../generated/raw/ruviz_web_raw.js";
import {
  AXIS_SCALE_NAMES,
  BAR_ORIENTATION_NAMES,
  cloneSeriesSnapshot,
  clonePlotSnapshot,
  LEGEND_POSITION_NAMES,
  LINE_STYLE_NAMES,
  MARKER_NAMES,
  normalizeSineSignalOptions,
  PLOT_THEME_NAMES,
  SNAPSHOT_SCHEMA_VERSION,
  toNumberArray,
  validateColor,
  assertFinitePositive,
  assertPair,
  validateName,
  type AxisScaleName,
  type AxisScaleSnapshot,
  type BackendPreference,
  type BarSeriesSnapshot,
  type BarOrientationName,
  type BarSeriesStyle,
  type BoxplotSeriesStyle,
  type CanvasSessionOptions,
  type BoxplotSeriesSnapshot,
  type CommonSeriesStyle,
  type ContourSeriesSnapshot,
  type ContourSeriesStyle,
  type EcdfSeriesSnapshot,
  type ErrorBarsSeriesSnapshot,
  type ErrorBarsXYSeriesSnapshot,
  type FigureOptions,
  type HeatmapSeriesSnapshot,
  type HistogramSeriesSnapshot,
  type HistogramSeriesStyle,
  type KdeSeriesSnapshot,
  type KdeSeriesStyle,
  type LegendPositionName,
  type LineSeriesStyle,
  type LineStyleName,
  type MarkerName,
  type NumericReactiveSourceSnapshot,
  type NormalizedSineSignalOptions,
  type NumericArray,
  type ReferenceLineStyle,
  type PieSeriesSnapshot,
  type PlotAnnotationSnapshot,
  type PlotSnapshot,
  type PlotSaveOptions,
  type PlotSeriesSnapshot,
  type PlotTheme,
  type PolarLineSeriesSnapshot,
  type RadarSeriesSnapshot,
  type RuntimeCapabilities,
  type ScatterSeriesStyle,
  type SeriesHit,
  type SeriesStyleSnapshot,
  type SessionMode,
  type SineSignalOptions,
  type StaticSourceSnapshot,
  type StrokedSeriesStyle,
  type TextAnnotationStyle,
  type ViolinSeriesSnapshot,
  type WorkerSessionOptions,
  type XSourceSnapshot,
  type YSourceSnapshot,
} from "./shared.js";
import {
  applySnapshotMetadata,
  normalizeBackendPreference,
  toRawBackendPreference,
} from "./plot-runtime.js";

export { BAR_ORIENTATION_NAMES, PLOT_THEME_NAMES, SNAPSHOT_SCHEMA_VERSION } from "./shared.js";

export {
  createPlot3d,
  line3d,
  Plot3dBuilder,
  scatter3d,
  surface,
  wireframe,
  type GridValues3d,
  type Plot3dMountOptions,
  type Plot3dSession,
  type Plot3dSessionMode,
} from "./3d.js";

export type {
  AxisScaleName,
  AxisScaleSnapshot,
  BackendPreference,
  FigureOptions,
  HLineAnnotationSnapshot,
  PlotAnnotationSnapshot,
  ReferenceLineStyle,
  TextAnnotationSnapshot,
  TextAnnotationStyle,
  VLineAnnotationSnapshot,
  BarSeriesSnapshot,
  BarOrientationName,
  BarSeriesStyle,
  BoxplotSeriesStyle,
  CanvasSessionOptions,
  BoxplotSeriesSnapshot,
  CommonSeriesStyle,
  ContourSeriesSnapshot,
  ContourSeriesStyle,
  EcdfSeriesSnapshot,
  ErrorBarsSeriesSnapshot,
  ErrorBarsXYSeriesSnapshot,
  HeatmapSeriesSnapshot,
  HistogramSeriesSnapshot,
  HistogramSeriesStyle,
  KdeSeriesSnapshot,
  KdeSeriesStyle,
  LegendPositionName,
  LineSeriesStyle,
  LineStyleName,
  MarkerName,
  NumericArray,
  PieSeriesSnapshot,
  PlotTheme,
  PlotSaveOptions,
  PlotSnapshot,
  PlotSeriesSnapshot,
  PolarLineSeriesSnapshot,
  RadarSeriesSnapshot,
  RuntimeCapabilities,
  ScatterSeriesStyle,
  SeriesHit,
  SeriesStyleSnapshot,
  SessionMode,
  SineSignalOptions,
  StaticSourceSnapshot,
  StrokedSeriesStyle,
  ViolinSeriesSnapshot,
  WorkerSessionOptions,
} from "./shared.js";

type RawModule = typeof import("../generated/raw/ruviz_web_raw.js");

type StaticSourceDefinition = { kind: "static"; values: number[] };
type ObservableSourceDefinition = { kind: "observable"; source: ObservableSeries };
type SignalSourceDefinition = { kind: "sine-signal"; source: SineSignal };
type NumericReactiveSourceDefinition = StaticSourceDefinition | ObservableSourceDefinition;
type XSourceDefinition = NumericReactiveSourceDefinition;
type YSourceDefinition = NumericReactiveSourceDefinition | SignalSourceDefinition;

interface XYSeriesInput {
  x: NumericArray | ObservableSeries;
  y: NumericArray | ObservableSeries | SineSignal;
}

interface LineSeriesInput extends XYSeriesInput {
  style?: LineSeriesStyle;
}

interface ScatterSeriesInput extends XYSeriesInput {
  style?: ScatterSeriesStyle;
}

interface BarSeriesInput {
  categories: readonly string[] | ArrayLike<string>;
  values: NumericArray | ObservableSeries;
  style?: BarSeriesStyle;
}

interface ErrorBarsInput {
  x: NumericArray | ObservableSeries;
  y: NumericArray | ObservableSeries;
  yErrors: NumericArray | ObservableSeries;
  style?: StrokedSeriesStyle;
}

interface ErrorBarsXYInput {
  x: NumericArray | ObservableSeries;
  y: NumericArray | ObservableSeries;
  xErrors: NumericArray | ObservableSeries;
  yErrors: NumericArray | ObservableSeries;
  style?: StrokedSeriesStyle;
}

interface RadarSeriesInput {
  name?: string;
  values: NumericArray;
}

interface RadarInput {
  labels: readonly string[] | ArrayLike<string>;
  series: readonly RadarSeriesInput[];
}

interface ContourInput {
  x: NumericArray;
  y: NumericArray;
  z: NumericArray;
  style?: ContourSeriesStyle;
}

interface PolarLineInput {
  r: NumericArray;
  theta: NumericArray;
  style?: StrokedSeriesStyle;
}

interface LineSeriesDefinition {
  kind: "line";
  style?: LineSeriesStyle;
  x: XSourceDefinition;
  y: YSourceDefinition;
}

interface ScatterSeriesDefinition {
  kind: "scatter";
  style?: ScatterSeriesStyle;
  x: XSourceDefinition;
  y: YSourceDefinition;
}

interface BarSeriesDefinition {
  kind: "bar";
  style?: BarSeriesStyle;
  categories: string[];
  values: NumericReactiveSourceDefinition;
}

interface HistogramSeriesDefinition {
  kind: "histogram";
  style?: HistogramSeriesStyle;
  data: NumericReactiveSourceDefinition;
}

interface BoxplotSeriesDefinition {
  kind: "boxplot";
  style?: BoxplotSeriesStyle;
  data: NumericReactiveSourceDefinition;
}

interface ErrorBarsSeriesDefinition {
  kind: "error-bars";
  style?: StrokedSeriesStyle;
  x: NumericReactiveSourceDefinition;
  y: NumericReactiveSourceDefinition;
  yErrors: NumericReactiveSourceDefinition;
}

interface ErrorBarsXYSeriesDefinition {
  kind: "error-bars-xy";
  style?: StrokedSeriesStyle;
  x: NumericReactiveSourceDefinition;
  y: NumericReactiveSourceDefinition;
  xErrors: NumericReactiveSourceDefinition;
  yErrors: NumericReactiveSourceDefinition;
}

type PlotSeriesDefinition =
  | LineSeriesDefinition
  | ScatterSeriesDefinition
  | BarSeriesDefinition
  | HistogramSeriesDefinition
  | BoxplotSeriesDefinition
  | HeatmapSeriesSnapshot
  | ErrorBarsSeriesDefinition
  | ErrorBarsXYSeriesDefinition
  | KdeSeriesSnapshot
  | EcdfSeriesSnapshot
  | ContourSeriesSnapshot
  | PieSeriesSnapshot
  | RadarSeriesSnapshot
  | ViolinSeriesSnapshot
  | PolarLineSeriesSnapshot;

interface PlotState {
  sizePx?: [number, number];
  sizeIn?: [number, number];
  dpi?: number;
  fontSize?: number;
  titleSize?: number;
  fontFamily?: string;
  scaleTypography?: number;
  lineWidthPt?: number;
  margin?: number;
  tightLayoutPad?: number;
  scientificNotation?: boolean;
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
  /** Present (and true) only when the axis was flipped high-to-low. */
  invertX?: boolean;
  invertY?: boolean;
  xScale?: AxisScaleSnapshot;
  yScale?: AxisScaleSnapshot;
  /** Plot-level annotations in call order; absent until the first one is added. */
  annotations?: PlotAnnotationSnapshot[];
  series: PlotSeriesDefinition[];
}

interface NormalizedCanvasSessionOptions {
  backendPreference: BackendPreference;
  autoResize: boolean;
  bindInput: boolean;
  initialTime?: number;
}

interface NormalizedWorkerSessionOptions extends NormalizedCanvasSessionOptions {
  fallbackToMainThread: boolean;
}

interface WorkerEnvelope {
  type: string;
  requestId?: number;
  payload?: unknown;
  canvas?: OffscreenCanvas;
  width?: number;
  height?: number;
  scaleFactor?: number;
  backendPreference?: BackendPreference;
  initialTime?: number;
}

interface RequestHandlers {
  resolve: (value: unknown) => void;
  reject: (reason: unknown) => void;
}

let rawModulePromise: Promise<RawModule> | null = null;

function readBooleanProperty(record: unknown, snakeName: string, camelName: string): boolean {
  if (!record || typeof record !== "object") {
    return false;
  }

  const snakeValue = (record as Record<string, unknown>)[snakeName];
  if (typeof snakeValue === "boolean") {
    return snakeValue;
  }

  const camelValue = (record as Record<string, unknown>)[camelName];
  return typeof camelValue === "boolean" ? camelValue : false;
}

async function ensureRawModule(): Promise<RawModule> {
  if (!rawModulePromise) {
    rawModulePromise = initRaw().then(() => {
      try {
        raw.register_default_browser_fonts_js();
      } catch {
        // A build without the embedded-font feature has no default face until
        // registerFont() supplies one — which needs this module first. Every
        // render entry point re-checks and reports the actionable error, so
        // module init must not fail here.
      }
      return raw;
    });
  }

  return rawModulePromise;
}

function normalizeCanvasSessionOptions(
  options: CanvasSessionOptions | undefined,
): NormalizedCanvasSessionOptions {
  const initialTime = options?.initialTime;

  return {
    backendPreference: normalizeBackendPreference(options?.backendPreference),
    autoResize: options?.autoResize ?? true,
    bindInput: options?.bindInput ?? true,
    initialTime: Number.isFinite(initialTime) ? initialTime : undefined,
  };
}

function normalizeWorkerSessionOptions(
  options: WorkerSessionOptions | undefined,
): NormalizedWorkerSessionOptions {
  const base = normalizeCanvasSessionOptions(options);

  return {
    ...base,
    fallbackToMainThread: options?.fallbackToMainThread ?? true,
  };
}

function getCanvasMetrics(canvas: HTMLCanvasElement): {
  width: number;
  height: number;
  scaleFactor: number;
  rect: DOMRect;
} {
  const rect = canvas.getBoundingClientRect();
  const scaleFactor = window.devicePixelRatio || 1;
  const width = Math.max(1, Math.round(rect.width * scaleFactor));
  const height = Math.max(1, Math.round(rect.height * scaleFactor));

  return { width, height, scaleFactor, rect };
}

function pointerPosition(
  canvas: HTMLCanvasElement,
  event: MouseEvent | PointerEvent | WheelEvent,
): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  const scaleX = rect.width === 0 ? 1 : canvas.width / rect.width;
  const scaleY = rect.height === 0 ? 1 : canvas.height / rect.height;

  return {
    x: (event.clientX - rect.left) * scaleX,
    y: (event.clientY - rect.top) * scaleY,
  };
}

function installCanvasResize(
  canvas: HTMLCanvasElement,
  onResize: (width: number, height: number, scaleFactor: number) => void,
): () => void {
  let lastWidth = 0;
  let lastHeight = 0;
  let lastScaleFactor = 0;

  const sync = () => {
    const { width, height, scaleFactor } = getCanvasMetrics(canvas);

    if (
      width === lastWidth &&
      height === lastHeight &&
      Math.abs(scaleFactor - lastScaleFactor) < 1e-6
    ) {
      return;
    }

    lastWidth = width;
    lastHeight = height;
    lastScaleFactor = scaleFactor;
    onResize(width, height, scaleFactor);
  };

  sync();

  const observer = new ResizeObserver(sync);
  observer.observe(canvas);
  window.addEventListener("resize", sync);

  return () => {
    observer.disconnect();
    window.removeEventListener("resize", sync);
  };
}

interface InputHandlers {
  pointerDown: (x: number, y: number, button: number) => void;
  pointerMove: (x: number, y: number) => void;
  pointerUp: (x: number, y: number, button: number) => void;
  pointerLeave: () => void;
  wheel: (deltaY: number, x: number, y: number) => void;
}

function attachCanvasEvents(canvas: HTMLCanvasElement, handlers: InputHandlers): () => void {
  const onContextMenu = (event: MouseEvent) => {
    event.preventDefault();
  };

  const onPointerDown = (event: PointerEvent) => {
    const point = pointerPosition(canvas, event);
    canvas.setPointerCapture(event.pointerId);
    handlers.pointerDown(point.x, point.y, event.button);
  };

  const onPointerMove = (event: PointerEvent) => {
    const point = pointerPosition(canvas, event);
    handlers.pointerMove(point.x, point.y);
  };

  const onPointerUp = (event: PointerEvent) => {
    const point = pointerPosition(canvas, event);
    if (canvas.hasPointerCapture(event.pointerId)) {
      canvas.releasePointerCapture(event.pointerId);
    }
    handlers.pointerUp(point.x, point.y, event.button);
  };

  const onPointerCancel = (event: PointerEvent) => {
    if (canvas.hasPointerCapture(event.pointerId)) {
      canvas.releasePointerCapture(event.pointerId);
    }
    handlers.pointerLeave();
  };

  const onPointerLeave = () => {
    handlers.pointerLeave();
  };

  const onWheel = (event: WheelEvent) => {
    event.preventDefault();
    const point = pointerPosition(canvas, event);
    handlers.wheel(event.deltaY, point.x, point.y);
  };

  canvas.addEventListener("contextmenu", onContextMenu);
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerCancel);
  canvas.addEventListener("pointerleave", onPointerLeave);
  canvas.addEventListener("wheel", onWheel, { passive: false });

  return () => {
    canvas.removeEventListener("contextmenu", onContextMenu);
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerCancel);
    canvas.removeEventListener("pointerleave", onPointerLeave);
    canvas.removeEventListener("wheel", onWheel);
  };
}

function workerCanvasSupported(canvas: HTMLCanvasElement): boolean {
  return (
    typeof window.Worker !== "undefined" &&
    typeof window.OffscreenCanvas !== "undefined" &&
    "transferControlToOffscreen" in canvas
  );
}

function toStringArray(values: readonly string[] | ArrayLike<string>): string[] {
  return Array.from(values, (value) => String(value));
}

function sourceLength(
  source: XSourceDefinition | YSourceDefinition | NumericReactiveSourceDefinition,
): number {
  switch (source.kind) {
    case "observable":
      return source.source.length;
    case "sine-signal":
      return source.source.length;
    case "static":
      return source.values.length;
  }
}

function normalizeXSource(source: NumericArray | ObservableSeries): XSourceDefinition {
  if (source instanceof ObservableSeries) {
    return { kind: "observable", source };
  }

  return { kind: "static", values: toNumberArray(source) };
}

function normalizeReactiveSource(
  source: NumericArray | ObservableSeries,
): NumericReactiveSourceDefinition {
  if (source instanceof ObservableSeries) {
    return { kind: "observable", source };
  }

  return { kind: "static", values: toNumberArray(source) };
}

function normalizeYSource(source: NumericArray | ObservableSeries | SineSignal): YSourceDefinition {
  if (source instanceof ObservableSeries) {
    return { kind: "observable", source };
  }

  if (source instanceof SineSignal) {
    return { kind: "sine-signal", source };
  }

  return { kind: "static", values: toNumberArray(source) };
}

async function ephemeralObservable(
  module: RawModule,
  source: NumericReactiveSourceDefinition,
): Promise<RawObservableVecF64> {
  if (source.kind === "observable") {
    return source.source._toRawObservable(module);
  }

  return new module.ObservableVecF64(Float64Array.from(source.values));
}

/** Copy the plot-level settings, keeping keys this build does not name. */
function clonePlotMetadata(state: PlotState | PlotSnapshot): Omit<PlotState, "series"> {
  return structuredClone(clonePlotMetadataUnchecked(state));
}

/** Split the plot-level settings off a snapshot that is already owned. */
function clonePlotMetadataUnchecked(state: PlotState | PlotSnapshot): Omit<PlotState, "series"> {
  const { series: _series, ...metadata } = state;
  return metadata;
}

/** Reject the style values the wasm layer would only catch at render time. */
function normalizeSeriesStyle<T extends SeriesStyleSnapshot>(style: T | undefined): T | undefined {
  if (!style) {
    return undefined;
  }

  const {
    alpha,
    width,
    markerSize,
    bandwidth,
    bins,
    density,
    levels,
    color,
    marker,
    linestyle,
    orientation,
  } = style as SeriesStyleSnapshot;
  if (alpha !== undefined && !(alpha >= 0 && alpha <= 1)) {
    throw new RangeError("alpha must be between 0.0 and 1.0");
  }
  if (density !== undefined && typeof density !== "boolean") {
    throw new TypeError("density must be a bool");
  }

  if (color !== undefined) {
    validateColor(color);
  }
  if (marker !== undefined) {
    validateName(MARKER_NAMES, "marker", marker);
  }
  if (linestyle !== undefined) {
    validateName(LINE_STYLE_NAMES, "linestyle", linestyle);
  }
  if (orientation !== undefined) {
    validateName(BAR_ORIENTATION_NAMES, "orientation", orientation);
  }

  for (const [name, value] of [
    ["width", width],
    ["markerSize", markerSize],
    ["bandwidth", bandwidth],
  ] as const) {
    if (value !== undefined && !(Number.isFinite(value) && value > 0)) {
      throw new RangeError(`${name} must be a finite positive number`);
    }
  }

  for (const [name, value, minimum] of [
    ["bins", bins, 1],
    ["levels", levels, 2],
  ] as const) {
    if (value !== undefined && !(Number.isInteger(value) && value >= minimum)) {
      throw new RangeError(`${name} must be an integer >= ${minimum}`);
    }
  }

  return { ...style };
}

async function buildRawPlotFromState(state: PlotState, module: RawModule): Promise<RawJsPlot> {
  const rawPlot = new module.JsPlot();
  applySnapshotMetadata(rawPlot, state);

  for (const series of state.series) {
    switch (series.kind) {
      case "line":
      case "scatter": {
        if (series.y.kind === "sine-signal") {
          const signal = await series.y.source._toRawSignal(module);
          const xValues =
            series.x.kind === "observable"
              ? Float64Array.from(series.x.source.snapshotValues())
              : Float64Array.from(series.x.values);

          if (series.kind === "line") {
            rawPlot.line_signal(xValues, signal, series.style);
          } else {
            rawPlot.scatter_signal(xValues, signal, series.style);
          }
          break;
        }

        if (series.x.kind === "observable" || series.y.kind === "observable") {
          const xObservable = await ephemeralObservable(module, series.x);
          const yObservable = await ephemeralObservable(module, series.y);

          if (series.kind === "line") {
            rawPlot.line_observable(xObservable, yObservable, series.style);
          } else {
            rawPlot.scatter_observable(xObservable, yObservable, series.style);
          }
          break;
        }

        const xValues = Float64Array.from(series.x.values);
        const yValues = Float64Array.from(series.y.values);

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
            await ephemeralObservable(module, series.values),
            series.style,
          );
        } else {
          rawPlot.bar(series.categories, Float64Array.from(series.values.values), series.style);
        }
        break;
      }
      case "histogram": {
        if (series.data.kind === "observable") {
          rawPlot.histogram_observable(
            await ephemeralObservable(module, series.data),
            series.style,
          );
        } else {
          rawPlot.histogram(Float64Array.from(series.data.values), series.style);
        }
        break;
      }
      case "boxplot": {
        if (series.data.kind === "observable") {
          rawPlot.boxplot_observable(await ephemeralObservable(module, series.data), series.style);
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
            await ephemeralObservable(module, series.x),
            await ephemeralObservable(module, series.y),
            await ephemeralObservable(module, series.yErrors),
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
            await ephemeralObservable(module, series.x),
            await ephemeralObservable(module, series.y),
            await ephemeralObservable(module, series.xErrors),
            await ephemeralObservable(module, series.yErrors),
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

/** Mutable numeric data source for reactive plot updates. */
export class ObservableSeries {
  #values: number[];
  #rawHandle: RawObservableVecF64 | null;

  constructor(values: NumericArray) {
    this.#values = toNumberArray(values);
    this.#rawHandle = null;
  }

  get length(): number {
    return this.#values.length;
  }

  replace(values: NumericArray): void {
    const nextValues = toNumberArray(values);
    this.#values = nextValues;

    if (this.#rawHandle) {
      this.#rawHandle.replace(Float64Array.from(nextValues));
    }
  }

  setAt(index: number, value: number): void {
    if (!Number.isInteger(index) || index < 0 || index >= this.#values.length) {
      throw new RangeError("observable index is out of bounds");
    }

    this.#values[index] = value;
    this.#rawHandle?.set_at(index, value);
  }

  values(): Float64Array {
    return Float64Array.from(this.#values);
  }

  snapshotValues(): number[] {
    return [...this.#values];
  }

  async _toRawObservable(module?: RawModule): Promise<RawObservableVecF64> {
    const currentModule = module ?? (await ensureRawModule());
    if (!this.#rawHandle) {
      this.#rawHandle = new currentModule.ObservableVecF64(
        Float64Array.from(this.snapshotValues()),
      );
    }
    return this.#rawHandle;
  }
}

/** Procedural sine-wave signal for temporal playback in interactive sessions. */
export class SineSignal {
  readonly options: NormalizedSineSignalOptions;
  #rawHandle: RawSignalVecF64 | null;

  constructor(options: SineSignalOptions) {
    this.options = normalizeSineSignalOptions(options);
    this.#rawHandle = null;
  }

  get length(): number {
    return this.options.points;
  }

  valuesAt(timeSeconds: number): Float64Array {
    const {
      amplitude,
      cycles,
      domainEnd,
      domainStart,
      phaseOffset,
      phaseVelocity,
      points,
      verticalOffset,
    } = this.options;
    const span = domainEnd - domainStart;
    const phase = phaseOffset + phaseVelocity * timeSeconds;
    const denominator = points - 1;

    return Float64Array.from(
      Array.from({ length: points }, (_, index) => {
        const progress = index / denominator;
        const x = domainStart + span * progress;
        return verticalOffset + amplitude * Math.sin(cycles * x + phase);
      }),
    );
  }

  async _toRawSignal(module?: RawModule): Promise<RawSignalVecF64> {
    const currentModule = module ?? (await ensureRawModule());
    if (!this.#rawHandle) {
      const {
        amplitude,
        cycles,
        domainEnd,
        domainStart,
        phaseOffset,
        phaseVelocity,
        points,
        verticalOffset,
      } = this.options;
      this.#rawHandle = currentModule.SignalVecF64.sineWave(
        points,
        domainStart,
        domainEnd,
        amplitude,
        cycles,
        phaseVelocity,
        phaseOffset,
        verticalOffset,
      );
    }
    return this.#rawHandle;
  }
}

function flattenHeatmapValues(input: ReadonlyArray<NumericArray>): {
  values: number[];
  rows: number;
  cols: number;
} {
  const rows = input.length;
  if (rows === 0) {
    throw new Error("heatmap input must contain at least one row");
  }

  const normalizedRows = input.map((row) => toNumberArray(row));
  const cols = normalizedRows[0]?.length ?? 0;
  if (cols === 0) {
    throw new Error("heatmap rows must contain at least one value");
  }

  if (normalizedRows.some((row) => row.length !== cols)) {
    throw new Error("heatmap rows must all have the same length");
  }

  return {
    values: normalizedRows.flat(),
    rows,
    cols,
  };
}

function defaultSaveFileName(format: "png" | "svg"): string {
  return format === "svg" ? "ruviz.svg" : "ruviz.png";
}

function downloadBlob(blob: Blob, fileName: string): void {
  const href = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = href;
  anchor.download = fileName;
  anchor.click();
  URL.revokeObjectURL(href);
}

/** Fluent plot builder for static export and interactive canvas mounting. */
export class PlotBuilder {
  #state: PlotState;
  #revision: number;
  #rawPlotCache: {
    module: RawModule;
    revision: number;
    rawPlot: RawJsPlot;
  } | null;
  #pngCache: {
    revision: number;
    bytes: Uint8Array;
  } | null;
  #svgCache: {
    revision: number;
    value: string;
  } | null;

  constructor(state?: PlotState) {
    this.#state = state
      ? {
          ...clonePlotMetadata(state),
          series: state.series.map((series) => PlotBuilder.#cloneSeries(series)),
        }
      : { series: [] };
    this.#revision = 0;
    this.#rawPlotCache = null;
    this.#pngCache = null;
    this.#svgCache = null;
  }

  static fromSnapshot(snapshot: PlotSnapshot): PlotBuilder {
    return new PlotBuilder(PlotBuilder.#stateFromSnapshot(snapshot));
  }

  static #stateFromSnapshot(input: PlotSnapshot): PlotState {
    // Snapshots arrive as caller-owned JSON, possibly written by a newer ruviz.
    // One deep clone up front is what keeps fields this build does not name --
    // including nested ones -- and guarantees no part of the builder's state
    // ever aliases the caller's object.
    const snapshot = clonePlotSnapshot(input);

    return {
      ...clonePlotMetadataUnchecked(snapshot),
      series: snapshot.series.map((series) => {
        switch (series.kind) {
          case "line":
          case "scatter":
            return {
              ...series,
              x: PlotBuilder.#sourceFromSnapshot(series.x),
              y: PlotBuilder.#ySourceFromSnapshot(series.y),
            };
          case "bar":
            return {
              ...series,
              categories: [...series.categories],
              values: PlotBuilder.#sourceFromSnapshot(series.values),
            };
          case "histogram":
          case "boxplot":
            return {
              ...series,
              data: PlotBuilder.#sourceFromSnapshot(series.data),
            };
          case "error-bars":
            return {
              ...series,
              x: PlotBuilder.#sourceFromSnapshot(series.x),
              y: PlotBuilder.#sourceFromSnapshot(series.y),
              yErrors: PlotBuilder.#sourceFromSnapshot(series.yErrors),
            };
          case "error-bars-xy":
            return {
              ...series,
              x: PlotBuilder.#sourceFromSnapshot(series.x),
              y: PlotBuilder.#sourceFromSnapshot(series.y),
              xErrors: PlotBuilder.#sourceFromSnapshot(series.xErrors),
              yErrors: PlotBuilder.#sourceFromSnapshot(series.yErrors),
            };
          default:
            return series as PlotSeriesDefinition;
        }
      }),
    };
  }

  static #sourceFromSnapshot(
    source: NumericReactiveSourceSnapshot | XSourceSnapshot,
  ): NumericReactiveSourceDefinition {
    // `extras` carries whatever a newer producer wrote alongside kind/values,
    // so serializing the builder back out reproduces the source it was given.
    const { kind: _kind, values, ...extras } = source;
    if (source.kind === "observable") {
      return { ...extras, kind: "observable", source: new ObservableSeries(values) };
    }

    return { ...extras, kind: "static", values: [...values] };
  }

  static #ySourceFromSnapshot(source: YSourceSnapshot): YSourceDefinition {
    if (source.kind === "sine-signal") {
      const { kind: _kind, options, ...extras } = source;
      return {
        ...extras,
        kind: "sine-signal",
        source: new SineSignal({
          points: options.points,
          domain: [options.domainStart, options.domainEnd],
          amplitude: options.amplitude,
          cycles: options.cycles,
          phaseVelocity: options.phaseVelocity,
          phaseOffset: options.phaseOffset,
          verticalOffset: options.verticalOffset,
        }),
      };
    }

    return PlotBuilder.#sourceFromSnapshot(source);
  }

  static #cloneSeries(series: PlotSeriesDefinition): PlotSeriesDefinition {
    switch (series.kind) {
      case "line":
      case "scatter":
        return {
          ...series,
          x: PlotBuilder.#cloneSource(series.x),
          y:
            series.y.kind === "static"
              ? { kind: "static", values: [...series.y.values] }
              : series.y,
        };
      case "bar":
        return {
          ...series,
          categories: [...series.categories],
          values: PlotBuilder.#cloneSource(series.values),
        };
      case "histogram":
      case "boxplot":
        return { ...series, data: PlotBuilder.#cloneSource(series.data) };
      case "error-bars":
        return {
          ...series,
          x: PlotBuilder.#cloneSource(series.x),
          y: PlotBuilder.#cloneSource(series.y),
          yErrors: PlotBuilder.#cloneSource(series.yErrors),
        };
      case "error-bars-xy":
        return {
          ...series,
          x: PlotBuilder.#cloneSource(series.x),
          y: PlotBuilder.#cloneSource(series.y),
          xErrors: PlotBuilder.#cloneSource(series.xErrors),
          yErrors: PlotBuilder.#cloneSource(series.yErrors),
        };
      default:
        return cloneSeriesSnapshot(series) as PlotSeriesDefinition;
    }
  }

  static #cloneSource(source: NumericReactiveSourceDefinition): NumericReactiveSourceDefinition {
    if (source.kind === "observable") {
      return { ...source };
    }

    const { kind: _kind, values, ...extras } = source;
    return { ...extras, kind: "static", values: [...values] };
  }

  sizePx(width: number, height: number): this {
    return this.setSizePx(width, height);
  }

  setSizePx(width: number, height: number): this {
    this.#state.sizePx = [Math.max(1, Math.round(width)), Math.max(1, Math.round(height))];
    this.#markDirty();
    return this;
  }

  /**
   * Applies figure-level presentation settings in one call.
   *
   * These are chosen as a set — a journal's column width, body point size and
   * rule weight are one decision — so they are grouped rather than spread over
   * individual chained setters. Omitted fields keep their current value.
   *
   * The call is atomic: every option is validated before any is applied, so a
   * rejected value leaves the plot unchanged.
   *
   * ```ts
   * plot.figure({ size: [3.25, 2.5], dpi: 300, fontSize: 9, fontFamily: "serif" });
   * ```
   */
  figure(options: FigureOptions): this {
    if (options === null || typeof options !== "object") {
      throw new TypeError("figure options must be an object");
    }

    const {
      size,
      dpi,
      fontSize,
      titleSize,
      fontFamily,
      scaleTypography,
      lineWidthPt,
      margin,
      tightLayoutPad,
      scientificNotation,
      maxResolution,
      ...unknown
    } = options;

    // A misspelled key would otherwise be silently dropped and yield a figure
    // that looks nearly right — the worst outcome for a print submission.
    const unknownKeys = Object.keys(unknown);
    if (unknownKeys.length > 0) {
      throw new TypeError(`unknown figure option(s): ${unknownKeys.join(", ")}`);
    }

    // Validate every option into `staged` before touching #state, so a
    // rejected value leaves the plot (and its render caches) unchanged.
    const staged: Partial<Omit<PlotState, "series">> = {};

    if (typeof size !== "undefined") {
      assertPair(size, "figure size");
      for (const [value, label] of [
        [size[0], "figure width"],
        [size[1], "figure height"],
      ] as const) {
        assertFinitePositive(value, label);
        // The core builder clamps sizes below 1.0 inch, which would silently
        // change both the physical size and the aspect ratio.
        if (value < 1) {
          throw new RangeError(`${label} must be a finite number of at least 1.0 inch`);
        }
      }
      staged.sizeIn = [size[0], size[1]];
    }

    if (typeof dpi !== "undefined") {
      // The core clamps dpi below 72; reject it instead.
      if (!(Number.isInteger(dpi) && dpi >= 72 && dpi <= 4294967295)) {
        throw new RangeError("figure dpi must be an integer between 72 and 4294967295");
      }
      staged.dpi = dpi;
    }

    if (typeof fontSize !== "undefined") {
      assertFinitePositive(fontSize, "font size");
      // The core clamps font sizes below 4pt; reject them instead.
      if (fontSize < 4) {
        throw new RangeError("font size must be at least 4 points");
      }
      staged.fontSize = fontSize;
    }

    if (typeof titleSize !== "undefined") {
      assertFinitePositive(titleSize, "title size");
      staged.titleSize = titleSize;
    }

    if (typeof fontFamily !== "undefined") {
      if (typeof fontFamily !== "string" || fontFamily.trim() === "") {
        throw new TypeError("font family must be a non-empty string");
      }
      staged.fontFamily = fontFamily;
    }

    if (typeof scaleTypography !== "undefined") {
      assertFinitePositive(scaleTypography, "typography scale");
      staged.scaleTypography = scaleTypography;
    }

    if (typeof lineWidthPt !== "undefined") {
      assertFinitePositive(lineWidthPt, "line width");
      // The core clamps line widths below 0.1pt; reject them instead.
      if (lineWidthPt < 0.1) {
        throw new RangeError("line width must be at least 0.1 points");
      }
      staged.lineWidthPt = lineWidthPt;
    }

    if (typeof margin !== "undefined") {
      // The core builder clamps to 0.0–0.5; reject out-of-range here instead
      // so 0.9 does not silently render as 0.5.
      if (!Number.isFinite(margin) || margin < 0 || margin > 0.5) {
        throw new RangeError("figure margin must be a fraction between 0.0 and 0.5");
      }
      staged.margin = margin;
    }

    if (typeof tightLayoutPad !== "undefined") {
      if (
        !Number.isFinite(tightLayoutPad) ||
        tightLayoutPad < 0 ||
        tightLayoutPad > 3.4028234663852886e38
      ) {
        throw new RangeError(
          "tight layout padding must be a finite, non-negative number of points",
        );
      }
      staged.tightLayoutPad = tightLayoutPad;
    }

    if (typeof scientificNotation !== "undefined") {
      if (typeof scientificNotation !== "boolean") {
        throw new TypeError("scientificNotation must be a boolean");
      }
      staged.scientificNotation = scientificNotation;
    }

    if (typeof maxResolution !== "undefined") {
      assertPair(maxResolution, "max resolution");
      for (const [value, label] of [
        [maxResolution[0], "max resolution width"],
        [maxResolution[1], "max resolution height"],
      ] as const) {
        if (!(Number.isInteger(value) && value > 0 && value <= 4294967295)) {
          throw new RangeError(`${label} must be an integer greater than zero, at most 4294967295`);
        }
      }
      staged.maxResolution = [maxResolution[0], maxResolution[1]];
    }

    Object.assign(this.#state, staged);
    this.#markDirty();
    return this;
  }

  /** Sets the output DPI, scaling the pixels exported from the figure size. */
  dpi(dpi: number): this {
    return this.setDpi(dpi);
  }

  setDpi(dpi: number): this {
    // The core clamps dpi below 72; reject it instead.
    if (!(Number.isInteger(dpi) && dpi >= 72 && dpi <= 4294967295)) {
      throw new RangeError("plot dpi must be an integer between 72 and 4294967295");
    }
    this.#state.dpi = dpi;
    this.#markDirty();
    return this;
  }

  theme(theme: PlotTheme): this {
    return this.setTheme(theme);
  }

  setTheme(theme: PlotTheme): this {
    validateName(PLOT_THEME_NAMES, "theme", theme);
    this.#state.theme = theme;
    this.#markDirty();
    return this;
  }

  ticks(enabled: boolean): this {
    return this.setTicks(enabled);
  }

  setTicks(enabled: boolean): this {
    this.#state.ticks = enabled;
    this.#markDirty();
    return this;
  }

  title(title: string): this {
    return this.setTitle(title);
  }

  setTitle(title: string): this {
    this.#state.title = title;
    this.#markDirty();
    return this;
  }

  xlabel(label: string): this {
    return this.setXLabel(label);
  }

  setXLabel(label: string): this {
    this.#state.xLabel = label;
    this.#markDirty();
    return this;
  }

  ylabel(label: string): this {
    return this.setYLabel(label);
  }

  setYLabel(label: string): this {
    this.#state.yLabel = label;
    this.#markDirty();
    return this;
  }

  legend(position: LegendPositionName = "best"): this {
    return this.setLegend(position);
  }

  setLegend(position: LegendPositionName = "best"): this {
    validateName(LEGEND_POSITION_NAMES, "legend position", position);
    this.#state.legend = position;
    this.#markDirty();
    return this;
  }

  grid(enabled = true): this {
    return this.setGrid(enabled);
  }

  setGrid(enabled = true): this {
    this.#state.grid = enabled;
    this.#markDirty();
    return this;
  }

  xlim(minimum: number, maximum: number): this {
    return this.setXLim(minimum, maximum);
  }

  setXLim(minimum: number, maximum: number): this {
    this.#state.xLim = PlotBuilder.#limits("x", minimum, maximum);
    this.#markDirty();
    return this;
  }

  ylim(minimum: number, maximum: number): this {
    return this.setYLim(minimum, maximum);
  }

  setYLim(minimum: number, maximum: number): this {
    this.#state.yLim = PlotBuilder.#limits("y", minimum, maximum);
    this.#markDirty();
    return this;
  }

  /** Flip the x axis after its range resolves, so it runs high-to-low. */
  invertX(): this {
    this.#state.invertX = true;
    this.#markDirty();
    return this;
  }

  /**
   * Flip the y axis after its range resolves.
   *
   * On a horizontal categorical chart this puts the first category at the
   * top, so ranked bars read in the order they were given.
   */
  invertY(): this {
    this.#state.invertY = true;
    this.#markDirty();
    return this;
  }

  xscale(scale: AxisScaleName, linthresh?: number): this {
    return this.setXScale(scale, linthresh);
  }

  setXScale(scale: AxisScaleName, linthresh?: number): this {
    this.#state.xScale = PlotBuilder.#scale(scale, linthresh);
    this.#markDirty();
    return this;
  }

  yscale(scale: AxisScaleName, linthresh?: number): this {
    return this.setYScale(scale, linthresh);
  }

  setYScale(scale: AxisScaleName, linthresh?: number): this {
    this.#state.yScale = PlotBuilder.#scale(scale, linthresh);
    this.#markDirty();
    return this;
  }

  /** Inverted bounds are kept: they render a descending axis. */
  static #limits(axis: "x" | "y", minimum: number, maximum: number): [number, number] {
    if (!Number.isFinite(minimum) || !Number.isFinite(maximum) || minimum === maximum) {
      throw new RangeError(`${axis} limits must be finite and different`);
    }
    return [minimum, maximum];
  }

  static #scale(scale: AxisScaleName, linthresh: number | undefined): AxisScaleSnapshot {
    validateName(AXIS_SCALE_NAMES, "axis scale", scale);
    if (scale !== "symlog") {
      if (linthresh !== undefined) {
        throw new RangeError("linthresh only applies to the symlog scale");
      }
      return [scale];
    }

    const threshold = linthresh ?? 1;
    if (!(Number.isFinite(threshold) && threshold > 0)) {
      throw new RangeError("symlog linthresh must be a finite positive number");
    }
    return [scale, threshold];
  }

  /**
   * Adds a vertical reference line spanning the plot height at data
   * x-coordinate `x` — an absorption edge, a threshold, a boundary. Without a
   * style it renders as the core default, a 1pt dashed gray.
   */
  vline(x: number, style?: ReferenceLineStyle): this {
    PlotBuilder.#finiteCoordinate(x, "vline x");
    this.#pushAnnotation({
      kind: "vline",
      x,
      ...PlotBuilder.#referenceLineStyle(style, "vline"),
    });
    return this;
  }

  /**
   * Adds a horizontal reference line spanning the plot width at data
   * y-coordinate `y`. Without a style it renders as the core default, a 1pt
   * dashed gray.
   */
  hline(y: number, style?: ReferenceLineStyle): this {
    PlotBuilder.#finiteCoordinate(y, "hline y");
    this.#pushAnnotation({
      kind: "hline",
      y,
      ...PlotBuilder.#referenceLineStyle(style, "hline"),
    });
    return this;
  }

  /**
   * Adds a text annotation at data coordinates — a reference-line label, a
   * peak marker. The default is 10pt black, so pass a `color` when the plot
   * uses a dark theme.
   */
  annotateText(x: number, y: number, text: string, style?: TextAnnotationStyle): this {
    PlotBuilder.#finiteCoordinate(x, "annotation x");
    PlotBuilder.#finiteCoordinate(y, "annotation y");
    if (typeof text !== "string") {
      throw new TypeError("annotation text must be a string");
    }
    if (text === "") {
      throw new RangeError("annotation text must be a non-empty string");
    }

    this.#pushAnnotation({
      kind: "text",
      x,
      y,
      text,
      ...PlotBuilder.#textAnnotationStyle(style),
    });
    return this;
  }

  #pushAnnotation(annotation: PlotAnnotationSnapshot): void {
    (this.#state.annotations ??= []).push(annotation);
    this.#markDirty();
  }

  static #finiteCoordinate(value: number, label: string): void {
    if (!Number.isFinite(value)) {
      throw new RangeError(`${label} must be a finite number`);
    }
  }

  /**
   * A spreadable `{style}` holding only the defined entries, or nothing when
   * every entry was undefined — so an empty style never reaches the
   * snapshot, matching the Python binding's output exactly.
   */
  static #definedStyle<T extends object>(style: T): { style: T } | undefined {
    const cleaned = Object.fromEntries(
      Object.entries(style).filter(([, value]) => value !== undefined),
    ) as T;
    return Object.keys(cleaned).length > 0 ? { style: cleaned } : undefined;
  }

  /** Validate a reference-line style, returning a spreadable `{style}` or nothing. */
  static #referenceLineStyle(
    style: ReferenceLineStyle | undefined,
    kind: string,
  ): { style: ReferenceLineStyle } | undefined {
    // `== null` also accepts null from untyped callers, like every other
    // style parameter in this package.
    if (style == null) {
      return undefined;
    }

    const { color, width, linestyle, ...unknown } = style;
    const unknownKeys = Object.keys(unknown);
    if (unknownKeys.length > 0) {
      throw new TypeError(`unknown ${kind} style option(s): ${unknownKeys.join(", ")}`);
    }
    if (color !== undefined) {
      validateColor(color);
    }
    if (width !== undefined) {
      assertFinitePositive(width, `${kind} width`);
    }
    if (linestyle !== undefined) {
      validateName(LINE_STYLE_NAMES, "linestyle", linestyle);
    }
    return PlotBuilder.#definedStyle({ color, width, linestyle });
  }

  /** Validate a text-annotation style, returning a spreadable `{style}` or nothing. */
  static #textAnnotationStyle(
    style: TextAnnotationStyle | undefined,
  ): { style: TextAnnotationStyle } | undefined {
    if (style == null) {
      return undefined;
    }

    const { color, fontSize, ...unknown } = style;
    const unknownKeys = Object.keys(unknown);
    if (unknownKeys.length > 0) {
      throw new TypeError(`unknown annotation style option(s): ${unknownKeys.join(", ")}`);
    }
    if (color !== undefined) {
      validateColor(color);
    }
    if (fontSize !== undefined) {
      assertFinitePositive(fontSize, "annotation fontSize");
    }
    return PlotBuilder.#definedStyle({ color, fontSize });
  }

  line(input: LineSeriesInput): this {
    return this.#addXYSeries("line", input);
  }

  addLine(input: LineSeriesInput): this {
    return this.line(input);
  }

  scatter(input: ScatterSeriesInput): this {
    return this.#addXYSeries("scatter", input);
  }

  addScatter(input: ScatterSeriesInput): this {
    return this.scatter(input);
  }

  bar(input: BarSeriesInput): this {
    const categories = toStringArray(input.categories);
    const values = normalizeReactiveSource(input.values);
    if (categories.length !== sourceLength(values)) {
      throw new Error("bar categories and values must have the same length");
    }
    this.#state.series.push({
      kind: "bar",
      style: normalizeSeriesStyle(input.style),
      categories,
      values,
    });
    this.#markDirty();
    return this;
  }

  histogram(input: NumericArray | ObservableSeries, style?: HistogramSeriesStyle): this {
    const values = normalizeReactiveSource(input);
    this.#state.series.push({
      kind: "histogram",
      style: normalizeSeriesStyle(style),
      data: values,
    });
    this.#markDirty();
    return this;
  }

  boxplot(input: NumericArray | ObservableSeries, style?: BoxplotSeriesStyle): this {
    const values = normalizeReactiveSource(input);
    this.#state.series.push({ kind: "boxplot", style: normalizeSeriesStyle(style), data: values });
    this.#markDirty();
    return this;
  }

  heatmap(input: ReadonlyArray<NumericArray>): this {
    const { values, rows, cols } = flattenHeatmapValues(input);
    this.#state.series.push({ kind: "heatmap", values, rows, cols });
    this.#markDirty();
    return this;
  }

  errorBars(input: ErrorBarsInput): this {
    const x = normalizeReactiveSource(input.x);
    const y = normalizeReactiveSource(input.y);
    const yErrors = normalizeReactiveSource(input.yErrors);
    if (sourceLength(x) !== sourceLength(y) || sourceLength(x) !== sourceLength(yErrors)) {
      throw new Error("x, y, and yErrors must have the same length");
    }
    this.#state.series.push({
      kind: "error-bars",
      style: normalizeSeriesStyle(input.style),
      x,
      y,
      yErrors,
    });
    this.#markDirty();
    return this;
  }

  errorBarsXY(input: ErrorBarsXYInput): this {
    const x = normalizeReactiveSource(input.x);
    const y = normalizeReactiveSource(input.y);
    const xErrors = normalizeReactiveSource(input.xErrors);
    const yErrors = normalizeReactiveSource(input.yErrors);
    const expected = sourceLength(x);
    if (
      expected !== sourceLength(y) ||
      expected !== sourceLength(xErrors) ||
      expected !== sourceLength(yErrors)
    ) {
      throw new Error("x, y, xErrors, and yErrors must have the same length");
    }
    this.#state.series.push({
      kind: "error-bars-xy",
      style: normalizeSeriesStyle(input.style),
      x,
      y,
      xErrors,
      yErrors,
    });
    this.#markDirty();
    return this;
  }

  kde(input: NumericArray, style?: KdeSeriesStyle): this {
    this.#state.series.push({
      kind: "kde",
      style: normalizeSeriesStyle(style),
      data: toNumberArray(input),
    });
    this.#markDirty();
    return this;
  }

  ecdf(input: NumericArray, style?: StrokedSeriesStyle): this {
    this.#state.series.push({
      kind: "ecdf",
      style: normalizeSeriesStyle(style),
      data: toNumberArray(input),
    });
    this.#markDirty();
    return this;
  }

  contour(input: ContourInput): this {
    const x = toNumberArray(input.x);
    const y = toNumberArray(input.y);
    const z = toNumberArray(input.z);
    if (x.length === 0 || y.length === 0 || z.length !== x.length * y.length) {
      throw new Error("contour z must contain x.length * y.length values");
    }
    this.#state.series.push({ kind: "contour", style: normalizeSeriesStyle(input.style), x, y, z });
    this.#markDirty();
    return this;
  }

  pie(values: NumericArray, labelsInput?: readonly string[] | ArrayLike<string>): this {
    const normalizedValues = toNumberArray(values);
    const labels = labelsInput ? toStringArray(labelsInput) : undefined;
    if (labels && labels.length !== normalizedValues.length) {
      throw new Error("pie values and labels must have the same length");
    }
    this.#state.series.push({ kind: "pie", values: normalizedValues, labels });
    this.#markDirty();
    return this;
  }

  radar(input: RadarInput): this {
    const labels = toStringArray(input.labels);
    if (labels.length === 0) {
      throw new Error("radar labels must not be empty");
    }

    const series = input.series.map((item) => {
      const values = toNumberArray(item.values);
      if (values.length !== labels.length) {
        throw new Error("each radar series must match the labels length");
      }
      return { name: item.name, values };
    });

    if (series.length === 0) {
      throw new Error("radar input must contain at least one series");
    }

    this.#state.series.push({ kind: "radar", labels, series });
    this.#markDirty();
    return this;
  }

  violin(input: NumericArray, style?: StrokedSeriesStyle): this {
    this.#state.series.push({
      kind: "violin",
      style: normalizeSeriesStyle(style),
      data: toNumberArray(input),
    });
    this.#markDirty();
    return this;
  }

  polarLine(input: PolarLineInput): this {
    const r = toNumberArray(input.r);
    const theta = toNumberArray(input.theta);
    if (r.length !== theta.length) {
      throw new Error("polar r and theta must have the same length");
    }
    this.#state.series.push({
      kind: "polar-line",
      style: normalizeSeriesStyle(input.style),
      r,
      theta,
    });
    this.#markDirty();
    return this;
  }

  clone(): PlotBuilder {
    return PlotBuilder.fromSnapshot(this.toSnapshot());
  }

  dispose(): void {
    if (this.#rawPlotCache) {
      this.#rawPlotCache.rawPlot.free();
      this.#rawPlotCache = null;
    }
    this.#pngCache = null;
    this.#svgCache = null;
  }

  toSnapshot(): PlotSnapshot {
    const { series, ...metadata } = this.#state;

    return clonePlotSnapshot({
      schemaVersion: SNAPSHOT_SCHEMA_VERSION,
      ...metadata,
      series: series.map((item) => this.#serializeSeries(item)),
    });
  }

  async renderPng(): Promise<Uint8Array> {
    const pngCache = this.#pngCache;
    if (pngCache && pngCache.revision === this.#revision) {
      return new Uint8Array(pngCache.bytes);
    }

    const module = await ensureRawModule();
    const rawPlot = await this._toRawPlot(module);
    const png = rawPlot.render_png_bytes();
    if (this.#canCacheStaticExports()) {
      this.#pngCache = {
        revision: this.#revision,
        bytes: new Uint8Array(png),
      };
    }
    return png;
  }

  async renderSvg(): Promise<string> {
    const svgCache = this.#svgCache;
    if (svgCache && svgCache.revision === this.#revision) {
      return svgCache.value;
    }

    const module = await ensureRawModule();
    const rawPlot = await this._toRawPlot(module);
    const svg = rawPlot.render_svg();
    if (this.#canCacheStaticExports()) {
      this.#svgCache = {
        revision: this.#revision,
        value: svg,
      };
    }
    return svg;
  }

  async save(options?: PlotSaveOptions): Promise<void> {
    const format = options?.format ?? "png";
    const fileName = options?.fileName ?? defaultSaveFileName(format);
    if (format === "svg") {
      const svg = await this.renderSvg();
      downloadBlob(new Blob([svg], { type: "image/svg+xml" }), fileName);
      return;
    }

    const png = await this.renderPng();
    const blobBytes = new Uint8Array(png);
    downloadBlob(new Blob([blobBytes.buffer as ArrayBuffer], { type: "image/png" }), fileName);
  }

  async mount(canvas: HTMLCanvasElement, options?: CanvasSessionOptions): Promise<CanvasSession> {
    const session = await createCanvasSession(canvas, options);
    await session.setPlot(this);
    session.render();
    return session;
  }

  async mountWorker(
    canvas: HTMLCanvasElement,
    options?: WorkerSessionOptions,
  ): Promise<WorkerSession> {
    const session = await createWorkerSession(canvas, options);
    await session.setPlot(this);
    session.render();
    return session;
  }

  async _toRawPlot(module?: RawModule): Promise<RawJsPlot> {
    const currentModule = module ?? (await ensureRawModule());
    const cached = this.#rawPlotCache;
    if (cached && cached.module === currentModule && cached.revision === this.#revision) {
      return cached.rawPlot;
    }

    const rawPlot = await buildRawPlotFromState(this.#state, currentModule);
    if (this.#rawPlotCache) {
      this.#rawPlotCache.rawPlot.free();
    }
    this.#rawPlotCache = {
      module: currentModule,
      revision: this.#revision,
      rawPlot,
    };
    return rawPlot;
  }

  #addXYSeries(kind: "line" | "scatter", input: LineSeriesInput | ScatterSeriesInput): this {
    const x = normalizeXSource(input.x);
    const y = normalizeYSource(input.y);
    if (sourceLength(x) !== sourceLength(y)) {
      throw new Error("x and y must have the same length");
    }
    this.#state.series.push({
      kind,
      style: normalizeSeriesStyle(input.style),
      x,
      y,
    } as LineSeriesDefinition | ScatterSeriesDefinition);
    this.#markDirty();
    return this;
  }

  _revision(): number {
    return this.#revision;
  }

  #markDirty(): void {
    this.#revision += 1;
    if (this.#rawPlotCache) {
      this.#rawPlotCache.rawPlot.free();
      this.#rawPlotCache = null;
    }
    this.#pngCache = null;
    this.#svgCache = null;
  }

  #canCacheStaticExports(): boolean {
    return this.#state.series.every((series) => !PlotBuilder.#seriesHasObservableSource(series));
  }

  static #seriesHasObservableSource(series: PlotSeriesDefinition): boolean {
    switch (series.kind) {
      case "line":
      case "scatter":
        return series.x.kind === "observable" || series.y.kind === "observable";
      case "bar":
        return series.values.kind === "observable";
      case "histogram":
      case "boxplot":
        return series.data.kind === "observable";
      case "error-bars":
        return (
          series.x.kind === "observable" ||
          series.y.kind === "observable" ||
          series.yErrors.kind === "observable"
        );
      case "error-bars-xy":
        return (
          series.x.kind === "observable" ||
          series.y.kind === "observable" ||
          series.xErrors.kind === "observable" ||
          series.yErrors.kind === "observable"
        );
      default:
        return false;
    }
  }

  #serializeSeries(series: PlotSeriesDefinition): PlotSeriesSnapshot {
    switch (series.kind) {
      case "line":
      case "scatter":
        return {
          ...series,
          x: this.#serializeXSource(series.x),
          y: this.#serializeYSource(series.y),
        };
      case "bar":
        return {
          ...series,
          categories: [...series.categories],
          values: this.#serializeReactiveSource(series.values),
        };
      case "histogram":
      case "boxplot":
        return { ...series, data: this.#serializeReactiveSource(series.data) };
      case "error-bars":
        return {
          ...series,
          x: this.#serializeReactiveSource(series.x),
          y: this.#serializeReactiveSource(series.y),
          yErrors: this.#serializeReactiveSource(series.yErrors),
        };
      case "error-bars-xy":
        return {
          ...series,
          x: this.#serializeReactiveSource(series.x),
          y: this.#serializeReactiveSource(series.y),
          xErrors: this.#serializeReactiveSource(series.xErrors),
          yErrors: this.#serializeReactiveSource(series.yErrors),
        };
      default:
        return cloneSeriesSnapshot(series);
    }
  }

  #serializeXSource(source: XSourceDefinition): XSourceSnapshot {
    return this.#serializeReactiveSource(source);
  }

  /** Serialize a source, carrying back the fields this build does not name. */
  #serializeReactiveSource(source: NumericReactiveSourceDefinition): NumericReactiveSourceSnapshot {
    if (source.kind === "observable") {
      const { kind: _kind, source: observable, ...extras } = source;
      return { ...extras, kind: "observable", values: observable.snapshotValues() };
    }

    const { kind: _kind, values, ...extras } = source;
    return { ...extras, kind: "static", values: [...values] };
  }

  #serializeYSource(source: YSourceDefinition): YSourceSnapshot {
    if (source.kind === "sine-signal") {
      const { kind: _kind, source: signal, ...extras } = source;
      return { ...extras, kind: "sine-signal", options: { ...signal.options } };
    }

    return this.#serializeReactiveSource(source);
  }
}

/** Main-thread interactive canvas session. */
export class CanvasSession {
  readonly mode: SessionMode = "main-thread";

  #module: RawModule;
  #rawSession: RawWebCanvasSession;
  #canvas: HTMLCanvasElement;
  #cleanup: Array<() => void>;
  #attachedPlot: PlotBuilder | null;
  #attachedRevision: number | null;

  constructor(module: RawModule, rawSession: RawWebCanvasSession, canvas: HTMLCanvasElement) {
    this.#module = module;
    this.#rawSession = rawSession;
    this.#canvas = canvas;
    this.#cleanup = [];
    this.#attachedPlot = null;
    this.#attachedRevision = null;
  }

  hasPlot(): boolean {
    return this.#rawSession.has_plot();
  }

  #withAttachedPlot(fn: () => void): void {
    if (!this.hasPlot()) {
      return;
    }

    fn();
  }

  #withAttachedPlotResult<T>(fn: () => T): T | null {
    if (!this.hasPlot()) {
      return null;
    }

    return fn();
  }

  async setPlot(plot: PlotBuilder): Promise<void> {
    const revision = plot._revision();
    if (this.#attachedPlot === plot && this.#attachedRevision === revision) {
      return;
    }
    const rawPlot = await plot._toRawPlot(this.#module);
    this.#rawSession.set_plot(rawPlot);
    this.#attachedPlot = plot;
    this.#attachedRevision = revision;
  }

  resize(width?: number, height?: number, scaleFactor?: number): void {
    const nextMetrics =
      typeof width === "number" && typeof height === "number" && typeof scaleFactor === "number"
        ? { width, height, scaleFactor }
        : getCanvasMetrics(this.#canvas);

    this.#rawSession.resize(nextMetrics.width, nextMetrics.height, nextMetrics.scaleFactor);
  }

  setTime(timeSeconds: number): void {
    this.#rawSession.set_time(timeSeconds);
  }

  setBackendPreference(backendPreference: BackendPreference): void {
    this.#rawSession.set_backend_preference(
      toRawBackendPreference(this.#module, normalizeBackendPreference(backendPreference)),
    );
  }

  render(): void {
    this.#withAttachedPlot(() => {
      this.#rawSession.render();
    });
  }

  resetView(): void {
    this.#withAttachedPlot(() => {
      this.#rawSession.reset_view();
    });
  }

  pointerDown(x: number, y: number, button: number): void {
    this.#withAttachedPlot(() => {
      this.#rawSession.pointer_down(x, y, button);
    });
  }

  pointerMove(x: number, y: number): void {
    this.#withAttachedPlot(() => {
      this.#rawSession.pointer_move(x, y);
    });
  }

  pointerUp(x: number, y: number, button: number): void {
    this.#withAttachedPlot(() => {
      this.#rawSession.pointer_up(x, y, button);
    });
  }

  pointerLeave(): void {
    this.#withAttachedPlot(() => {
      this.#rawSession.pointer_leave();
    });
  }

  wheel(deltaY: number, x: number, y: number): void {
    this.#withAttachedPlot(() => {
      this.#rawSession.wheel(deltaY, x, y);
    });
  }

  /** The series data point under a canvas pixel, or `null`. */
  hitAt(x: number, y: number): SeriesHit | null {
    return this.#withAttachedPlotResult(() => {
      const raw = this.#rawSession.hit_at(x, y);
      if (raw === null || raw === undefined) {
        return null;
      }
      const hit: SeriesHit = {
        seriesIndex: raw.series_index,
        seriesLabel: raw.series_label ?? null,
        pointIndex: raw.point_index,
        dataX: raw.data_x,
        dataY: raw.data_y,
        distancePx: raw.distance_px,
      };
      raw.free();
      return hit;
    });
  }

  /** The index of the series behind a legend entry at a canvas pixel, or `null`. */
  legendEntryAt(x: number, y: number): number | null {
    const index = this.#withAttachedPlotResult(() => this.#rawSession.legend_entry_at(x, y));
    return index === null || index < 0 ? null : index;
  }

  seriesCount(): number {
    return this.#withAttachedPlotResult(() => this.#rawSession.series_count()) ?? 0;
  }

  seriesLabel(seriesIndex: number): string | null {
    return this.#withAttachedPlotResult(() => this.#rawSession.series_label(seriesIndex) ?? null);
  }

  seriesVisible(seriesIndex: number): boolean {
    return this.#withAttachedPlotResult(() => this.#rawSession.series_visible(seriesIndex)) ?? true;
  }

  /**
   * Show or hide a series and re-render; the legend entry stays, dimmed.
   * A grouped series toggles with its group. Returns `false` when the index
   * is out of range or no plot is attached.
   */
  setSeriesVisible(seriesIndex: number, visible: boolean): boolean {
    return (
      this.#withAttachedPlotResult(() =>
        this.#rawSession.set_series_visible(seriesIndex, visible),
      ) ?? false
    );
  }

  async exportPng(): Promise<Uint8Array> {
    const result = this.#withAttachedPlotResult(() => this.#rawSession.export_png());
    if (result === null) {
      throw new Error("no plot is attached to this session");
    }
    return result;
  }

  async exportSvg(): Promise<string> {
    const result = this.#withAttachedPlotResult(() => this.#rawSession.export_svg());
    if (result === null) {
      throw new Error("no plot is attached to this session");
    }
    return result;
  }

  destroy(): void {
    this.#rawSession.destroy();
    this.#attachedPlot = null;
    this.#attachedRevision = null;
  }

  dispose(): void {
    for (const dispose of this.#cleanup.splice(0)) {
      dispose();
    }
    this.#rawSession.destroy();
    this.#attachedPlot = null;
    this.#attachedRevision = null;
  }

  _pushCleanup(dispose: () => void): void {
    this.#cleanup.push(dispose);
  }
}

/** Worker-backed interactive canvas session with main-thread fallback support. */
export class WorkerSession {
  readonly mode: SessionMode;

  #canvas: HTMLCanvasElement;
  #worker: Worker | null;
  #fallbackSession: CanvasSession | null;
  #cleanup: Array<() => void>;
  #pendingRequests: Map<number, RequestHandlers>;
  #nextRequestId: number;
  #readyPromise: Promise<void>;
  #resolveReady: (() => void) | null;
  #rejectReady: ((reason: unknown) => void) | null;
  #hasPlot: boolean;
  #sessionEpoch: number;
  #attachedPlot: PlotBuilder | null;
  #attachedRevision: number | null;

  constructor(canvas: HTMLCanvasElement, mode: SessionMode, fallbackSession?: CanvasSession) {
    this.mode = mode;
    this.#canvas = canvas;
    this.#worker = null;
    this.#fallbackSession = fallbackSession ?? null;
    this.#cleanup = [];
    this.#pendingRequests = new Map();
    this.#nextRequestId = 1;
    this.#hasPlot = false;
    this.#sessionEpoch = 0;
    this.#attachedPlot = null;
    this.#attachedRevision = null;
    this.#resolveReady = null;
    this.#rejectReady = null;
    if (this.#fallbackSession) {
      this.#readyPromise = Promise.resolve();
    } else {
      this.#readyPromise = new Promise((resolve, reject) => {
        this.#resolveReady = resolve;
        this.#rejectReady = reject;
      });
    }
  }

  get canvas(): HTMLCanvasElement {
    return this.#canvas;
  }

  attachWorker(worker: Worker): void {
    this.#worker = worker;
    worker.onmessage = (event) => this.#handleMessage(event);
    worker.onerror = (event) => {
      this.#failAll(new Error(event.message || "worker session error"));
    };
    worker.onmessageerror = () => {
      this.#failAll(new Error("worker session failed to decode a worker message"));
    };
  }

  hasPlot(): boolean {
    return this.#fallbackSession ? this.#fallbackSession.hasPlot() : this.#hasPlot;
  }

  #canDispatchPlotCommand(): boolean {
    return this.#fallbackSession ? this.#fallbackSession.hasPlot() : this.#hasPlot;
  }

  async setPlot(plot: PlotBuilder): Promise<void> {
    if (this.#fallbackSession) {
      await this.#fallbackSession.setPlot(plot);
      return;
    }

    const revision = plot._revision();
    if (this.#attachedPlot === plot && this.#attachedRevision === revision) {
      return;
    }

    const sessionEpoch = this.#sessionEpoch;

    try {
      await this.#request("setPlot", { snapshot: plot.toSnapshot() });
      if (sessionEpoch !== this.#sessionEpoch || !this.#worker) {
        return;
      }
      this.#hasPlot = true;
      this.#attachedPlot = plot;
      this.#attachedRevision = revision;
    } catch (error) {
      if (sessionEpoch === this.#sessionEpoch) {
        this.#hasPlot = false;
        this.#attachedPlot = null;
        this.#attachedRevision = null;
      }
      throw error;
    }
  }

  resize(width?: number, height?: number, scaleFactor?: number): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.resize(width, height, scaleFactor);
      return;
    }

    const nextMetrics =
      typeof width === "number" && typeof height === "number" && typeof scaleFactor === "number"
        ? { width, height, scaleFactor }
        : getCanvasMetrics(this.#canvas);

    this.#post("resize", nextMetrics);
  }

  setTime(timeSeconds: number): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.setTime(timeSeconds);
      return;
    }

    if (!this.#worker) {
      return;
    }

    this.#post("setTime", { timeSeconds });
  }

  setBackendPreference(backendPreference: BackendPreference): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.setBackendPreference(backendPreference);
      return;
    }

    this.#post("setBackendPreference", {
      backendPreference: normalizeBackendPreference(backendPreference),
    });
  }

  render(): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.render();
      return;
    }

    if (!this.#canDispatchPlotCommand()) {
      return;
    }

    this.#post("render");
  }

  resetView(): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.resetView();
      return;
    }

    if (!this.#canDispatchPlotCommand()) {
      return;
    }

    this.#post("resetView");
  }

  pointerDown(x: number, y: number, button: number): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.pointerDown(x, y, button);
      return;
    }

    if (!this.#canDispatchPlotCommand()) {
      return;
    }

    this.#post("pointerDown", { x, y, button });
  }

  pointerMove(x: number, y: number): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.pointerMove(x, y);
      return;
    }

    if (!this.#canDispatchPlotCommand()) {
      return;
    }

    this.#post("pointerMove", { x, y });
  }

  pointerUp(x: number, y: number, button: number): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.pointerUp(x, y, button);
      return;
    }

    if (!this.#canDispatchPlotCommand()) {
      return;
    }

    this.#post("pointerUp", { x, y, button });
  }

  pointerLeave(): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.pointerLeave();
      return;
    }

    if (!this.#canDispatchPlotCommand()) {
      return;
    }

    this.#post("pointerLeave");
  }

  wheel(deltaY: number, x: number, y: number): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.wheel(deltaY, x, y);
      return;
    }

    if (!this.#canDispatchPlotCommand()) {
      return;
    }

    this.#post("wheel", { deltaY, x, y });
  }

  /**
   * The series data point under a canvas pixel, or `null`. Asynchronous
   * because a worker session answers from the worker; the main-thread
   * fallback resolves immediately.
   */
  async hitAt(x: number, y: number): Promise<SeriesHit | null> {
    if (this.#fallbackSession) {
      return this.#fallbackSession.hitAt(x, y);
    }

    if (!this.#canDispatchPlotCommand()) {
      return null;
    }

    const payload = await this.#request("hitAt", { x, y });
    return payload as SeriesHit | null;
  }

  /** The index of the series behind a legend entry at a canvas pixel, or `null`. */
  async legendEntryAt(x: number, y: number): Promise<number | null> {
    if (this.#fallbackSession) {
      return this.#fallbackSession.legendEntryAt(x, y);
    }

    if (!this.#canDispatchPlotCommand()) {
      return null;
    }

    const payload = await this.#request("legendEntryAt", { x, y });
    return payload as number | null;
  }

  async seriesCount(): Promise<number> {
    if (this.#fallbackSession) {
      return this.#fallbackSession.seriesCount();
    }

    if (!this.#canDispatchPlotCommand()) {
      return 0;
    }

    const payload = await this.#request("seriesCount");
    return payload as number;
  }

  async seriesLabel(seriesIndex: number): Promise<string | null> {
    if (this.#fallbackSession) {
      return this.#fallbackSession.seriesLabel(seriesIndex);
    }

    if (!this.#canDispatchPlotCommand()) {
      return null;
    }

    const payload = await this.#request("seriesLabel", { seriesIndex });
    return payload as string | null;
  }

  async seriesVisible(seriesIndex: number): Promise<boolean> {
    if (this.#fallbackSession) {
      return this.#fallbackSession.seriesVisible(seriesIndex);
    }

    if (!this.#canDispatchPlotCommand()) {
      return true;
    }

    const payload = await this.#request("seriesVisible", { seriesIndex });
    return payload as boolean;
  }

  /**
   * Show or hide a series and re-render; the legend entry stays, dimmed.
   * A grouped series toggles with its group. Resolves `false` when the
   * index is out of range or no plot is attached.
   */
  async setSeriesVisible(seriesIndex: number, visible: boolean): Promise<boolean> {
    if (this.#fallbackSession) {
      return this.#fallbackSession.setSeriesVisible(seriesIndex, visible);
    }

    if (!this.#canDispatchPlotCommand()) {
      return false;
    }

    const payload = await this.#request("setSeriesVisible", { seriesIndex, visible });
    return payload as boolean;
  }

  async exportPng(): Promise<Uint8Array> {
    if (this.#fallbackSession) {
      return this.#fallbackSession.exportPng();
    }

    if (!this.#canDispatchPlotCommand()) {
      throw new Error("no plot is attached to this worker session");
    }

    const payload = await this.#request("exportPng");
    return payload as Uint8Array;
  }

  async exportSvg(): Promise<string> {
    if (this.#fallbackSession) {
      return this.#fallbackSession.exportSvg();
    }

    if (!this.#canDispatchPlotCommand()) {
      throw new Error("no plot is attached to this worker session");
    }

    const payload = await this.#request("exportSvg");
    return payload as string;
  }

  async ready(): Promise<void> {
    return this.#readyPromise;
  }

  destroy(): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.destroy();
      this.#hasPlot = false;
      this.#attachedPlot = null;
      this.#attachedRevision = null;
      return;
    }

    this.#sessionEpoch += 1;
    this.#hasPlot = false;
    this.#attachedPlot = null;
    this.#attachedRevision = null;
    this.#post("destroy");
  }

  dispose(): void {
    for (const dispose of this.#cleanup.splice(0)) {
      dispose();
    }

    if (this.#fallbackSession) {
      this.#fallbackSession.dispose();
      this.#fallbackSession = null;
      this.#hasPlot = false;
      this.#attachedPlot = null;
      this.#attachedRevision = null;
      return;
    }

    this.#sessionEpoch += 1;
    this.#hasPlot = false;
    this.#attachedPlot = null;
    this.#attachedRevision = null;
    this.#worker?.terminate();
    this.#worker = null;
    this.#failAll(new Error("worker session was disposed"));
  }

  _pushCleanup(dispose: () => void): void {
    this.#cleanup.push(dispose);
  }

  #post(type: string, payload?: unknown): void {
    this.#worker?.postMessage({ type, payload } satisfies WorkerEnvelope);
  }

  #request(type: string, payload?: unknown): Promise<unknown> {
    const worker = this.#worker;
    if (!worker) {
      return Promise.reject(new Error("worker session is not active"));
    }

    const requestId = this.#nextRequestId++;

    return new Promise((resolve, reject) => {
      this.#pendingRequests.set(requestId, { resolve, reject });
      worker.postMessage({ type, payload, requestId } satisfies WorkerEnvelope);
    });
  }

  #handleMessage(event: MessageEvent<WorkerEnvelope>): void {
    const { payload, requestId, type } = event.data;

    if (type === "ready") {
      this.#resolveReady?.();
      this.#resolveReady = null;
      this.#rejectReady = null;
      return;
    }

    if (type === "error") {
      const error = new Error(String(payload ?? "worker session failed"));
      if (typeof requestId === "number") {
        const pending = this.#pendingRequests.get(requestId);
        if (pending) {
          this.#pendingRequests.delete(requestId);
          pending.reject(error);
          return;
        }
      }
      this.#failAll(error);
      return;
    }

    if (typeof requestId !== "number") {
      return;
    }

    const pending = this.#pendingRequests.get(requestId);
    if (!pending) {
      return;
    }

    this.#pendingRequests.delete(requestId);
    pending.resolve(payload);
  }

  #failAll(reason: unknown): void {
    this.#hasPlot = false;
    this.#attachedPlot = null;
    this.#attachedRevision = null;
    this.#rejectReady?.(reason);
    this.#resolveReady = null;
    this.#rejectReady = null;

    for (const [requestId, pending] of this.#pendingRequests.entries()) {
      this.#pendingRequests.delete(requestId);
      pending.reject(reason);
    }
  }
}

/** Create a new fluent plot builder. */
export function createPlot(): PlotBuilder {
  return new PlotBuilder();
}

/** Rehydrate a plot builder from a serialized snapshot. */
export function createPlotFromSnapshot(snapshot: PlotSnapshot): PlotBuilder {
  return PlotBuilder.fromSnapshot(snapshot);
}

/** Create a mutable observable numeric series. */
export function createObservable(values: NumericArray): ObservableSeries {
  return new ObservableSeries(values);
}

/** Create a time-varying sine signal. */
export function createSineSignal(options: SineSignalOptions): SineSignal {
  return new SineSignal(options);
}

/** Register custom font bytes for browser-hosted text rendering. */
export async function registerFont(bytes: Uint8Array | ArrayLike<number>): Promise<void> {
  const module = await ensureRawModule();
  module.register_font_bytes_js(Uint8Array.from(bytes));
}

/** Inspect browser runtime capabilities used by the interactive renderer. */
export async function getRuntimeCapabilities(): Promise<RuntimeCapabilities> {
  const module = await ensureRawModule();
  const capabilities = module.web_runtime_capabilities();

  return {
    offscreenCanvasSupported: readBooleanProperty(
      capabilities,
      "offscreen_canvas_supported",
      "offscreenCanvasSupported",
    ),
    workerSupported: readBooleanProperty(capabilities, "worker_supported", "workerSupported"),
    webgpuSupported: readBooleanProperty(capabilities, "webgpu_supported", "webgpuSupported"),
    touchInputSupported: readBooleanProperty(
      capabilities,
      "touch_input_supported",
      "touchInputSupported",
    ),
    defaultBrowserFontRegistered: readBooleanProperty(
      capabilities,
      "default_browser_font_registered",
      "defaultBrowserFontRegistered",
    ),
    gpuCanvasFastPathAvailable: readBooleanProperty(
      capabilities,
      "gpu_canvas_fast_path_available",
      "gpuCanvasFastPathAvailable",
    ),
  };
}

/**
 * Create an interactive main-thread canvas session.
 *
 * Use this when you want explicit control over session setup rather than
 * calling `plot.mount(canvas)`.
 */
export async function createCanvasSession(
  canvas: HTMLCanvasElement,
  options?: CanvasSessionOptions,
): Promise<CanvasSession> {
  const normalized = normalizeCanvasSessionOptions(options);
  const module = await ensureRawModule();
  const rawSession = new module.WebCanvasSession(canvas);
  const session = new CanvasSession(module, rawSession, canvas);

  session.setBackendPreference(normalized.backendPreference);
  if (typeof normalized.initialTime === "number") {
    session.setTime(normalized.initialTime);
  }

  session.resize();

  if (normalized.autoResize) {
    session._pushCleanup(
      installCanvasResize(canvas, (width, height, scaleFactor) => {
        session.resize(width, height, scaleFactor);
      }),
    );
  }

  if (normalized.bindInput) {
    session._pushCleanup(
      attachCanvasEvents(canvas, {
        pointerDown: (x, y, button) => session.pointerDown(x, y, button),
        pointerMove: (x, y) => session.pointerMove(x, y),
        pointerUp: (x, y, button) => session.pointerUp(x, y, button),
        pointerLeave: () => session.pointerLeave(),
        wheel: (deltaY, x, y) => session.wheel(deltaY, x, y),
      }),
    );
  }

  return session;
}

/**
 * Create a worker-backed canvas session with optional main-thread fallback.
 *
 * This is the preferred path for heavier interactive views when the browser
 * supports `Worker` plus `OffscreenCanvas`.
 */
export async function createWorkerSession(
  canvas: HTMLCanvasElement,
  options?: WorkerSessionOptions,
): Promise<WorkerSession> {
  const normalized = normalizeWorkerSessionOptions(options);

  if (!workerCanvasSupported(canvas)) {
    if (!normalized.fallbackToMainThread) {
      throw new Error("worker canvas support is unavailable in this browser");
    }

    const fallbackSession = await createCanvasSession(canvas, normalized);
    const session = new WorkerSession(canvas, "main-thread", fallbackSession);
    await session.ready();
    return session;
  }

  const session = new WorkerSession(canvas, "worker");
  const worker = new Worker(new URL("./session-worker.js", import.meta.url), {
    type: "module",
  });
  session.attachWorker(worker);

  const offscreen = (
    canvas as HTMLCanvasElement & {
      transferControlToOffscreen(): OffscreenCanvas;
    }
  ).transferControlToOffscreen();
  const initialMetrics = getCanvasMetrics(canvas);

  worker.postMessage(
    {
      type: "init",
      canvas: offscreen,
      width: initialMetrics.width,
      height: initialMetrics.height,
      scaleFactor: initialMetrics.scaleFactor,
      backendPreference: normalized.backendPreference,
      initialTime: normalized.initialTime,
    } satisfies WorkerEnvelope,
    [offscreen],
  );

  if (normalized.autoResize) {
    session._pushCleanup(
      installCanvasResize(canvas, (width, height, scaleFactor) => {
        session.resize(width, height, scaleFactor);
      }),
    );
  }

  if (normalized.bindInput) {
    session._pushCleanup(
      attachCanvasEvents(canvas, {
        pointerDown: (x, y, button) => session.pointerDown(x, y, button),
        pointerMove: (x, y) => session.pointerMove(x, y),
        pointerUp: (x, y, button) => session.pointerUp(x, y, button),
        pointerLeave: () => session.pointerLeave(),
        wheel: (deltaY, x, y) => session.wheel(deltaY, x, y),
      }),
    );
  }

  await session.ready();
  return session;
}
