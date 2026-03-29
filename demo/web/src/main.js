import {
  createCanvasSession,
  createObservable,
  createPlot,
  createSineSignal,
  createWorkerSession,
  getRuntimeCapabilities,
} from "ruviz";

const TAU = Math.PI * 2;
const TEMPORAL_DURATION = TAU * 2;

const mainCanvas = document.getElementById("main-canvas");
const workerCanvas = document.getElementById("worker-canvas");
const temporalCanvas = document.getElementById("temporal-canvas");
const observableCanvas = document.getElementById("observable-canvas");

const capabilityStatus = document.getElementById("capability-status");
const exportStatus = document.getElementById("export-status");
const mainStatus = document.getElementById("main-status");
const workerStatus = document.getElementById("worker-status");
const temporalStatus = document.getElementById("temporal-status");
const observableStatus = document.getElementById("observable-status");

const mainBackendSelect = document.getElementById("main-backend");
const temporalTimeInput = document.getElementById("temporal-time");
const temporalTimeValue = document.getElementById("temporal-time-value");
const temporalPlayButton = document.getElementById("temporal-play");
const observablePhaseInput = document.getElementById("observable-phase");
const observablePhaseValue = document.getElementById("observable-phase-value");
const observablePlayButton = document.getElementById("observable-play");

function sampleWave(points = 200, extent = Math.PI * 8) {
  const x = Array.from({ length: points }, (_, index) => (index / (points - 1)) * extent);
  const y = x.map((value) => Math.sin(value) * Math.cos(value * 0.3));
  return { x, y };
}

function sampleEvery(x, y, stride) {
  return x.reduce(
    (acc, value, index) => {
      if (index % stride === 0) {
        acc.x.push(value);
        acc.y.push(y[index]);
      }
      return acc;
    },
    { x: [], y: [] },
  );
}

function buildMainPlotTemplate() {
  const { x, y } = sampleWave(220);
  const markers = sampleEvery(x, y, 18);

  return createPlot()
    .setSizePx(640, 360)
    .setTheme("light")
    .setTicks(true)
    .setTitle("Session API coverage")
    .setXLabel("x")
    .setYLabel("y")
    .addLine({ x, y })
    .addScatter({ x: markers.x, y: markers.y });
}

function buildWorkerPlot() {
  const { x, y } = sampleWave(220);
  const markers = sampleEvery(x, y, 20);

  return createPlot()
    .setSizePx(640, 360)
    .setTheme("dark")
    .setTicks(false)
    .setTitle("Worker canvas")
    .setXLabel("x")
    .setYLabel("y")
    .addLine({ x, y })
    .addScatter({ x: markers.x, y: markers.y });
}

function buildTemporalPlot(domain, waveSignal, markerDomain, markerSignal) {
  return createPlot()
    .setSizePx(960, 360)
    .setTheme("dark")
    .setTicks(true)
    .setTitle("Temporal signal playback")
    .setXLabel("x")
    .setYLabel("signal")
    .addLine({ x: domain, y: waveSignal })
    .addScatter({ x: markerDomain, y: markerSignal });
}

function buildObservablePlot(xObservable, yObservable, markerXObservable, markerYObservable) {
  return createPlot()
    .setSizePx(960, 360)
    .setTheme("light")
    .setTicks(false)
    .setTitle("Observable reactive updates")
    .setXLabel("x")
    .setYLabel("y")
    .addLine({ x: xObservable, y: yObservable })
    .addScatter({ x: markerXObservable, y: markerYObservable });
}

