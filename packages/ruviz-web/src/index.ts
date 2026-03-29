import initRaw, * as raw from "../generated/raw/ruviz_web_raw.js";
import type {
  JsPlot as RawJsPlot,
  ObservableVecF64 as RawObservableVecF64,
  SignalVecF64 as RawSignalVecF64,
  WebCanvasSession as RawWebCanvasSession,
} from "../generated/raw/ruviz_web_raw.js";
import {
  clonePlotSnapshot,
  normalizeSineSignalOptions,
  toNumberArray,
  type BackendPreference,
  type CanvasSessionOptions,
  type NormalizedSineSignalOptions,
  type NumericArray,
  type PlotSnapshot,
  type PlotTheme,
  type RuntimeCapabilities,
  type SessionMode,
  type SineSignalOptions,
  type WorkerSessionOptions,
  type XSourceSnapshot,
  type YSourceSnapshot,
} from "./shared.js";

export type {
  BackendPreference,
  CanvasSessionOptions,
  NumericArray,
  PlotTheme,
  RuntimeCapabilities,
  SessionMode,
  SineSignalOptions,
  WorkerSessionOptions,
} from "./shared.js";

type RawModule = typeof import("../generated/raw/ruviz_web_raw.js");

type XSourceDefinition =
  | { kind: "static"; values: number[] }
  | { kind: "observable"; source: ObservableSeries };

type YSourceDefinition =
  | { kind: "static"; values: number[] }
  | { kind: "observable"; source: ObservableSeries }
  | { kind: "sine-signal"; source: SineSignal };

interface PlotSeriesDefinition {
  kind: "line" | "scatter";
  x: XSourceDefinition;
  y: YSourceDefinition;
}

interface PlotSeriesInput {
  x: NumericArray | ObservableSeries;
  y: NumericArray | ObservableSeries | SineSignal;
}

interface PlotState {
  sizePx?: [number, number];
  theme?: PlotTheme;
  ticks?: boolean;
  title?: string;
  xLabel?: string;
  yLabel?: string;
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
      raw.register_default_browser_fonts_js();
      return raw;
    });
  }

  return rawModulePromise;
}

