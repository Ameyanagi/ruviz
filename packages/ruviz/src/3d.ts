import initRaw, * as raw from "../generated/raw/ruviz_web_raw.js";
import { toNumberArray, type NumericArray } from "./shared.js";

type RawModule = typeof import("../generated/raw/ruviz_web_raw.js");
type RawPlot3d = raw.JsPlot3D;
type RawSession3d = raw.WebGPU3DCanvasSession | raw.OffscreenWebGPU3DCanvasSession;

type PlotKind3d = "scatter3d" | "line3d" | "surface" | "wireframe";

interface PointSeries3d {
  kind: "scatter3d" | "line3d";
  x: number[];
  y: number[];
  z: number[];
}

interface GridSeries3d {
  kind: "surface" | "wireframe";
  x: number[];
  y: number[];
  z: number[];
}

type Series3d = PointSeries3d | GridSeries3d;

/** A row-major grid, either flat or grouped into one row per y value. */
export type GridValues3d = NumericArray | readonly NumericArray[];

export interface Plot3dMountOptions {
  /**
   * Keep an HTML canvas backing surface synchronized with its CSS size.
   *
   * Defaults to `true` for `HTMLCanvasElement` and is unavailable for an
   * `OffscreenCanvas`.
   */
  autoResize?: boolean;

  /**
   * Bind orbit, pan, zoom, reset, and picking input to an HTML canvas.
   *
   * Defaults to `true` for `HTMLCanvasElement` and is unavailable for an
   * `OffscreenCanvas`.
   */
  bindInput?: boolean;

  /**
   * Device scale used for text and point sizing.
   *
   * Defaults to `window.devicePixelRatio` on the main thread and `1` for an
   * `OffscreenCanvas`. Pass the main thread's device pixel ratio when mounting
   * a transferred canvas in a worker.
   */
  scaleFactor?: number;

  /** Scheduled render/input errors. Initial mounting errors reject mount(). */
  onError?: (error: Error) => void;
}

export type Plot3dSessionMode = "main-thread" | "offscreen";

/**
 * A mounted retained WebGPU 3D plot.
 *
 * Input, resize, and `render()` calls are coalesced into at most one WebGPU
 * submission per animation frame.
 */
export interface Plot3dSession {
  readonly mode: Plot3dSessionMode;
  readonly canvas: HTMLCanvasElement | OffscreenCanvas;

  /** Request a frame. Multiple requests in one animation frame are coalesced. */
  render(): void;

  /** Last scheduled failure, cleared after a successful render. */
  readonly error: Error | null;

  resize(width?: number, height?: number, scaleFactor?: number): void;
  resetView(): void;
  pointerDown(x: number, y: number, button: number): void;
  pointerMove(x: number, y: number): void;
  pointerUp(x: number, y: number, button: number): void;
  doubleClick(x: number, y: number): void;
  wheel(deltaY: number): void;

  selectedSeries(): number | null;
  selectedSource(): number | null;
  backend(): string;
  needsRecreate(): boolean;
  exportPng(): Promise<Uint8Array>;

  destroy(): void;
  dispose(): void;
}

let rawModulePromise: Promise<RawModule> | null = null;

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

function finiteScaleFactor(value: number | undefined, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : fallback;
}

function isHtmlCanvas(canvas: HTMLCanvasElement | OffscreenCanvas): canvas is HTMLCanvasElement {
  return typeof HTMLCanvasElement !== "undefined" && canvas instanceof HTMLCanvasElement;
}

function assertWebGpuAvailable(): void {
  if (typeof navigator === "undefined" || !("gpu" in navigator)) {
    throw new Error("WebGPU is not available in this browser");
  }
}

