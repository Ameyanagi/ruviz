import init, {
  JsPlot,
  ObservableVecF64,
  WebCanvasSession,
  web_runtime_capabilities,
} from "../pkg/ruviz_web.js";

const TAU = Math.PI * 2;

const mainCanvas = document.getElementById("main-canvas");
const workerCanvas = document.getElementById("worker-canvas");
const observableCanvas = document.getElementById("observable-canvas");

const capabilityStatus = document.getElementById("capability-status");
const exportStatus = document.getElementById("export-status");
const mainStatus = document.getElementById("main-status");
const workerStatus = document.getElementById("worker-status");
const observableStatus = document.getElementById("observable-status");

const observablePhaseInput = document.getElementById("observable-phase");
const observablePhaseValue = document.getElementById("observable-phase-value");
const observablePlayButton = document.getElementById("observable-play");

function sampleWave(points = 200) {
  const x = Array.from({ length: points }, (_, i) => (i / (points - 1)) * Math.PI * 8);
  const y = x.map((value) => Math.sin(value) * Math.cos(value * 0.3));
  return { x, y };
}

function buildMainPlot() {
  const plot = new JsPlot();
  const { x, y } = sampleWave();

  plot.size_px(640, 360);
  plot.ticks(false);
  plot.theme_light();
  plot.xlabel("x");
  plot.ylabel("y");
  plot.line(x, y);

  return plot;
}

function buildWorkerPlot() {
  const plot = new JsPlot();
  const { x, y } = sampleWave();

  plot.size_px(640, 360);
  plot.ticks(false);
  plot.theme_dark();
  plot.xlabel("x");
  plot.ylabel("y");
  plot.line(x, y);

  return plot;
}

function buildObservablePlot(xObservable, yObservable) {
  const plot = new JsPlot();
  plot.size_px(960, 360);
  plot.ticks(false);
  plot.theme_light();
  plot.xlabel("x");
  plot.ylabel("y");
  plot.line_observable(xObservable, yObservable);
  return plot;
}

function buildDirectExportPlot() {
  const plot = new JsPlot();
  const { x, y } = sampleWave();

  plot.size_px(800, 420);
  plot.theme_light();
  plot.xlabel("x");
  plot.ylabel("y");
  plot.title("Direct wasm export");
  plot.line(x, y);

  return plot;
}

function computeObservableSeries(x, phase) {
  return x.map(
    (value) => 0.85 * Math.sin(value + phase) + 0.35 * Math.cos(value * 0.55 - phase * 0.6),
  );
}

function status(node, message) {
  node.textContent = message;
}

function describeError(error) {
  if (error && typeof error === "object") {
    if ("stack" in error && error.stack) {
      return String(error.stack);
    }
    if ("message" in error && error.message) {
      return String(error.message);
    }
  }
  return String(error);
}

function formatPhase(phase) {
  return `${phase.toFixed(2)} rad`;
}