function buildDirectExportPlot() {
  const plot = buildMainPlotTemplate().clone();
  plot.setTitle("Direct wasm export");
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

function formatTime(seconds) {
  return `${seconds.toFixed(2)} s`;
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

function describeCapabilities(capabilities) {
  const workerMode =
    capabilities.offscreenCanvasSupported && capabilities.workerSupported
      ? "available"
      : "fallback";
  const webgpuMode = capabilities.gpuCanvasFastPathAvailable
    ? "ready"
    : capabilities.webgpuSupported
      ? "detected, CPU path active"
      : "unavailable";

  return [
    `default font: ${capabilities.defaultBrowserFontRegistered ? "ready" : "unavailable"}`,
    `touch input: ${capabilities.touchInputSupported ? "supported" : "pointer-only"}`,
    `worker canvas: ${workerMode}`,
    `WebGPU: ${webgpuMode}`,
  ].join(" | ");
}

function backendPreference() {
  return mainBackendSelect.value;
}

async function setupMainThreadDemo() {
  const plotTemplate = buildMainPlotTemplate();
  const session = await createCanvasSession(mainCanvas, {
    backendPreference: backendPreference(),
  });

  function syncStatus(mode = session.hasPlot() ? "ready" : "detached") {
    status(
      mainStatus,
      `${mode} | backend ${backendPreference()} | has_plot ${session.hasPlot()}`,
    );
  }

  async function attachPlot() {
    await session.setPlot(plotTemplate.clone());
    session.setBackendPreference(backendPreference());
    session.render();
    syncStatus("ready");
  }

  await attachPlot();

  mainBackendSelect.addEventListener("change", () => {
    session.setBackendPreference(backendPreference());
    if (session.hasPlot()) {
      session.render();
    }
    syncStatus();
  });

  document.getElementById("main-reset").addEventListener("click", () => {
    if (!session.hasPlot()) {
      syncStatus("detached");
      return;
    }
    session.resetView();
    syncStatus("ready");
  });

  document.getElementById("main-export").addEventListener("click", async () => {
    if (!session.hasPlot()) {
      syncStatus("detached");
      return;
    }
    downloadBytes(await session.exportPng(), "ruviz-main-thread.png");
  });

  document.getElementById("main-export-svg").addEventListener("click", async () => {
    if (!session.hasPlot()) {
      syncStatus("detached");
      return;
    }
    downloadText(await session.exportSvg(), "ruviz-main-thread.svg", "image/svg+xml");
  });

  document.getElementById("main-destroy").addEventListener("click", () => {
    session.destroy();
    syncStatus("detached");
  });

  document.getElementById("main-reattach").addEventListener("click", async () => {
    await attachPlot();
  });

  return session;
}

async function setupWorkerDemo() {
  const session = await createWorkerSession(workerCanvas);
  await session.setPlot(buildWorkerPlot());

  status(
    workerStatus,
    session.mode === "worker" ? "ready | worker session" : "ready | main-thread fallback",
  );

  document.getElementById("worker-reset").addEventListener("click", () => {
    session.resetView();
  });

  document.getElementById("worker-export").addEventListener("click", async () => {
    downloadBytes(await session.exportPng(), "ruviz-offscreen.png");
  });

  document.getElementById("worker-export-svg").addEventListener("click", async () => {
    downloadText(await session.exportSvg(), "ruviz-offscreen.svg", "image/svg+xml");
  });

  return session;
}

async function setupTemporalDemo() {
  const domain = Array.from({ length: 256 }, (_, index) => (index / 255) * Math.PI * 8);
  const markerDomain = Array.from({ length: 24 }, (_, index) => (index / 23) * Math.PI * 8);
  const waveSignal = createSineSignal({
    points: domain.length,
    domain: [domain[0], domain[domain.length - 1]],
    amplitude: 0.9,
    cycles: 1,
    phaseVelocity: 1.4,
  });
  const markerSignal = createSineSignal({
    points: markerDomain.length,
    domain: [markerDomain[0], markerDomain[markerDomain.length - 1]],
    amplitude: 0.6,
    cycles: 1,
    phaseVelocity: 1.4,
    phaseOffset: Math.PI / 3,
    verticalOffset: 0.12,
  });
  const session = await createCanvasSession(temporalCanvas, { initialTime: 0 });
  await session.setPlot(buildTemporalPlot(domain, waveSignal, markerDomain, markerSignal));

  let currentTime = 0;
  let playing = false;
  let rafId = null;
  let animationStart = null;

  function updateStatus(mode = playing ? "ready | playing" : "ready") {
    status(
      temporalStatus,
      `${mode} | t=${formatTime(currentTime)} | has_plot ${session.hasPlot()}`,
    );
  }

  function renderTime(nextTime) {
    currentTime = ((nextTime % TEMPORAL_DURATION) + TEMPORAL_DURATION) % TEMPORAL_DURATION;
    temporalTimeInput.value = String(currentTime);
    temporalTimeValue.textContent = formatTime(currentTime);
    session.setTime(currentTime);
    updateStatus();
  }

  function stopPlayback() {
    playing = false;
    animationStart = null;
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
    temporalPlayButton.textContent = "Play";
    updateStatus("ready");
  }

  function animate(timestamp) {
    if (!playing) {
      return;
    }

    if (animationStart === null) {
      animationStart = timestamp - (currentTime / TEMPORAL_DURATION) * 6000;
    }

    const elapsed = timestamp - animationStart;
    renderTime((elapsed / 6000) * TEMPORAL_DURATION);
    rafId = requestAnimationFrame(animate);
  }

  temporalTimeInput.addEventListener("input", () => {
    const nextTime = Number.parseFloat(temporalTimeInput.value);
    stopPlayback();
    renderTime(Number.isFinite(nextTime) ? nextTime : 0);
  });

  temporalPlayButton.addEventListener("click", () => {
    if (playing) {
      stopPlayback();
      return;
    }

    playing = true;
    animationStart = null;
    temporalPlayButton.textContent = "Pause";
    updateStatus("ready | playing");
    rafId = requestAnimationFrame(animate);
  });

  document.getElementById("temporal-reset").addEventListener("click", () => {
    session.resetView();
    updateStatus(playing ? "ready | playing" : "ready");
  });

  document.getElementById("temporal-export").addEventListener("click", async () => {
    downloadBytes(await session.exportPng(), "ruviz-temporal.png");
  });

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      stopPlayback();
    }
  });

  renderTime(0);
  return session;
}

