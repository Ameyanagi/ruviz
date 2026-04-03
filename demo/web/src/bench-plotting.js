import { createPlot } from "ruviz";
import initRaw, * as raw from "ruviz/raw";
import { buildRawPlotFromSnapshot } from "../../../packages/ruviz-web/src/plot-runtime.ts";

let rawModulePromise = null;

function setStatus(message) {
  const status = document.getElementById("status");
  if (status) {
    status.textContent = message;
  }
}

async function ensureRawModule() {
  if (!rawModulePromise) {
    rawModulePromise = initRaw().then(() => {
      raw.register_default_browser_fonts_js();
      return raw;
    });
  }
  return rawModulePromise;
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

async function lineWave(points) {
  const x = new Float64Array(points);
  const y = new Float64Array(points);
  const divisor = Math.max(points - 1, 1);

  for (let index = 0; index < points; index += 1) {
    x[index] = index * (200 / divisor);
    y[index] =
      triangleWave(index, 1024) + 0.35 * triangleWave(index, 257) + 0.1 * triangleWave(index, 61);
  }

  return {
    x,
    y,
    hash: await sha256F64Arrays([x, y]),
    elements: points,
  };
}

async function scatterCloud(points) {
  const x = new Float64Array(points);
  const y = new Float64Array(points);
  const modulus = 2147483647;

  for (let index = 0; index < points; index += 1) {
    const xRaw = (index * 48271) % modulus;
    const noiseRaw = (index * 69621 + 12345) % modulus;
    const xValue = xRaw / modulus;
    const noise = noiseRaw / modulus;
    const band = (index % 11) / 10 - 0.5;
    x[index] = xValue;
    y[index] = Math.min(1, Math.max(0, 0.62 * xValue + 0.25 * noise + 0.13 * band));
  }

  return {
    x,
    y,
    hash: await sha256F64Arrays([x, y]),
    elements: points,
  };
}

async function histogramSignal(samples) {
  const values = new Float64Array(samples);
  const modulus = 2147483647;

  for (let index = 0; index < samples; index += 1) {
    const a = ((index * 1103515245 + 12345) % modulus) / modulus;
    const b = ((index * 214013 + 2531011) % modulus) / modulus;
    const cluster = (index % 17) / 16;
    values[index] = (0.55 * a + 0.35 * b + 0.1 * cluster) * 10 - 5;
  }

  return {
    values,
    hash: await sha256F64Arrays([values]),
    elements: samples,
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
    flat,
    hash: await sha256F64Arrays([flat]),
    elements: rows * cols,
  };
}

async function buildDataset(run) {
  const { size } = run;
  switch (run.datasetKind) {
    case "line_wave":
      return lineWave(size.points);
    case "scatter_cloud":
      return scatterCloud(size.points);
    case "histogram_signal":
      return histogramSignal(size.samples);
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
  if (Number.isFinite(size.samples)) {
    return Number(size.samples);
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
        elements: elementCount(size),
      });
    }
  }
  return runs;
}

function buildPlot(run, dataset) {
  const plot = createPlot().sizePx(run.canvas.width, run.canvas.height).theme("light");
  switch (run.plotKind) {
    case "line":
      return plot.line({ x: dataset.x, y: dataset.y });
    case "scatter":
      return plot.scatter({ x: dataset.x, y: dataset.y });
    case "histogram":
      return plot.histogram(dataset.values);
    case "heatmap":
      return plot.heatmap(dataset.matrix);
    default:
      throw new Error(`unsupported plot kind: ${run.plotKind}`);
  }
}

async function measure(warmupIterations, measuredIterations, fn) {
  for (let index = 0; index < warmupIterations; index += 1) {
    await fn();
  }

  const iterationsMs = [];
  let lastLength = 0;
  for (let index = 0; index < measuredIterations; index += 1) {
    const start = performance.now();
    const payload = await fn();
    iterationsMs.push(performance.now() - start);
    lastLength = payload.length;
  }
  return { iterationsMs, byteCount: lastLength };
}

async function benchmarkRun(run) {
  const module = await ensureRawModule();
  const dataset = await buildDataset(run);
  const builtPlot = buildPlot(run, dataset);
  const snapshot = builtPlot.toSnapshot();
  const rawPlot = buildRawPlotFromSnapshot(snapshot, module);

  try {
    const renderOnly = await measure(run.warmupIterations, run.measuredIterations, async () =>
      rawPlot.render_png_bytes(),
    );
    const publicRender = await measure(run.warmupIterations, run.measuredIterations, async () =>
      buildPlot(run, dataset).renderPng(),
    );

    return [
      {
        implementation: "ruviz",
        scenarioId: run.scenarioId,
        plotKind: run.plotKind,
        sizeLabel: run.size.label,
        boundary: "render_only",
        outputTarget: "png_bytes",
        elements: run.elements,
        canvas: run.canvas,
        datasetHash: dataset.hash,
        warmupIterations: run.warmupIterations,
        measuredIterations: run.measuredIterations,
        byteCount: renderOnly.byteCount,
        iterationsMs: renderOnly.iterationsMs,
        summary: summarizeIterations(renderOnly.iterationsMs, run.elements),
      },
      {
        implementation: "ruviz",
        scenarioId: run.scenarioId,
        plotKind: run.plotKind,
        sizeLabel: run.size.label,
        boundary: "public_api_render",
        outputTarget: "png_bytes",
        elements: run.elements,
        canvas: run.canvas,
        datasetHash: dataset.hash,
        warmupIterations: run.warmupIterations,
        measuredIterations: run.measuredIterations,
        byteCount: publicRender.byteCount,
        iterationsMs: publicRender.iterationsMs,
        summary: summarizeIterations(publicRender.iterationsMs, run.elements),
      },
    ];
  } finally {
    rawPlot.free();
  }
}

async function run(manifest, mode = "full") {
  setStatus("initializing");
  await ensureRawModule();

  const runs = scenarioRuns(manifest, mode);
  const results = [];

  for (const runConfig of runs) {
    setStatus(`benchmarking ${runConfig.scenarioId} ${runConfig.size.label}`);
    results.push(...(await benchmarkRun(runConfig)));
  }

  setStatus("complete");
  return {
    schemaVersion: 1,
    runtime: "wasm",
    environment: {
      userAgent: navigator.userAgent,
      platform: navigator.platform,
      hardwareConcurrency: navigator.hardwareConcurrency ?? null,
      deviceMemory: navigator.deviceMemory ?? null,
    },
    results,
  };
}

window.__ruvizPlottingBenchmark = { run };
setStatus("ready");
