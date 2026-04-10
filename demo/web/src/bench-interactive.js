import { createCanvasSession, createPlot, createSineSignal, getRuntimeCapabilities } from "ruviz";

let workerModuleUrl = null;

function setStatus(message) {
  const status = document.getElementById("status");
  if (status) {
    status.textContent = message;
  }
}

function percentile(sortedValues, ratio) {
  if (sortedValues.length === 0) {
    return 0;
  }
  if (sortedValues.length === 1) {
    return sortedValues[0];
  }

  const position = ratio * (sortedValues.length - 1);
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  if (lower === upper) {
    return sortedValues[lower];
  }

  const weight = position - lower;
  return sortedValues[lower] * (1 - weight) + sortedValues[upper] * weight;
}

function summarizeIterations(iterationsMs, elements) {
  const sortedValues = [...iterationsMs].sort((a, b) => a - b);
  const mean = iterationsMs.reduce((sum, value) => sum + value, 0) / iterationsMs.length;
  const variance =
    iterationsMs.reduce((sum, value) => sum + (value - mean) ** 2, 0) /
    Math.max(iterationsMs.length - 1, 1);
  const median = percentile(sortedValues, 0.5);
  const p95 = percentile(sortedValues, 0.95);
  return {
    meanMs: mean,
    medianMs: median,
    p95Ms: p95,
    minMs: sortedValues[0],
    maxMs: sortedValues[sortedValues.length - 1],
    stdevMs: Math.sqrt(variance),
    throughputElementsPerSec: median <= 0 ? 0 : elements / (median / 1000),
  };
}