async function setupObservableDemo() {
  const domain = Array.from({ length: 240 }, (_, index) => (index / 239) * TAU * 2.5);
  const markerDomain = domain.filter((_, index) => index % 12 === 0);
  const xObservable = createObservable(domain);
  const yObservable = createObservable(computeObservableSeries(domain, 0));
  const markerXObservable = createObservable(markerDomain);
  const markerYObservable = createObservable(computeObservableSeries(markerDomain, 0));
  const session = await createCanvasSession(observableCanvas);
  await session.setPlot(
    buildObservablePlot(xObservable, yObservable, markerXObservable, markerYObservable),
  );

  let currentPhase = 0;
  let playing = false;
  let rafId = null;
  let animationStart = null;

  function updateStatus(mode = playing ? "ready | playing" : "ready") {
    status(
      observableStatus,
      `${mode} | phase ${formatPhase(currentPhase)} | markers ${markerYObservable.length}`,
    );
  }

  function renderPhase(phase) {
    currentPhase = phase;
    yObservable.replace(computeObservableSeries(domain, phase));

    const markerValues = computeObservableSeries(markerDomain, phase);
    markerValues.forEach((value, index) => {
      markerYObservable.setAt(index, value);
    });

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
    updateStatus("ready");
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
    updateStatus("ready | playing");
    rafId = requestAnimationFrame(animate);
  });

  document.getElementById("observable-reset").addEventListener("click", () => {
    session.resetView();
    updateStatus(playing ? "ready | playing" : "ready");
  });

  document.getElementById("observable-export").addEventListener("click", async () => {
    downloadBytes(await session.exportPng(), "ruviz-observable.png");
  });

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      stopPlayback();
    }
  });

  renderPhase(0);
  return session;
}

async function setupDirectExportDemo() {
  const plot = buildDirectExportPlot();
  status(exportStatus, "ready | public SDK direct export");

  document.getElementById("export-direct-png").addEventListener("click", async () => {
    downloadBytes(await plot.renderPng(), "ruviz-direct-export.png");
  });

  document.getElementById("export-direct-svg").addEventListener("click", async () => {
    downloadText(await plot.renderSvg(), "ruviz-direct-export.svg", "image/svg+xml");
  });

  return plot;
}

async function main() {
  const capabilities = await getRuntimeCapabilities();
  status(capabilityStatus, describeCapabilities(capabilities));
  const directExportPlot = await setupDirectExportDemo();
  const mainSession = await setupMainThreadDemo();
  const workerSession = await setupWorkerDemo();
  const temporalSession = await setupTemporalDemo();
  const observableSession = await setupObservableDemo();

  window.__ruvizDemo = {
    capabilities,
    directExportPlot,
    sdk: {
      createCanvasSession,
      createPlot,
      createSineSignal,
    },
    mainSession,
    workerSession,
    temporalSession,
    observableSession,
  };
}

main().catch((error) => {
  const message = describeError(error);
  console.error(error);
  status(exportStatus, message);
  status(mainStatus, message);
  status(workerStatus, message);
  status(temporalStatus, message);
  status(observableStatus, message);
});