function numericValues(values: NumericArray, label: string, minimumLength: number): number[] {
  const normalized = toNumberArray(values);
  if (normalized.length < minimumLength) {
    throw new Error(`${label} must contain at least ${minimumLength} value(s)`);
  }

  const invalidIndex = normalized.findIndex((value) => !Number.isFinite(value));
  if (invalidIndex >= 0) {
    throw new Error(`${label}[${invalidIndex}] must be finite`);
  }

  return normalized;
}

function equalPointLengths(kind: PlotKind3d, x: number[], y: number[], z: number[]): void {
  if (x.length !== y.length || y.length !== z.length) {
    throw new Error(
      `${kind} x, y, and z must have the same length ` +
        `(x=${x.length}, y=${y.length}, z=${z.length})`,
    );
  }
}

function isNestedGrid(values: GridValues3d): values is readonly NumericArray[] {
  if (ArrayBuffer.isView(values)) {
    return false;
  }

  const first = (values as ArrayLike<unknown>)[0];
  return first !== undefined && typeof first !== "number";
}

function gridValues(
  kind: "surface" | "wireframe",
  x: number[],
  y: number[],
  values: GridValues3d,
): number[] {
  if (!isNestedGrid(values)) {
    const flat = numericValues(values as NumericArray, `${kind} z`, x.length * y.length);
    if (flat.length !== x.length * y.length) {
      throw new Error(
        `${kind} z must contain x.length * y.length values ` +
          `(expected ${x.length * y.length}, got ${flat.length})`,
      );
    }
    return flat;
  }

  if (values.length !== y.length) {
    throw new Error(
      `${kind} z must have one row per y value ` + `(expected ${y.length}, got ${values.length})`,
    );
  }

  const flat: number[] = [];
  values.forEach((row, rowIndex) => {
    const normalized = numericValues(row, `${kind} z[${rowIndex}]`, x.length);
    if (normalized.length !== x.length) {
      throw new Error(
        `${kind} z[${rowIndex}] must have one value per x value ` +
          `(expected ${x.length}, got ${normalized.length})`,
      );
    }
    flat.push(...normalized);
  });
  return flat;
}

function getCanvasMetrics(
  canvas: HTMLCanvasElement,
  scaleFactorOverride?: number,
): { width: number; height: number; scaleFactor: number } {
  const rect = canvas.getBoundingClientRect();
  const defaultScale = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
  const scaleFactor = finiteScaleFactor(scaleFactorOverride, defaultScale);
  return {
    width: Math.max(1, Math.round(rect.width * scaleFactor)),
    height: Math.max(1, Math.round(rect.height * scaleFactor)),
    scaleFactor,
  };
}

function pointerPosition(
  canvas: HTMLCanvasElement,
  event: MouseEvent | PointerEvent,
): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  const scaleX = rect.width === 0 ? 1 : canvas.width / rect.width;
  const scaleY = rect.height === 0 ? 1 : canvas.height / rect.height;
  return {
    x: (event.clientX - rect.left) * scaleX,
    y: (event.clientY - rect.top) * scaleY,
  };
}

class FrameScheduler3d {
  #callback: () => void;
  #frame: number | null = null;
  #timeout = false;
  #disposed = false;

  constructor(callback: () => void) {
    this.#callback = callback;
  }

  request(): void {
    if (this.#disposed || this.#frame !== null) {
      return;
    }

    if (typeof globalThis.requestAnimationFrame === "function") {
      this.#timeout = false;
      this.#frame = globalThis.requestAnimationFrame(() => this.#run());
      return;
    }

    this.#timeout = true;
    this.#frame = globalThis.setTimeout(() => this.#run(), 16);
  }

  dispose(): void {
    this.#disposed = true;
    if (this.#frame === null) {
      return;
    }

    if (this.#timeout) {
      globalThis.clearTimeout(this.#frame);
    } else {
      globalThis.cancelAnimationFrame(this.#frame);
    }
    this.#frame = null;
  }

  #run(): void {
    this.#frame = null;
    if (!this.#disposed) {
      this.#callback();
    }
  }
}

class MountedPlot3dSession implements Plot3dSession {
  readonly mode: Plot3dSessionMode;
  readonly canvas: HTMLCanvasElement | OffscreenCanvas;