async function sha256F64Arrays(arrays) {
  let totalLength = 0;
  for (const array of arrays) {
    totalLength += array.length;
  }

  const buffer = new ArrayBuffer(totalLength * 8);
  const view = new DataView(buffer);
  let offset = 0;
  for (const array of arrays) {
    for (const value of array) {
      view.setFloat64(offset, Number(value), true);
      offset += 8;
    }
  }

  const digest = await crypto.subtle.digest("SHA-256", buffer);
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function triangleWave(index, period) {
  const phase = (index % period) / period;
  return 1 - 4 * Math.abs(phase - 0.5);
}

async function lineSignal(points) {
  const x = new Float64Array(points);
  const divisor = Math.max(points - 1, 1);
  for (let index = 0; index < points; index += 1) {
    x[index] = index * (200 / divisor);
  }

  const signalConfig = new Float64Array([points, 0, 200, 1, 6, 1.5, 0, 0]);
  return {
    x,
    hash: await sha256F64Arrays([x, signalConfig]),
    elements: points,
  };
}

async function scatterSignal(points) {
  const x = new Float64Array(points);
  const modulus = 2147483647;
  for (let index = 0; index < points; index += 1) {
    x[index] = ((index * 48271) % modulus) / modulus;
  }

  const signalConfig = new Float64Array([points, 0, 1, 0.22, 5, 1.1, 0, 0.52]);
  return {
    x,
    hash: await sha256F64Arrays([x, signalConfig]),
    elements: points,
  };
}

async function heatmapField(rows, cols) {
  const matrix = [];
  const flat = new Float64Array(rows * cols);

  for (let row = 0; row < rows; row += 1) {
    const rowValues = new Float64Array(cols);
    for (let col = 0; col < cols; col += 1) {
      const value =
        triangleWave(row, 79) * triangleWave(col, 113) + 0.2 * triangleWave(row * 3 + col * 5, 47);
      rowValues[col] = value;
      flat[row * cols + col] = value;
    }
    matrix.push(rowValues);
  }

  return {
    matrix,
    hash: await sha256F64Arrays([flat]),
    elements: rows * cols,
  };
}

async function buildDataset(run) {
  const { size } = run;
  switch (run.datasetKind) {
    case "line_signal":
      return lineSignal(size.points);
    case "scatter_signal":
      return scatterSignal(size.points);
    case "heatmap_field":
      return heatmapField(size.rows, size.cols);
    default:
      throw new Error(`unsupported dataset kind: ${run.datasetKind}`);
  }
}

function elementCount(size) {
  if (Number.isFinite(size.points)) {
    return Number(size.points);
  }
  return Number(size.rows) * Number(size.cols);
}

function scenarioRuns(manifest, mode) {
  const defaults = manifest.defaults;
  const warmupIterations =
    mode === "smoke" ? defaults.smokeWarmupIterations : defaults.warmupIterations;
  const measuredIterations =
    mode === "smoke" ? defaults.smokeMeasuredIterations : defaults.measuredIterations;

  const runs = [];
  for (const scenario of manifest.scenarios) {
    const sizes = mode === "smoke" ? scenario.sizes.slice(0, 1) : scenario.sizes;
    for (const size of sizes) {
      runs.push({
        scenarioId: scenario.id,
        plotKind: scenario.plotKind,
        datasetKind: scenario.datasetKind,
        canvas: { ...scenario.canvas },
        size: { ...size },
        warmupIterations,
        measuredIterations,
        hoverStridePx: defaults.hoverStridePx,
        panDeltaPx: defaults.panDeltaPx,
        timeStepSeconds: defaults.timeStepSeconds,
        elements: elementCount(size),
      });
    }
  }
  return runs;
}

function buildPlot(run, dataset) {
  const plot = createPlot().sizePx(run.canvas.width, run.canvas.height).theme("light").ticks(false);
  switch (run.plotKind) {
    case "line":
      return plot.line({
        x: dataset.x,
        y: createSineSignal({
          points: dataset.x.length,
          domainStart: 0,
          domainEnd: 200,
          amplitude: 1,
          cycles: 6,
          phaseVelocity: 1.5,
          phaseOffset: 0,
          verticalOffset: 0,
        }),
      });
    case "scatter":
      return plot.scatter({
        x: dataset.x,
        y: createSineSignal({
          points: dataset.x.length,
          domainStart: 0,
          domainEnd: 1,
          amplitude: 0.22,
          cycles: 5,
          phaseVelocity: 1.1,
          phaseOffset: 0,
          verticalOffset: 0.52,
        }),
      });
    case "heatmap":
      return plot.heatmap(dataset.matrix);
    default:
      throw new Error(`unsupported plot kind: ${run.plotKind}`);
  }
}

function createHiddenCanvas(run) {
  const root = document.getElementById("benchmark-root") ?? document.body;
  const canvas = document.createElement("canvas");
  canvas.width = run.canvas.width;
  canvas.height = run.canvas.height;
  canvas.style.width = `${run.canvas.width}px`;
  canvas.style.height = `${run.canvas.height}px`;
  root.appendChild(canvas);
  return canvas;
}

function destroyCanvas(canvas) {
  canvas.remove();
}

function hoverPoint(run, iteration) {
  const strideX = Math.max(1, run.hoverStridePx);
  const strideY = Math.max(1, Math.floor(run.hoverStridePx / 2));
  const centerX = run.canvas.width * 0.5;
  const centerY = run.canvas.height * 0.5;
  const radiusX = Math.max(12, run.canvas.width * 0.22);
  const radiusY = Math.max(12, run.canvas.height * 0.18);
  return {
    x: Math.max(
      8,
      Math.min(run.canvas.width - 8, centerX + Math.sin((iteration * strideX) / radiusX) * radiusX),
    ),
    y: Math.max(
      8,
      Math.min(
        run.canvas.height - 8,
        centerY + Math.cos((iteration * strideY) / radiusY) * radiusY,
      ),
    ),
  };
}

function panPoint(run, iteration) {
  const [dx, dy] = run.panDeltaPx;
  const direction = iteration % 2 === 0 ? 1 : -1;
  return {
    x: run.canvas.width * 0.5 + dx * direction,
    y: run.canvas.height * 0.5 + dy * direction,
  };
}

async function measure(warmupIterations, measuredIterations, fn) {
  let lastByteCount = 0;
  for (let index = 0; index < warmupIterations; index += 1) {
    const result = await fn(index);
    lastByteCount = result?.byteCount ?? 0;
  }

  const iterationsMs = [];
  for (let index = 0; index < measuredIterations; index += 1) {
    const start = performance.now();
    const result = await fn(warmupIterations + index);
    iterationsMs.push(performance.now() - start);
    lastByteCount = result?.byteCount ?? 0;
  }

  return {
    iterationsMs,
    byteCount: lastByteCount,
  };
}

function resultRecord(run, boundary, outputTarget, datasetHash, measurement, sessionMode = null) {
  return {
    implementation: "ruviz",
    scenarioId: run.scenarioId,
    plotKind: run.plotKind,
    sizeLabel: run.size.label,
    boundary,
    outputTarget,
    elements: run.elements,
    canvas: run.canvas,
    datasetHash,
    warmupIterations: run.warmupIterations,
    measuredIterations: run.measuredIterations,
    sessionMode,
    byteCount: measurement.byteCount,
    iterationsMs: measurement.iterationsMs,
    summary: summarizeIterations(measurement.iterationsMs, run.elements),
  };
}

class BenchWorkerSession {
  #canvas;
  #worker;
  #pending;
  #nextRequestId;
  #resolveReady;
  #rejectReady;

  constructor(canvas) {
    this.#canvas = canvas;
    this.#worker = null;
    this.#pending = new Map();
    this.#nextRequestId = 1;
    this.#resolveReady = null;
    this.#rejectReady = null;
  }

  async init() {
    if (!workerModuleUrl) {
      workerModuleUrl = new URL("./bench-session-worker.ts", import.meta.url);
    }

    const worker = new Worker(workerModuleUrl, { type: "module" });
    this.#worker = worker;
    const ready = new Promise((resolve, reject) => {
      this.#resolveReady = resolve;
      this.#rejectReady = reject;
      worker.onmessage = (event) => this.#handleMessage(event);
      worker.onerror = (event) => reject(new Error(event.message || "worker session error"));
      worker.onmessageerror = () => reject(new Error("worker session message decoding failed"));
    });

    const offscreen = this.#canvas.transferControlToOffscreen();
    worker.postMessage(
      {
        type: "init",
        canvas: offscreen,
        width: this.#canvas.width,
        height: this.#canvas.height,
        scaleFactor: 1,
        backendPreference: "auto",
        initialTime: 0,
      },
      [offscreen],
    );
    await ready;
  }

  #handleMessage(event) {
    const { type, requestId, payload } = event.data;
    if (type === "ready") {
      this.#resolveReady?.();
      this.#resolveReady = null;
      this.#rejectReady = null;
      return;
    }

    if (type === "error" && requestId == null) {
      const error = new Error(String(payload ?? "worker session error"));
      if (this.#rejectReady) {
        this.#rejectReady(error);
        this.#resolveReady = null;
        this.#rejectReady = null;
      }
      return;
    }

    const handlers = requestId ? this.#pending.get(requestId) : null;
    if (!handlers) {
      return;
    }

    this.#pending.delete(requestId);
    if (type === "error") {
      handlers.reject(new Error(String(payload ?? "worker error")));
      return;
    }
    handlers.resolve(payload);
  }

  #request(type, payload, transfer = []) {
    if (!this.#worker) {
      throw new Error("worker is not initialized");
    }

    const requestId = this.#nextRequestId++;
    const promise = new Promise((resolve, reject) => {
      this.#pending.set(requestId, { resolve, reject });
    });
    this.#worker.postMessage({ type, requestId, payload }, transfer);
    return promise;
  }

  #post(type, payload) {
    if (!this.#worker) {
      throw new Error("worker is not initialized");
    }
    this.#worker.postMessage({ type, payload });
  }

  async setPlot(snapshot) {
    await this.#request("setPlot", { snapshot });
  }

  async setTime(timeSeconds) {
    await this.#request("setTime", { timeSeconds });
  }

  pointerDown(x, y, button) {
    this.#post("pointerDown", { x, y, button });
  }

  async pointerMoveAndWait(x, y) {
    this.#post("pointerMove", { x, y });
    await this.#request("flush", {});
  }

  async pointerUpAndWait(x, y, button) {
    this.#post("pointerUp", { x, y, button });
    await this.#request("flush", {});
  }

  async resetView() {
    await this.#request("resetView", {});
  }

  async exportPng() {
    return this.#request("exportPng", {});
  }

  async destroy() {
    if (!this.#worker) {
      return;
    }
    try {
      await this.#request("destroy", {});
    } finally {
      this.#worker.terminate();
      this.#worker = null;
    }
  }
}