function normalizeBackendPreference(
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

function toRawBackendPreference(
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

function staticValuesFromXSource(source: XSourceDefinition): number[] {
  return source.kind === "observable" ? source.source.snapshotValues() : [...source.values];
}

function staticValuesFromYSource(source: Exclude<YSourceDefinition, { kind: "sine-signal" }>): number[] {
  return source.kind === "observable" ? source.source.snapshotValues() : [...source.values];
}

function xSourceLength(source: XSourceDefinition): number {
  return source.kind === "observable" ? source.source.length : source.values.length;
}

function ySourceLength(source: YSourceDefinition): number {
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

function normalizeYSource(
  source: NumericArray | ObservableSeries | SineSignal,
): YSourceDefinition {
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
  source: XSourceDefinition | Exclude<YSourceDefinition, { kind: "sine-signal" }>,
): Promise<RawObservableVecF64> {
  if (source.kind === "observable") {
    return source.source._toRawObservable(module);
  }

  return new module.ObservableVecF64(Float64Array.from(source.values));
}

function applyPlotMetadata(rawPlot: RawJsPlot, state: PlotState): void {
  if (state.sizePx) {
    rawPlot.size_px(state.sizePx[0], state.sizePx[1]);
  }

  if (state.theme === "dark") {
    rawPlot.theme_dark();
  } else if (state.theme === "light") {
    rawPlot.theme_light();
  }

  if (typeof state.ticks === "boolean") {
    rawPlot.ticks(state.ticks);
  }

  if (state.title) {
    rawPlot.title(state.title);
  }

  if (state.xLabel) {
    rawPlot.xlabel(state.xLabel);
  }

  if (state.yLabel) {
    rawPlot.ylabel(state.yLabel);
  }
}

async function buildRawPlotFromState(state: PlotState, module: RawModule): Promise<RawJsPlot> {
  const rawPlot = new module.JsPlot();
  applyPlotMetadata(rawPlot, state);

  for (const series of state.series) {
    if (series.y.kind === "sine-signal") {
      const signal = await series.y.source._toRawSignal(module);
      const xValues = Float64Array.from(staticValuesFromXSource(series.x));

      if (series.kind === "line") {
        rawPlot.line_signal(xValues, signal);
      } else {
        rawPlot.scatter_signal(xValues, signal);
      }
      continue;
    }

    if (series.x.kind === "observable" || series.y.kind === "observable") {
      const xObservable = await ephemeralObservable(module, series.x);
      const yObservable = await ephemeralObservable(module, series.y);

      if (series.kind === "line") {
        rawPlot.line_observable(xObservable, yObservable);
      } else {
        rawPlot.scatter_observable(xObservable, yObservable);
      }
      continue;
    }

    const xValues = Float64Array.from(series.x.values);
    const yValues = Float64Array.from(series.y.values);

    if (series.kind === "line") {
      rawPlot.line(xValues, yValues);
    } else {
      rawPlot.scatter(xValues, yValues);
    }
  }

  return rawPlot;
}

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

export class PlotBuilder {
  #state: PlotState;

  constructor(state?: PlotState) {
    this.#state = state
      ? {
          ...state,
          sizePx: state.sizePx ? [...state.sizePx] as [number, number] : undefined,
          series: state.series.map((series) => ({
            kind: series.kind,
            x: series.x.kind === "static" ? { kind: "static", values: [...series.x.values] } : series.x,
            y:
              series.y.kind === "static"
                ? { kind: "static", values: [...series.y.values] }
                : series.y,
          })),
        }
      : { series: [] };
  }

  setSizePx(width: number, height: number): this {
    this.#state.sizePx = [Math.max(1, Math.round(width)), Math.max(1, Math.round(height))];
    return this;
  }

  setTheme(theme: PlotTheme): this {
    this.#state.theme = theme;
    return this;
  }

  setTicks(enabled: boolean): this {
    this.#state.ticks = enabled;
    return this;
  }

  setTitle(title: string): this {
    this.#state.title = title;
    return this;
  }

  setXLabel(label: string): this {
    this.#state.xLabel = label;
    return this;
  }

  setYLabel(label: string): this {
    this.#state.yLabel = label;
    return this;
  }

  addLine(input: PlotSeriesInput): this {
    return this.#addSeries("line", input);
  }

  addScatter(input: PlotSeriesInput): this {
    return this.#addSeries("scatter", input);
  }

  clone(): PlotBuilder {
    return new PlotBuilder(this.#state);
  }

  toSnapshot(): PlotSnapshot {
    const snapshot: PlotSnapshot = {
      sizePx: this.#state.sizePx ? [...this.#state.sizePx] as [number, number] : undefined,
      theme: this.#state.theme,
      ticks: this.#state.ticks,
      title: this.#state.title,
      xLabel: this.#state.xLabel,
      yLabel: this.#state.yLabel,
      series: this.#state.series.map((series) => ({
        kind: series.kind,
        x: this.#serializeXSource(series.x),
        y: this.#serializeYSource(series.y),
      })),
    };

    return clonePlotSnapshot(snapshot);
  }

  async renderPng(): Promise<Uint8Array> {
    const module = await ensureRawModule();
    const rawPlot = await this._toRawPlot(module);
    return rawPlot.render_png_bytes();
  }

  async renderSvg(): Promise<string> {
    const module = await ensureRawModule();
    const rawPlot = await this._toRawPlot(module);
    return rawPlot.render_svg();
  }

  async _toRawPlot(module?: RawModule): Promise<RawJsPlot> {
    const currentModule = module ?? (await ensureRawModule());
    return buildRawPlotFromState(this.#state, currentModule);
  }

  #addSeries(kind: PlotSeriesDefinition["kind"], input: PlotSeriesInput): this {
    const x = normalizeXSource(input.x);
    const y = normalizeYSource(input.y);

    if (xSourceLength(x) !== ySourceLength(y)) {
      throw new Error("x and y must have the same length");
    }

    this.#state.series.push({ kind, x, y });
    return this;
  }

  #serializeXSource(source: XSourceDefinition): XSourceSnapshot {
    if (source.kind === "observable") {
      return {
        kind: "observable",
        values: source.source.snapshotValues(),
      };
    }

    return {
      kind: "static",
      values: [...source.values],
    };
  }

  #serializeYSource(source: YSourceDefinition): YSourceSnapshot {
    switch (source.kind) {
      case "observable":
        return {
          kind: "observable",
          values: source.source.snapshotValues(),
        };
      case "sine-signal":
        return {
          kind: "sine-signal",
          options: { ...source.source.options },
        };
      case "static":
        return {
          kind: "static",
          values: [...source.values],
        };
    }
  }
}

export class CanvasSession {
  readonly mode: SessionMode = "main-thread";

  #module: RawModule;
  #rawSession: RawWebCanvasSession;
  #canvas: HTMLCanvasElement;
  #cleanup: Array<() => void>;

  constructor(module: RawModule, rawSession: RawWebCanvasSession, canvas: HTMLCanvasElement) {
    this.#module = module;
    this.#rawSession = rawSession;
    this.#canvas = canvas;
    this.#cleanup = [];
  }