  #raw: RawSession3d;
  #scheduler: FrameScheduler3d;
  #cleanup: Array<() => void> = [];
  #destroyed = false;
  #scaleFactor: number;
  #onError?: (error: Error) => void;
  #error: Error | null = null;
  #pendingMove: { x: number; y: number } | null = null;
  #pendingWheel = 0;

  constructor(
    rawSession: RawSession3d,
    canvas: HTMLCanvasElement | OffscreenCanvas,
    scaleFactor: number,
    onError?: (error: Error) => void,
  ) {
    this.#raw = rawSession;
    this.canvas = canvas;
    this.mode = isHtmlCanvas(canvas) ? "main-thread" : "offscreen";
    this.#scaleFactor = scaleFactor;
    this.#onError = onError;
    this.#scheduler = new FrameScheduler3d(() => {
      this.handleEvent(() => {
        this.#flushInput();
        this.#raw.render();
        this.#error = null;
      });
    });
  }

  get error(): Error | null {
    return this.#error;
  }

  render(): void {
    this.#assertActive();
    this.#scheduler.request();
  }

  // Browser event callbacks cannot reject a caller's promise. Route them to
  // the same explicit error channel as scheduled rendering.
  handleEvent(callback: () => void): void {
    if (this.#destroyed) return;
    try {
      callback();
    } catch (error) {
      this.#pendingMove = null;
      this.#pendingWheel = 0;
      this.#error = error instanceof Error ? error : new Error(String(error));
      if (this.#onError) this.#onError(this.#error);
      else console.error(this.#error);
    }
  }

  #flushInput(): void {
    const move = this.#pendingMove;
    const wheel = this.#pendingWheel;
    this.#pendingMove = null;
    this.#pendingWheel = 0;
    if (move) this.#raw.pointer_move(move.x, move.y);
    if (wheel !== 0) this.#raw.wheel(wheel);
  }

  resize(width?: number, height?: number, scaleFactor?: number): void {
    this.#assertActive();
    this.#flushInput();
    if (isHtmlCanvas(this.canvas) && (width === undefined || height === undefined)) {
      const metrics = getCanvasMetrics(this.canvas, scaleFactor ?? this.#scaleFactor);
      this.#scaleFactor = metrics.scaleFactor;
      this.#raw.resize(metrics.width, metrics.height, metrics.scaleFactor);
    } else {
      const nextWidth = Math.max(1, Math.round(width ?? this.canvas.width));
      const nextHeight = Math.max(1, Math.round(height ?? this.canvas.height));
      const nextScale = finiteScaleFactor(scaleFactor, this.#scaleFactor);
      this.#scaleFactor = nextScale;
      this.#raw.resize(nextWidth, nextHeight, nextScale);
    }
    this.render();
  }

  resetView(): void {
    this.#assertActive();
    this.#pendingMove = null;
    this.#pendingWheel = 0;
    this.#raw.reset_view();
    this.render();
  }

  pointerDown(x: number, y: number, button: number): void {
    this.#assertActive();
    this.#flushInput();
    this.#raw.pointer_down(x, y, button);
    this.render();
  }

  pointerMove(x: number, y: number): void {
    this.#assertActive();
    this.#pendingMove = { x, y };
    this.render();
  }

  pointerUp(x: number, y: number, button: number): void {
    this.#assertActive();
    this.#flushInput();
    this.#raw.pointer_up(x, y, button);
    this.render();
  }

  doubleClick(x: number, y: number): void {
    this.#assertActive();
    this.#pendingMove = null;
    this.#pendingWheel = 0;
    this.#raw.double_click(x, y);
    this.render();
  }

  wheel(deltaY: number): void {
    this.#assertActive();
    this.#pendingWheel += deltaY;
    this.render();
  }

  selectedSeries(): number | null {
    this.#assertActive();
    const selected = this.#raw.selected_series();
    return selected < 0 ? null : selected;
  }

  selectedSource(): number | null {
    this.#assertActive();
    const selected = this.#raw.selected_source();
    return selected < 0 ? null : selected;
  }

  backend(): string {
    this.#assertActive();
    return this.#raw.backend();
  }

  needsRecreate(): boolean {
    return this.#destroyed || this.#raw.needs_recreate();
  }

  async exportPng(): Promise<Uint8Array> {
    this.#assertActive();
    this.#flushInput();
    return new Uint8Array(this.#raw.export_png());
  }

  destroy(): void {
    this.dispose();
  }

  dispose(): void {
    if (this.#destroyed) {
      return;
    }

    this.#destroyed = true;
    this.#scheduler.dispose();
    for (const cleanup of this.#cleanup.splice(0)) {
      cleanup();
    }
    this.#raw.destroy();
    this.#raw.free();
  }

  pushCleanup(cleanup: () => void): void {
    this.#cleanup.push(cleanup);
  }

  #assertActive(): void {
    if (this.#destroyed) {
      throw new Error("the 3D plot session was destroyed");
    }
  }
}

function installCanvasResize(
  session: MountedPlot3dSession,
  canvas: HTMLCanvasElement,
  scaleFactor?: number,
): () => void {
  let lastWidth = 0;
  let lastHeight = 0;
  let lastScale = 0;

  const sync = () => {
    const metrics = getCanvasMetrics(canvas, scaleFactor);
    if (
      metrics.width === lastWidth &&
      metrics.height === lastHeight &&
      Math.abs(metrics.scaleFactor - lastScale) < 1e-6
    ) {
      return;
    }

    lastWidth = metrics.width;
    lastHeight = metrics.height;
    lastScale = metrics.scaleFactor;
    session.resize(metrics.width, metrics.height, metrics.scaleFactor);
  };

  sync();
  const onResize = () => session.handleEvent(sync);
  const observer = new ResizeObserver(onResize);
  observer.observe(canvas);
  window.addEventListener("resize", onResize);

  return () => {
    observer.disconnect();
    window.removeEventListener("resize", onResize);
  };
}

function bindCanvasInput(session: MountedPlot3dSession, canvas: HTMLCanvasElement): () => void {
  let activePointer: { id: number; button: number } | null = null;
  const previousTouchAction = canvas.style.touchAction;
  canvas.style.touchAction = "none";

  const onContextMenu = (event: MouseEvent) => {
    event.preventDefault();
  };

  const onPointerDown = (event: PointerEvent) =>
    session.handleEvent(() => {
      const point = pointerPosition(canvas, event);
      activePointer = { id: event.pointerId, button: event.button };
      canvas.setPointerCapture(event.pointerId);
      session.pointerDown(point.x, point.y, event.button);
    });

  const onPointerMove = (event: PointerEvent) =>
    session.handleEvent(() => {
      const point = pointerPosition(canvas, event);
      session.pointerMove(point.x, point.y);
    });

  const releasePointer = (event: PointerEvent) =>
    session.handleEvent(() => {
      const point = pointerPosition(canvas, event);
      const button = activePointer?.id === event.pointerId ? activePointer.button : event.button;
      if (canvas.hasPointerCapture(event.pointerId)) {
        canvas.releasePointerCapture(event.pointerId);
      }
      activePointer = null;
      session.pointerUp(point.x, point.y, button);
    });

  const onDoubleClick = (event: MouseEvent) =>
    session.handleEvent(() => {
      const point = pointerPosition(canvas, event);
      session.doubleClick(point.x, point.y);
    });

  const onWheel = (event: WheelEvent) =>
    session.handleEvent(() => {
      event.preventDefault();
      session.wheel(event.deltaY);
    });

  canvas.addEventListener("contextmenu", onContextMenu);
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", releasePointer);
  canvas.addEventListener("pointercancel", releasePointer);
  canvas.addEventListener("dblclick", onDoubleClick);
  canvas.addEventListener("wheel", onWheel, { passive: false });

  return () => {
    canvas.style.touchAction = previousTouchAction;
    canvas.removeEventListener("contextmenu", onContextMenu);
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", releasePointer);
    canvas.removeEventListener("pointercancel", releasePointer);
    canvas.removeEventListener("dblclick", onDoubleClick);
    canvas.removeEventListener("wheel", onWheel);
  };
}

/**
 * Fluent high-level WebGPU 3D plot builder.
 *
 * Series methods append in drawing order, matching the Rust builders.
 * Use clearSeries() before adding a replacement.
 */
export class Plot3dBuilder {
  #series: Series3d[] = [];
  #title: string | null = null;
  #axisAspect: [number, number, number] | "data" | null = null;
  #stableScale = false;

  scatter3d(x: NumericArray, y: NumericArray, z: NumericArray): this {
    const xValues = numericValues(x, "scatter3d x", 1);
    const yValues = numericValues(y, "scatter3d y", 1);
    const zValues = numericValues(z, "scatter3d z", 1);
    equalPointLengths("scatter3d", xValues, yValues, zValues);
    this.#series.push({ kind: "scatter3d", x: xValues, y: yValues, z: zValues });
    return this;
  }

  line3d(x: NumericArray, y: NumericArray, z: NumericArray): this {
    const xValues = numericValues(x, "line3d x", 2);
    const yValues = numericValues(y, "line3d y", 2);
    const zValues = numericValues(z, "line3d z", 2);
    equalPointLengths("line3d", xValues, yValues, zValues);
    this.#series.push({ kind: "line3d", x: xValues, y: yValues, z: zValues });
    return this;
  }

  surface(x: NumericArray, y: NumericArray, z: GridValues3d): this {
    const xValues = numericValues(x, "surface x", 2);
    const yValues = numericValues(y, "surface y", 2);
    this.#series.push({
      kind: "surface",
      x: xValues,
      y: yValues,
      z: gridValues("surface", xValues, yValues, z),
    });
    return this;
  }

  wireframe(x: NumericArray, y: NumericArray, z: GridValues3d): this {
    const xValues = numericValues(x, "wireframe x", 2);
    const yValues = numericValues(y, "wireframe y", 2);
    this.#series.push({
      kind: "wireframe",
      x: xValues,
      y: yValues,
      z: gridValues("wireframe", xValues, yValues, z),
    });
    return this;
  }

  /** Remove series while retaining plot options, for explicit replacement. */
  clearSeries(): this {
    this.#series = [];
    return this;
  }

  /** Fix the plotting box's X:Y:Z proportions; each component must be positive. */
  axisAspect(x: number, y: number, z: number): this {
    const values: [number, number, number] = [Math.fround(x), Math.fround(y), Math.fround(z)];
    const max = Math.max(...values);
    if (
      values.some(
        (value) =>
          !Number.isFinite(value) ||
          value <= 0 ||
          !Number.isFinite(Math.fround(1 / Math.fround(value / max))),
      )
    ) {
      throw new Error(
        "axisAspect(): ratios must be finite, positive, and representable in 3D coordinates",
      );
    }
    this.#axisAspect = values;
    return this;
  }

  /** Keep equal data units equally long on all axes, preserving physical shapes. */
  equalScale(): this {
    this.#axisAspect = "data";
    return this;
  }

  /** Keep framing and scale fixed while rotating; explicit zoom still works. */
  stableScale(enabled = true): this {
    this.#stableScale = enabled;
    return this;
  }

  title(title: string): this {
    this.#title = String(title);
    return this;
  }

  async mount(
    canvas: HTMLCanvasElement | OffscreenCanvas,
    options: Plot3dMountOptions = {},
  ): Promise<Plot3dSession> {
    assertWebGpuAvailable();
    if (this.#series.length === 0) {
      throw new Error(
        "createPlot3d(): call scatter3d(), line3d(), surface(), or wireframe() before mount()",
      );
    }

    const htmlCanvas = isHtmlCanvas(canvas);
    if (!htmlCanvas && options.autoResize) {
      throw new Error("autoResize is only available for an HTMLCanvasElement");
    }
    if (!htmlCanvas && options.bindInput) {
      throw new Error("bindInput is only available for an HTMLCanvasElement");
    }

    const defaultScale =
      htmlCanvas && typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;
    const scaleFactor = finiteScaleFactor(options.scaleFactor, defaultScale);
    if (htmlCanvas) {
      const metrics = getCanvasMetrics(canvas, scaleFactor);
      canvas.width = metrics.width;
      canvas.height = metrics.height;
    }

    const module = await ensureRawModule();
    const rawPlot = this.#toRawPlot(module);
    let rawSession: RawSession3d;
    try {
      rawSession = htmlCanvas
        ? await module.WebGPU3DCanvasSession.create(canvas, rawPlot)
        : await module.OffscreenWebGPU3DCanvasSession.create(canvas, rawPlot);
    } finally {
      rawPlot.free();
    }

    const session = new MountedPlot3dSession(rawSession, canvas, scaleFactor, options.onError);
    try {
      session.resize(canvas.width, canvas.height, scaleFactor);
      if (htmlCanvas && (options.autoResize ?? true)) {
        session.pushCleanup(installCanvasResize(session, canvas, options.scaleFactor));
      }
      if (htmlCanvas && (options.bindInput ?? true)) {
        session.pushCleanup(bindCanvasInput(session, canvas));
      }
    } catch (error) {
      session.dispose();
      throw error;
    }

    return session;
  }

  #toRawPlot(module: RawModule): RawPlot3d {
    const rawPlot = new module.JsPlot3D();
    try {
      for (const series of this.#series) {
        const x = Float64Array.from(series.x);
        const y = Float64Array.from(series.y);
        const z = Float64Array.from(series.z);
        switch (series.kind) {
          case "scatter3d":
            rawPlot.scatter3d(x, y, z);
            break;
          case "line3d":
            rawPlot.line3d(x, y, z);
            break;
          case "surface":
            rawPlot.surface(x, y, z);
            break;
          case "wireframe":
            rawPlot.wireframe(x, y, z);
            break;
        }
      }
      if (this.#axisAspect === "data") rawPlot.equal_scale();
      else if (this.#axisAspect !== null) rawPlot.axis_aspect(...this.#axisAspect);
      rawPlot.stable_scale(this.#stableScale);
      if (this.#title !== null) {
        rawPlot.title(this.#title);
      }
      return rawPlot;
    } catch (error) {
      rawPlot.free();
      throw error;
    }
  }
}

/** Create a fluent high-level WebGPU 3D plot. */
export function createPlot3d(): Plot3dBuilder {
  return new Plot3dBuilder();
}

/** Create a WebGPU 3D scatter plot. */
export function scatter3d(x: NumericArray, y: NumericArray, z: NumericArray): Plot3dBuilder {
  return createPlot3d().scatter3d(x, y, z);
}

/** Create a WebGPU 3D line plot. */
export function line3d(x: NumericArray, y: NumericArray, z: NumericArray): Plot3dBuilder {
  return createPlot3d().line3d(x, y, z);
}

/** Create a WebGPU 3D surface plot. */
export function surface(x: NumericArray, y: NumericArray, z: GridValues3d): Plot3dBuilder {
  return createPlot3d().surface(x, y, z);
}

/** Create a WebGPU 3D wireframe plot. */
export function wireframe(x: NumericArray, y: NumericArray, z: GridValues3d): Plot3dBuilder {
  return createPlot3d().wireframe(x, y, z);
}