async function benchmarkCanvasSession(run, dataset, plot) {
  const canvas = createHiddenCanvas(run);
  const session = await createCanvasSession(canvas, {
    autoResize: false,
    bindInput: false,
  });

  try {
    await session.setPlot(plot);
    const results = [];

    const setPlotMeasurement = await measure(
      run.warmupIterations,
      run.measuredIterations,
      async () => {
        await session.setPlot(plot);
        return { byteCount: 0 };
      },
    );
    results.push(
      resultRecord(
        run,
        "canvas_session_set_plot",
        "interactive_frame",
        dataset.hash,
        setPlotMeasurement,
        "main-thread",
      ),
    );

    const exportMeasurement = await measure(
      run.warmupIterations,
      run.measuredIterations,
      async () => {
        const png = await session.exportPng();
        return { byteCount: png.length };
      },
    );
    results.push(
      resultRecord(
        run,
        "canvas_session_export_png",
        "png_bytes",
        dataset.hash,
        exportMeasurement,
        "main-thread",
      ),
    );

    session.pointerLeave();
    const hoverMeasurement = await measure(
      run.warmupIterations,
      run.measuredIterations,
      async (iteration) => {
        const point = hoverPoint(run, iteration);
        session.pointerMove(point.x, point.y);
        return { byteCount: 0 };
      },
    );
    results.push(
      resultRecord(
        run,
        "canvas_session_hover",
        "interactive_frame",
        dataset.hash,
        hoverMeasurement,
        "main-thread",
      ),
    );

    session.resetView();
    const start = { x: run.canvas.width * 0.5, y: run.canvas.height * 0.5 };
    session.pointerDown(start.x, start.y, 0);
    const panMeasurement = await measure(
      run.warmupIterations,
      run.measuredIterations,
      async (iteration) => {
        const point = panPoint(run, iteration);
        session.pointerMove(point.x, point.y);
        return { byteCount: 0 };
      },
    );
    session.pointerUp(start.x, start.y, 0);
    results.push(
      resultRecord(
        run,
        "canvas_session_pan",
        "interactive_frame",
        dataset.hash,
        panMeasurement,
        "main-thread",
      ),
    );

    if (run.plotKind !== "heatmap") {
      const timeMeasurement = await measure(
        run.warmupIterations,
        run.measuredIterations,
        async (iteration) => {
          session.setTime((iteration + 1) * run.timeStepSeconds);
          return { byteCount: 0 };
        },
      );
      results.push(
        resultRecord(
          run,
          "canvas_session_time",
          "interactive_frame",
          dataset.hash,
          timeMeasurement,
          "main-thread",
        ),
      );
      session.setTime(0);
    }

    return results;
  } finally {
    session.dispose();
    destroyCanvas(canvas);
  }
}

