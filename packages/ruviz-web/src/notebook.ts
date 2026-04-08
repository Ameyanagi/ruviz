import {
  initSync,
  JsPlot,
  WebCanvasSession,
  register_default_browser_fonts_js,
} from "../generated/raw/ruviz_web_raw.js";
import type { PlotSnapshot, YSourceSnapshot } from "./shared.js";

let rawModuleSource: BufferSource | null = null;
let rawModuleInitialized = false;
const DEFAULT_SECONDARY_DRAG_THRESHOLD_PX = 4;

export function configureNotebookRawModuleSource(source: BufferSource): void {
  if (rawModuleInitialized) {
    throw new Error("notebook raw module source must be configured before initialization");
  }

  rawModuleSource = source;
}

function ensureRawModuleInitialized(): void {
  if (rawModuleInitialized) {
    return;
  }

  if (!rawModuleSource) {
    throw new Error("notebook raw module source must be configured before initialization");
  }

  initSync(rawModuleSource);
  register_default_browser_fonts_js();
  rawModuleInitialized = true;
}

function applyPlotMetadata(rawPlot: JsPlot, snapshot: PlotSnapshot): void {
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

function sourceValues(
  source: YSourceSnapshot | { kind: "static" | "observable"; values: number[] },
) {
  if (source.kind === "sine-signal") {
    return sineSignalValues(source);
  }

  return Float64Array.from(source.values);
}

function sineSignalValues(source: Extract<YSourceSnapshot, { kind: "sine-signal" }>): Float64Array {
  const { amplitude, cycles, domainEnd, domainStart, phaseOffset, points, verticalOffset } =
    source.options;
  const span = domainEnd - domainStart;
  const denominator = points <= 1 ? 1 : points - 1;

  return Float64Array.from(
    Array.from({ length: points }, (_, index) => {
      const progress = index / denominator;
      const x = domainStart + span * progress;
      return verticalOffset + amplitude * Math.sin(cycles * x + phaseOffset);
    }),
  );
}

function buildRawPlotFromSnapshot(snapshot: PlotSnapshot): JsPlot {
  const rawPlot = new JsPlot();
  applyPlotMetadata(rawPlot, snapshot);

  for (const series of snapshot.series) {
    switch (series.kind) {
      case "line":
        rawPlot.line(sourceValues(series.x), sourceValues(series.y));
        break;
      case "scatter":
        rawPlot.scatter(sourceValues(series.x), sourceValues(series.y));
        break;
      case "bar":
        rawPlot.bar(series.categories, sourceValues(series.values));
        break;
      case "histogram":
        rawPlot.histogram(sourceValues(series.data));
        break;
      case "boxplot":
        rawPlot.boxplot(sourceValues(series.data));
        break;
      case "heatmap":
        rawPlot.heatmap(Float64Array.from(series.values), series.rows, series.cols);
        break;
      case "error-bars":
        rawPlot.error_bars(
          sourceValues(series.x),
          sourceValues(series.y),
          sourceValues(series.yErrors),
        );
        break;
      case "error-bars-xy":
        rawPlot.error_bars_xy(
          sourceValues(series.x),
          sourceValues(series.y),
          sourceValues(series.xErrors),
          sourceValues(series.yErrors),
        );
        break;
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
  secondaryClick?: (clientX: number, clientY: number) => void;
}

interface CanvasEventOptions {
  secondaryDragThresholdPx?: number;
}

function attachCanvasEvents(
  canvas: HTMLCanvasElement,
  handlers: InputHandlers,
  options: CanvasEventOptions = {},
): () => void {
  const secondaryDragThresholdPx =
    options.secondaryDragThresholdPx ?? DEFAULT_SECONDARY_DRAG_THRESHOLD_PX;
  const secondaryDragThresholdSq = secondaryDragThresholdPx * secondaryDragThresholdPx;
  let secondaryDrag: {
    pointerId: number;
    anchorPoint: { x: number; y: number };
    anchorClientX: number;
    anchorClientY: number;
    dragging: boolean;
  } | null = null;

  const onContextMenu = (event: MouseEvent) => {
    event.preventDefault();
  };

  const onPointerDown = (event: PointerEvent) => {
    const point = pointerPosition(canvas, event);
    canvas.setPointerCapture(event.pointerId);

    if (event.button === 2) {
      secondaryDrag = {
        pointerId: event.pointerId,
        anchorPoint: point,
        anchorClientX: event.clientX,
        anchorClientY: event.clientY,
        dragging: false,
      };
      return;
    }

    handlers.pointerDown(point.x, point.y, event.button);
  };

  const onPointerMove = (event: PointerEvent) => {
    const point = pointerPosition(canvas, event);

    if (secondaryDrag && secondaryDrag.pointerId === event.pointerId) {
      if (!secondaryDrag.dragging) {
        const dx = event.clientX - secondaryDrag.anchorClientX;
        const dy = event.clientY - secondaryDrag.anchorClientY;
        if (dx * dx + dy * dy > secondaryDragThresholdSq) {
          secondaryDrag.dragging = true;
          handlers.pointerDown(secondaryDrag.anchorPoint.x, secondaryDrag.anchorPoint.y, 2);
        }
      }

      if (secondaryDrag.dragging) {
        handlers.pointerMove(point.x, point.y);
      }
      return;
    }

    handlers.pointerMove(point.x, point.y);
  };

  const onPointerUp = (event: PointerEvent) => {
    const point = pointerPosition(canvas, event);
    if (canvas.hasPointerCapture(event.pointerId)) {
      canvas.releasePointerCapture(event.pointerId);
    }

    if (secondaryDrag && secondaryDrag.pointerId === event.pointerId && event.button === 2) {
      if (secondaryDrag.dragging) {
        handlers.pointerUp(point.x, point.y, event.button);
      } else {
        handlers.secondaryClick?.(event.clientX, event.clientY);
      }
      secondaryDrag = null;
      return;
    }

    handlers.pointerUp(point.x, point.y, event.button);
  };

  const onPointerCancel = (event: PointerEvent) => {
    if (canvas.hasPointerCapture(event.pointerId)) {
      canvas.releasePointerCapture(event.pointerId);
    }

    if (secondaryDrag && secondaryDrag.pointerId === event.pointerId) {
      if (secondaryDrag.dragging) {
        handlers.pointerLeave();
      }
      secondaryDrag = null;
      return;
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

export class NotebookCanvasSession {
  readonly mode = "main-thread";
  #canvas: HTMLCanvasElement;
  #rawSession: WebCanvasSession;
  #cleanup: Array<() => void>;

  constructor(canvas: HTMLCanvasElement, options: NotebookCanvasSessionOptions = {}) {
    ensureRawModuleInitialized();
    this.#canvas = canvas;
    this.#rawSession = new WebCanvasSession(canvas);
    this.#cleanup = [];

    this.resize();
    this.#cleanup.push(
      installCanvasResize(canvas, (width, height, scaleFactor) => {
        this.resize(width, height, scaleFactor);
      }),
    );
    this.#cleanup.push(
      attachCanvasEvents(
        canvas,
        {
          pointerDown: (x, y, button) => this.pointerDown(x, y, button),
          pointerMove: (x, y) => this.pointerMove(x, y),
          pointerUp: (x, y, button) => this.pointerUp(x, y, button),
          pointerLeave: () => this.pointerLeave(),
          wheel: (deltaY, x, y) => this.wheel(deltaY, x, y),
          secondaryClick: options.onSecondaryClick,
        },
        {
          secondaryDragThresholdPx: options.secondaryDragThresholdPx,
        },
      ),
    );
  }

  hasPlot(): boolean {
    return this.#rawSession.has_plot();
  }

  async setPlot(snapshot: PlotSnapshot): Promise<void> {
    this.#rawSession.set_plot(buildRawPlotFromSnapshot(snapshot));
  }

  resize(width?: number, height?: number, scaleFactor?: number): void {
    const nextMetrics =
      typeof width === "number" && typeof height === "number" && typeof scaleFactor === "number"
        ? { width, height, scaleFactor }
        : getCanvasMetrics(this.#canvas);

    this.#rawSession.resize(nextMetrics.width, nextMetrics.height, nextMetrics.scaleFactor);
  }

  render(): void {
    if (!this.hasPlot()) {
      return;
    }

    this.#rawSession.render();
  }

  pointerDown(x: number, y: number, button: number): void {
    if (!this.hasPlot()) {
      return;
    }

    this.#rawSession.pointer_down(x, y, button);
  }

  pointerMove(x: number, y: number): void {
    if (!this.hasPlot()) {
      return;
    }

    this.#rawSession.pointer_move(x, y);
  }

  pointerUp(x: number, y: number, button: number): void {
    if (!this.hasPlot()) {
      return;
    }

    this.#rawSession.pointer_up(x, y, button);
  }

  pointerLeave(): void {
    if (!this.hasPlot()) {
      return;
    }

    this.#rawSession.pointer_leave();
  }

  wheel(deltaY: number, x: number, y: number): void {
    if (!this.hasPlot()) {
      return;
    }

    this.#rawSession.wheel(deltaY, x, y);
  }

  async exportPng(): Promise<Uint8Array> {
    if (!this.hasPlot()) {
      throw new Error("no plot is attached to this session");
    }

    return this.#rawSession.export_png();
  }

  async exportSvg(): Promise<string> {
    if (!this.hasPlot()) {
      throw new Error("no plot is attached to this session");
    }

    return this.#rawSession.export_svg();
  }

  dispose(): void {
    for (const dispose of this.#cleanup.splice(0)) {
      dispose();
    }

    this.#rawSession.destroy();
  }
}

export interface NotebookCanvasSessionOptions {
  onSecondaryClick?: (clientX: number, clientY: number) => void;
  secondaryDragThresholdPx?: number;
}

export async function createNotebookCanvasSession(
  canvas: HTMLCanvasElement,
  options: NotebookCanvasSessionOptions = {},
): Promise<NotebookCanvasSession> {
  return new NotebookCanvasSession(canvas, options);
}