function downloadBytes(bytes, filename) {
  const blob = new Blob([bytes], { type: "image/png" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

function downloadText(text, filename, mimeType) {
  const blob = new Blob([text], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

function getCanvasMetrics(canvas) {
  const rect = canvas.getBoundingClientRect();
  const scaleFactor = window.devicePixelRatio || 1;
  const width = Math.max(1, Math.round(rect.width * scaleFactor));
  const height = Math.max(1, Math.round(rect.height * scaleFactor));

  return { width, height, scaleFactor, rect };
}

function pointerPosition(canvas, event) {
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;

  return {
    x: (event.clientX - rect.left) * scaleX,
    y: (event.clientY - rect.top) * scaleY,
  };
}

function installCanvasResize(canvas, onResize) {
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

  const observer = new ResizeObserver(() => sync());
  observer.observe(canvas);
  window.addEventListener("resize", sync);

  return () => {
    observer.disconnect();
    window.removeEventListener("resize", sync);
  };
}

function describeCapabilities(capabilities) {
  const workerMode =
    capabilities.offscreen_canvas_supported && capabilities.worker_supported
      ? "available"
      : "fallback";
  const webgpuMode = capabilities.gpu_canvas_fast_path_available
    ? "ready"
    : capabilities.webgpu_supported
      ? "detected, CPU path active"
      : "unavailable";

  return [
    `default font: ${capabilities.default_browser_font_registered ? "ready" : "unavailable"}`,
    `touch input: ${capabilities.touch_input_supported ? "supported" : "pointer-only"}`,
    `worker canvas: ${workerMode}`,
    `WebGPU: ${webgpuMode}`,
  ].join(" | ");
}

function attachCanvasEvents(canvas, handlers) {
  canvas.addEventListener("contextmenu", (event) => event.preventDefault());

  canvas.addEventListener("pointerdown", (event) => {
    const point = pointerPosition(canvas, event);
    canvas.setPointerCapture(event.pointerId);
    handlers.pointer_down(point.x, point.y, event.button);
  });

  canvas.addEventListener("pointermove", (event) => {
    const point = pointerPosition(canvas, event);
    handlers.pointer_move(point.x, point.y);
  });

  canvas.addEventListener("pointerup", (event) => {
    const point = pointerPosition(canvas, event);
    if (canvas.hasPointerCapture(event.pointerId)) {
      canvas.releasePointerCapture(event.pointerId);
    }
    handlers.pointer_up(point.x, point.y, event.button);
  });

  canvas.addEventListener("pointercancel", (event) => {
    if (canvas.hasPointerCapture(event.pointerId)) {
      canvas.releasePointerCapture(event.pointerId);
    }
    handlers.pointer_leave();
  });

  canvas.addEventListener("pointerleave", () => {
    handlers.pointer_leave();
  });

  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      const point = pointerPosition(canvas, event);
      handlers.wheel(event.deltaY, point.x, point.y);
    },
    { passive: false },
  );
}

async function setupMainThreadDemo() {
  let session;
  try {
    session = new WebCanvasSession(mainCanvas);
  } catch (error) {
    status(mainStatus, `main:new: ${describeError(error)}`);
    throw error;
  }

  installCanvasResize(mainCanvas, (width, height, scaleFactor) => {
    session.resize(width, height, scaleFactor);
  });

  try {
    session.set_plot(buildMainPlot());
  } catch (error) {
    status(mainStatus, `main:set_plot: ${describeError(error)}`);
    throw error;
  }

  status(mainStatus, "ready");

  attachCanvasEvents(mainCanvas, {
    pointer_down: (x, y, button) => session.pointer_down(x, y, button),
    pointer_move: (x, y) => session.pointer_move(x, y),
    pointer_up: (x, y, button) => session.pointer_up(x, y, button),
    pointer_leave: () => session.pointer_leave(),
    wheel: (deltaY, x, y) => session.wheel(deltaY, x, y),
  });

  document.getElementById("main-reset").addEventListener("click", () => {
    session.reset_view();
  });

  document.getElementById("main-export").addEventListener("click", () => {
    downloadBytes(session.export_png(), "ruviz-main-thread.png");
  });
}

async function setupWorkerFallbackDemo() {
  const session = new WebCanvasSession(workerCanvas);

  installCanvasResize(workerCanvas, (width, height, scaleFactor) => {
    session.resize(width, height, scaleFactor);
  });

  session.set_plot(buildWorkerPlot());
  status(workerStatus, "ready | main-thread fallback");

  attachCanvasEvents(workerCanvas, {
    pointer_down: (x, y, button) => session.pointer_down(x, y, button),
    pointer_move: (x, y) => session.pointer_move(x, y),
    pointer_up: (x, y, button) => session.pointer_up(x, y, button),
    pointer_leave: () => session.pointer_leave(),
    wheel: (deltaY, x, y) => session.wheel(deltaY, x, y),
  });

  document.getElementById("worker-reset").addEventListener("click", () => {
    session.reset_view();
  });

  document.getElementById("worker-export").addEventListener("click", () => {
    downloadBytes(session.export_png(), "ruviz-offscreen.png");
  });
}

async function setupWorkerDemo(capabilities) {
  const supportsWorkerCanvas =
    capabilities.offscreen_canvas_supported &&
    capabilities.worker_supported &&
    "transferControlToOffscreen" in HTMLCanvasElement.prototype;

  if (!supportsWorkerCanvas) {
    await setupWorkerFallbackDemo();
    return;
  }

  const worker = new Worker(new URL("./offscreen-worker.js", import.meta.url), {
    type: "module",
  });

  worker.onerror = (event) => {
    status(workerStatus, `worker:error: ${event.message || "unknown error"}`);
  };

  worker.onmessageerror = () => {
    status(workerStatus, "worker:error: failed to decode worker message");
  };

  worker.onmessage = (event) => {
    const { type, payload } = event.data;
    if (type === "ready") {
      status(workerStatus, "ready");
      return;
    }

    if (type === "error") {
      status(workerStatus, payload);
      return;
    }

    if (type === "png") {
      downloadBytes(payload, "ruviz-offscreen.png");
    }
  };

  const offscreen = workerCanvas.transferControlToOffscreen();
  const initialMetrics = getCanvasMetrics(workerCanvas);
  worker.postMessage(
    {
      type: "init",
      canvas: offscreen,
      width: initialMetrics.width,
      height: initialMetrics.height,
      devicePixelRatio: initialMetrics.scaleFactor,
    },
    [offscreen],
  );

  installCanvasResize(workerCanvas, (width, height, scaleFactor) => {
    worker.postMessage({
      type: "resize",
      payload: { width, height, scaleFactor },
    });
  });

  attachCanvasEvents(workerCanvas, {
    pointer_down: (x, y, button) =>
      worker.postMessage({ type: "pointer_down", payload: { x, y, button } }),
    pointer_move: (x, y) => worker.postMessage({ type: "pointer_move", payload: { x, y } }),
    pointer_up: (x, y, button) =>
      worker.postMessage({ type: "pointer_up", payload: { x, y, button } }),
    pointer_leave: () => worker.postMessage({ type: "pointer_leave" }),
    wheel: (deltaY, x, y) => worker.postMessage({ type: "wheel", payload: { deltaY, x, y } }),
  });

  document.getElementById("worker-reset").addEventListener("click", () => {
    worker.postMessage({ type: "reset" });
  });

  document.getElementById("worker-export").addEventListener("click", () => {
    worker.postMessage({ type: "export_png" });
  });
}

async function setupObservableDemo() {
  const domain = Array.from({ length: 240 }, (_, i) => (i / 239) * TAU * 2.5);
  const xObservable = new ObservableVecF64(domain);
  const yObservable = new ObservableVecF64(computeObservableSeries(domain, 0));
  const session = new WebCanvasSession(observableCanvas);
  installCanvasResize(observableCanvas, (width, height, scaleFactor) => {
    session.resize(width, height, scaleFactor);
  });
  session.set_plot(buildObservablePlot(xObservable, yObservable));

  let currentPhase = 0;
  let playing = false;
  let rafId = null;
  let animationStart = null;

  function updateStatus() {
    status(
      observableStatus,
      playing
        ? `ready | playing | phase ${formatPhase(currentPhase)}`
        : `ready | phase ${formatPhase(currentPhase)}`,
    );
  }

  function renderPhase(phase) {
    currentPhase = phase;
    yObservable.replace(computeObservableSeries(domain, phase));
    observablePhaseInput.value = String(phase);
    observablePhaseValue.textContent = formatPhase(phase);
    session.render();
    updateStatus();
  }

  function stopPlayback() {
    playing = false;
    animationStart = null;
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
    observablePlayButton.textContent = "Play";
    updateStatus();
  }

  function animate(timestamp) {
    if (!playing) {
      return;
    }

    if (animationStart === null) {
      animationStart = timestamp - (currentPhase / TAU) * 4000;
    }

    const elapsed = timestamp - animationStart;
    renderPhase(((elapsed / 4000) * TAU) % TAU);
    rafId = requestAnimationFrame(animate);
  }

  attachCanvasEvents(observableCanvas, {
    pointer_down: (x, y, button) => session.pointer_down(x, y, button),
    pointer_move: (x, y) => session.pointer_move(x, y),
    pointer_up: (x, y, button) => session.pointer_up(x, y, button),
    pointer_leave: () => session.pointer_leave(),
    wheel: (deltaY, x, y) => session.wheel(deltaY, x, y),
  });

  observablePhaseInput.addEventListener("input", () => {
    const nextPhase = Number.parseFloat(observablePhaseInput.value);
    stopPlayback();
    renderPhase(Number.isFinite(nextPhase) ? nextPhase : 0);
  });

  observablePlayButton.addEventListener("click", () => {
    if (playing) {
      stopPlayback();
      return;
    }

    playing = true;
    animationStart = null;
    observablePlayButton.textContent = "Pause";
    updateStatus();
    rafId = requestAnimationFrame(animate);
  });

  document.getElementById("observable-reset").addEventListener("click", () => {
    session.reset_view();
  });

  document.getElementById("observable-export").addEventListener("click", () => {
    downloadBytes(session.export_png(), "ruviz-observable.png");
  });

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      stopPlayback();
    }
  });

  renderPhase(0);
}

async function setupDirectExportDemo() {
  const plot = buildDirectExportPlot();
  status(exportStatus, "ready");

  document.getElementById("export-direct-png").addEventListener("click", () => {
    downloadBytes(plot.render_png_bytes(), "ruviz-direct-export.png");
  });

  document.getElementById("export-direct-svg").addEventListener("click", () => {
    downloadText(plot.render_svg(), "ruviz-direct-export.svg", "image/svg+xml");
  });
}

async function main() {
  await init();
  const capabilities = web_runtime_capabilities();
  status(capabilityStatus, describeCapabilities(capabilities));
  await setupDirectExportDemo();
  await setupMainThreadDemo();
  await setupWorkerDemo(capabilities);
  await setupObservableDemo();
}

main().catch((error) => {
  const message = describeError(error);
  console.error(error);
  status(mainStatus, message);
  status(workerStatus, message);
  status(observableStatus, message);
});
