import initRaw, {
  JsPlot3D,
  WebGPU3DCanvasSession,
  register_default_browser_fonts_js,
} from "ruviz/raw";

const mainCanvas = document.getElementById("main-3d");
const workerCanvas = document.getElementById("worker-3d");
const mainStatus = document.getElementById("main-3d-status");
const workerStatus = document.getElementById("worker-3d-status");

function describeError(error) {
  return error instanceof Error ? error.message : String(error);
}

function surfaceData(size) {
  const axis = Float64Array.from({ length: size }, (_, index) => -3 + (index / (size - 1)) * 6);
  const z = new Float64Array(size * size);
  for (let row = 0; row < size; row += 1) {
    for (let column = 0; column < size; column += 1) {
      const x = axis[column];
      const y = axis[row];
      const radius = Math.hypot(x, y);
      z[row * size + column] = Math.cos(radius * 2) * Math.exp(-radius * 0.28);
    }
  }
  return { axis, z };
}

function plot3d(title) {
  const { axis, z } = surfaceData(48);
  const plot = new JsPlot3D();
  plot.surface(axis, axis, z);
  plot.title(title);
  return plot;
}

function backingPoint(canvas, event) {
  const bounds = canvas.getBoundingClientRect();
  return [
    ((event.clientX - bounds.left) * canvas.width) / bounds.width,
    ((event.clientY - bounds.top) * canvas.height) / bounds.height,
  ];
}

function createInputScheduler(session, requestFrame = requestAnimationFrame) {
  let frame = 0;
  let latestMove = null;
  let wheelDelta = 0;
  let appliedMoves = 0;
  let renderCalls = 0;
  let presentedFrames = 0;

  function flush() {
    frame = 0;
    if (latestMove) {
      session.pointer_move(latestMove[0], latestMove[1]);
      latestMove = null;
      appliedMoves += 1;
    }
    if (wheelDelta !== 0) {
      session.wheel(wheelDelta);
      wheelDelta = 0;
    }
    renderCalls += 1;
    if (session.render()) {
      presentedFrames += 1;
    }
  }

  function schedule() {
    if (frame === 0) {
      frame = requestFrame(flush);
    }
  }

  return {
    pointerDown(x, y, button) {
      session.pointer_down(x, y, button);
    },
    pointerMove(x, y) {
      latestMove = [x, y];
      schedule();
    },
    pointerUp(x, y, button) {
      if (latestMove) {
        session.pointer_move(latestMove[0], latestMove[1]);
        latestMove = null;
        appliedMoves += 1;
      }
      session.pointer_up(x, y, button);
      schedule();
    },
    wheel(delta) {
      wheelDelta += delta;
      schedule();
    },
    resize(width, height, scale) {
      session.resize(width, height, scale);
      schedule();
    },
    reset() {
      session.reset_view();
      schedule();
    },
    metrics() {
      return { appliedMoves, renderCalls, presentedFrames };
    },
  };
}

function bindCanvas(canvas, scheduler) {
  canvas.addEventListener("pointerdown", (event) => {
    canvas.setPointerCapture(event.pointerId);
    scheduler.pointerDown(...backingPoint(canvas, event), event.button);
  });
  canvas.addEventListener("pointermove", (event) => {
    scheduler.pointerMove(...backingPoint(canvas, event));
  });
  canvas.addEventListener("pointerup", (event) => {
    scheduler.pointerUp(...backingPoint(canvas, event), event.button);
  });
  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      scheduler.wheel(event.deltaY);
    },
    { passive: false },
  );
}

function resizeMainCanvas(canvas, scheduler) {
  const scale = window.devicePixelRatio || 1;
  const bounds = canvas.getBoundingClientRect();
  scheduler.resize(
    Math.max(1, Math.round(bounds.width * scale)),
    Math.max(1, Math.round(bounds.height * scale)),
    scale,
  );
}