async function benchmarkWorkerSession(run, dataset, snapshot) {
  const canvas = createHiddenCanvas(run);
  const session = new BenchWorkerSession(canvas);
  await session.init();

  try {
    await session.setPlot(snapshot);
    const results = [];

    const setPlotMeasurement = await measure(
      run.warmupIterations,
      run.measuredIterations,
      async () => {
        await session.setPlot(snapshot);
        return { byteCount: 0 };
      },
    );
    results.push(
      resultRecord(
        run,
        "worker_session_set_plot",
        "interactive_frame",
        dataset.hash,
        setPlotMeasurement,
        "worker",
      ),
    );

    const exportMeasurement = await measure(
      run.warmupIterations,
      run.measuredIterations,
      async () => {
        const png = await session.exportPng();
        return { byteCount: png.length };
      },
    );
    results.push(
      resultRecord(
        run,
        "worker_session_export_png",
        "png_bytes",
        dataset.hash,
        exportMeasurement,
        "worker",
      ),
    );

    const hoverMeasurement = await measure(
      run.warmupIterations,
      run.measuredIterations,
      async (iteration) => {
        const point = hoverPoint(run, iteration);
        await session.pointerMoveAndWait(point.x, point.y);
        return { byteCount: 0 };
      },
    );
    results.push(
      resultRecord(
        run,
        "worker_session_hover",
        "interactive_frame",
        dataset.hash,
        hoverMeasurement,
        "worker",
      ),
    );

    await session.resetView();
    const start = { x: run.canvas.width * 0.5, y: run.canvas.height * 0.5 };
    session.pointerDown(start.x, start.y, 0);
    const panMeasurement = await measure(
      run.warmupIterations,
      run.measuredIterations,
      async (iteration) => {
        const point = panPoint(run, iteration);
        await session.pointerMoveAndWait(point.x, point.y);
        return { byteCount: 0 };
      },
    );
    await session.pointerUpAndWait(start.x, start.y, 0);
    results.push(
      resultRecord(
        run,
        "worker_session_pan",
        "interactive_frame",
        dataset.hash,
        panMeasurement,
        "worker",
      ),
    );

    if (run.plotKind !== "heatmap") {
      const timeMeasurement = await measure(
        run.warmupIterations,
        run.measuredIterations,
        async (iteration) => {
          await session.setTime((iteration + 1) * run.timeStepSeconds);
          return { byteCount: 0 };
        },
      );
      results.push(
        resultRecord(
          run,
          "worker_session_time",
          "interactive_frame",
          dataset.hash,
          timeMeasurement,
          "worker",
        ),
      );
      await session.setTime(0);
    }

    return results;
  } finally {
    await session.destroy();
    destroyCanvas(canvas);
  }
}