  hasPlot(): boolean {
    return this.#rawSession.has_plot();
  }

  async setPlot(plot: PlotBuilder): Promise<void> {
    const rawPlot = await plot._toRawPlot(this.#module);
    this.#rawSession.set_plot(rawPlot);
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
    this.#rawSession.render();
  }

  resetView(): void {
    this.#rawSession.reset_view();
  }

  pointerDown(x: number, y: number, button: number): void {
    this.#rawSession.pointer_down(x, y, button);
  }

  pointerMove(x: number, y: number): void {
    this.#rawSession.pointer_move(x, y);
  }

  pointerUp(x: number, y: number, button: number): void {
    this.#rawSession.pointer_up(x, y, button);
  }

  pointerLeave(): void {
    this.#rawSession.pointer_leave();
  }

  wheel(deltaY: number, x: number, y: number): void {
    this.#rawSession.wheel(deltaY, x, y);
  }

  async exportPng(): Promise<Uint8Array> {
    return this.#rawSession.export_png();
  }

  async exportSvg(): Promise<string> {
    return this.#rawSession.export_svg();
  }

  destroy(): void {
    this.#rawSession.destroy();
  }

  dispose(): void {
    for (const dispose of this.#cleanup.splice(0)) {
      dispose();
    }
    this.#rawSession.destroy();
  }

  _pushCleanup(dispose: () => void): void {
    this.#cleanup.push(dispose);
  }
}

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

  constructor(canvas: HTMLCanvasElement, mode: SessionMode, fallbackSession?: CanvasSession) {
    this.mode = mode;
    this.#canvas = canvas;
    this.#worker = null;
    this.#fallbackSession = fallbackSession ?? null;
    this.#cleanup = [];
    this.#pendingRequests = new Map();
    this.#nextRequestId = 1;
    this.#hasPlot = false;
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

  async setPlot(plot: PlotBuilder): Promise<void> {
    if (this.#fallbackSession) {
      await this.#fallbackSession.setPlot(plot);
      return;
    }

    this.#hasPlot = true;
    await this.#request("setPlot", { snapshot: plot.toSnapshot() });
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

    this.#post("render");
  }

  resetView(): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.resetView();
      return;
    }

    this.#post("resetView");
  }

  pointerDown(x: number, y: number, button: number): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.pointerDown(x, y, button);
      return;
    }

    this.#post("pointerDown", { x, y, button });
  }

  pointerMove(x: number, y: number): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.pointerMove(x, y);
      return;
    }

    this.#post("pointerMove", { x, y });
  }

  pointerUp(x: number, y: number, button: number): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.pointerUp(x, y, button);
      return;
    }

    this.#post("pointerUp", { x, y, button });
  }

  pointerLeave(): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.pointerLeave();
      return;
    }

    this.#post("pointerLeave");
  }

  wheel(deltaY: number, x: number, y: number): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.wheel(deltaY, x, y);
      return;
    }

    this.#post("wheel", { deltaY, x, y });
  }

  async exportPng(): Promise<Uint8Array> {
    if (this.#fallbackSession) {
      return this.#fallbackSession.exportPng();
    }

    const payload = await this.#request("exportPng");
    return payload as Uint8Array;
  }

  async exportSvg(): Promise<string> {
    if (this.#fallbackSession) {
      return this.#fallbackSession.exportSvg();
    }

    const payload = await this.#request("exportSvg");
    return payload as string;
  }

  async ready(): Promise<void> {
    await this.#readyPromise;
  }

  destroy(): void {
    if (this.#fallbackSession) {
      this.#fallbackSession.destroy();
      this.#hasPlot = false;
      return;
    }

    this.#hasPlot = false;
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
      return;
    }

    this.#hasPlot = false;
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
    this.#rejectReady?.(reason);
    this.#resolveReady = null;
    this.#rejectReady = null;

    for (const [requestId, pending] of this.#pendingRequests.entries()) {
      this.#pendingRequests.delete(requestId);
      pending.reject(reason);
    }
  }
}

export function createPlot(): PlotBuilder {
  return new PlotBuilder();
}

export function createObservable(values: NumericArray): ObservableSeries {
  return new ObservableSeries(values);
}

export function createSineSignal(options: SineSignalOptions): SineSignal {
  return new SineSignal(options);
}

export async function registerFont(bytes: Uint8Array | ArrayLike<number>): Promise<void> {
  const module = await ensureRawModule();
  module.register_font_bytes_js(Uint8Array.from(bytes));
}

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

  const offscreen = (canvas as HTMLCanvasElement & {
    transferControlToOffscreen(): OffscreenCanvas;
  }).transferControlToOffscreen();
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