function createWorkerInputScheduler(worker) {
  let frame = 0;
  let latestMove = null;
  let wheelDelta = 0;
  let sentMoves = 0;

  function flush() {
    frame = 0;
    if (latestMove) {
      worker.postMessage({ type: "pointerMove", ...latestMove });
      latestMove = null;
      sentMoves += 1;
    }
    if (wheelDelta !== 0) {
      worker.postMessage({ type: "wheel", delta: wheelDelta });
      wheelDelta = 0;
    }
  }

  function schedule() {
    if (frame === 0) {
      frame = requestAnimationFrame(flush);
    }
  }

  return {
    pointerDown(x, y, button) {
      worker.postMessage({ type: "pointerDown", x, y, button });
    },
    pointerMove(x, y) {
      latestMove = { x, y };
      schedule();
    },
    pointerUp(x, y, button) {
      if (latestMove) {
        worker.postMessage({ type: "pointerMove", ...latestMove });
        latestMove = null;
        sentMoves += 1;
      }
      worker.postMessage({ type: "pointerUp", x, y, button });
      schedule();
    },
    wheel(delta) {
      wheelDelta += delta;
      schedule();
    },
    metrics() {
      return { sentMoves };
    },
  };
}

async function setupMain() {
  await initRaw();
  register_default_browser_fonts_js();
  const session = await WebGPU3DCanvasSession.create(mainCanvas, plot3d("WebGPU surface"));
  const scheduler = createInputScheduler(session);
  bindCanvas(mainCanvas, scheduler);
  resizeMainCanvas(mainCanvas, scheduler);
  window.addEventListener("resize", () => resizeMainCanvas(mainCanvas, scheduler));
  mainStatus.textContent = `${session.backend()} | readback ${session.readback_bytes()} | CPU frame upload ${session.cpu_frame_upload_bytes()}`;
  return { session, scheduler };
}

function setupWorker() {
  if (!workerCanvas.transferControlToOffscreen) {
    workerStatus.textContent = "OffscreenCanvas unavailable";
    return null;
  }
  const bounds = workerCanvas.getBoundingClientRect();
  const scale = window.devicePixelRatio || 1;
  const width = Math.max(1, Math.round(bounds.width * scale));
  const height = Math.max(1, Math.round(bounds.height * scale));
  workerCanvas.width = width;
  workerCanvas.height = height;

  const offscreen = workerCanvas.transferControlToOffscreen();
  const worker = new Worker(new URL("./3d-worker.js", import.meta.url), { type: "module" });
  const scheduler = createWorkerInputScheduler(worker);
  worker.postMessage({ type: "initialize", canvas: offscreen, width, height, scale }, [offscreen]);

  worker.addEventListener("message", ({ data }) => {
    if (data.type === "status") {
      workerStatus.textContent = data.message;
    }
  });
  worker.addEventListener("error", (event) => {
    workerStatus.textContent = event.message;
  });

  workerCanvas.addEventListener("pointerdown", (event) => {
    workerCanvas.setPointerCapture(event.pointerId);
    scheduler.pointerDown(...backingPoint(workerCanvas, event), event.button);
  });
  workerCanvas.addEventListener("pointermove", (event) => {
    scheduler.pointerMove(...backingPoint(workerCanvas, event));
  });
  workerCanvas.addEventListener("pointerup", (event) => {
    scheduler.pointerUp(...backingPoint(workerCanvas, event), event.button);
  });
  workerCanvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      scheduler.wheel(event.deltaY);
    },
    { passive: false },
  );
  return { worker, scheduler };
}

async function main() {
  if (!navigator.gpu) {
    throw new Error("WebGPU is unavailable");
  }
  const main3d = await setupMain();
  const worker = setupWorker();
  window.__ruviz3d = { main: main3d, worker };
}

main().catch((error) => {
  const message = describeError(error);
  mainStatus.textContent = message;
  workerStatus.textContent = message;
  console.error(error);
});