async function benchmarkRun(run) {
  const dataset = await buildDataset(run);
  const plot = buildPlot(run, dataset);
  const snapshot = plot.toSnapshot();

  const builderExportMeasurement = await measure(
    run.warmupIterations,
    run.measuredIterations,
    async () => {
      const png = await plot.renderPng();
      return { byteCount: png.length };
    },
  );

  const results = [
    resultRecord(
      run,
      "plot_builder_export_png",
      "png_bytes",
      dataset.hash,
      builderExportMeasurement,
    ),
  ];
  const warnings = [];

  results.push(...(await benchmarkCanvasSession(run, dataset, plot)));
  try {
    results.push(...(await benchmarkWorkerSession(run, dataset, snapshot)));
  } catch (error) {
    warnings.push(
      `${run.scenarioId} ${run.size.label}: worker benchmark unavailable (${error instanceof Error ? error.message : String(error)})`,
    );
  }
  return { results, warnings };
}

async function run(manifest, mode = "full") {
  setStatus("initializing");
  const capabilities = await getRuntimeCapabilities();
  const runs = scenarioRuns(manifest, mode);
  const results = [];
  const warnings = [];

  for (const runConfig of runs) {
    setStatus(`benchmarking ${runConfig.scenarioId} ${runConfig.size.label}`);
    const runResult = await benchmarkRun(runConfig);
    results.push(...runResult.results);
    warnings.push(...runResult.warnings);
  }

  setStatus("complete");
  return {
    schemaVersion: 1,
    suite: "interactive",
    runtime: "wasm",
    warnings,
    environment: {
      userAgent: navigator.userAgent,
      platform: navigator.platform,
      hardwareConcurrency: navigator.hardwareConcurrency ?? null,
      deviceMemory: navigator.deviceMemory ?? null,
      runtimeCapabilities: capabilities,
    },
    results,
  };
}

window.__ruvizInteractiveBenchmark = { run };
setStatus("ready");
