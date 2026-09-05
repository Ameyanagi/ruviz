import * as sdk from "ruviz/3d";

const mainCanvas = document.getElementById("main-3d");
const workerCanvas = document.getElementById("worker-3d");
const mainStatus = document.getElementById("main-3d-status");
const workerStatus = document.getElementById("worker-3d-status");

function surfacePlot() {
  const size = 48;
  const axis = Float64Array.from({ length: size }, (_, i) => -3 + (i / (size - 1)) * 6);
  const z = Array.from(axis, (y) =>
    Array.from(axis, (x) => {
      const radius = Math.hypot(x, y);
      return Math.cos(radius * 2) * Math.exp(-radius * 0.28);
    }),
  );
  return sdk.surface(axis, axis, z).axisAspect(1, 1, 0.5).stableScale().title("Damped radial wave");
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function bindControls(prefix, session) {
  const root = document.getElementById(`${prefix}-controls`);
  root.addEventListener("click", (event) => {
    const action = event.target.closest("button")?.dataset.action;
    if (action === "reset") session.resetView();
    if (action === "in") session.wheel(-120);
    if (action === "out") session.wheel(120);
    if (action === "left" || action === "right") {
      session.pointerDown(0, 0, 0);
      session.pointerMove(action === "left" ? -60 : 60, 0);
      session.pointerUp(action === "left" ? -60 : 60, 0, 0);
    }
  });
}

async function setupMain() {
  const retry = document.getElementById("main-3d-retry");
  let session;
  async function mount() {
    session?.dispose();
    mainStatus.textContent = "Loading surface…";
    session = await surfacePlot().mount(mainCanvas, {
      onError(error) {
        mainStatus.textContent = `Rendering paused: ${error.message}`;
        retry.hidden = false;
      },
    });
    mainStatus.textContent = "Ready. Drag to orbit, or use the view controls.";
    document.getElementById("main-3d-diagnostics").textContent = session.backend();
    retry.hidden = true;
    window.__ruviz3d.main = session;
  }
  await mount();
  // Forward to the current session after a device-loss remount.
  bindControls("main-3d", {
    resetView: () => session.resetView(),
    wheel: (delta) => session.wheel(delta),
    pointerDown: (...args) => session.pointerDown(...args),
    pointerMove: (...args) => session.pointerMove(...args),
    pointerUp: (...args) => session.pointerUp(...args),
  });
  retry.addEventListener("click", async () => {
    try {
      if (session.needsRecreate()) await mount();
      else {
        session.render();
        mainStatus.textContent = "Retry requested. Use the view controls to continue.";
        retry.hidden = true;
      }
    } catch (error) {
      mainStatus.textContent = errorMessage(error);
      retry.hidden = false;
    }
  });
  window.addEventListener("pagehide", () => session.dispose(), { once: true });
}

function setupWorker() {
  if (!workerCanvas.transferControlToOffscreen) {
    workerStatus.textContent =
      "Worker rendering is unavailable in this browser. Explore the surface above.";
    return;
  }
  const worker = new Worker(new URL("./3d-worker.js", import.meta.url), { type: "module" });
  const canvas = workerCanvas.transferControlToOffscreen();
  const send = (type, payload = {}) => worker.postMessage({ type, ...payload });
  const resize = () => {
    const bounds = workerCanvas.getBoundingClientRect();
    const scale = window.devicePixelRatio || 1;
    send("resize", {
      width: Math.max(1, Math.round(bounds.width * scale)),
      height: Math.max(1, Math.round(bounds.height * scale)),
      scale,
    });
  };
  worker.postMessage({ type: "initialize", canvas, scale: window.devicePixelRatio || 1 }, [canvas]);
  worker.addEventListener("message", ({ data }) => {
    if (data.type === "ready") resize();
    workerStatus.textContent = data.message;
  });
  worker.addEventListener("error", (event) => {
    workerStatus.textContent = event.message;
  });
  const controls = {
    resetView: () => send("reset"),
    wheel: (delta) => send("wheel", { delta }),
    pointerDown: (x, y, button) => send("pointerDown", { x, y, button }),
    pointerMove: (x, y) => send("pointerMove", { x, y }),
    pointerUp: (x, y, button) => send("pointerUp", { x, y, button }),
  };
  bindControls("worker-3d", controls);
  let activeButton = null;
  const point = (event) => {
    const bounds = workerCanvas.getBoundingClientRect();
    const scale = window.devicePixelRatio || 1;
    return [(event.clientX - bounds.left) * scale, (event.clientY - bounds.top) * scale];
  };
  workerCanvas.addEventListener("contextmenu", (event) => event.preventDefault());
  workerCanvas.addEventListener("pointerdown", (event) => {
    activeButton = event.button;
    workerCanvas.setPointerCapture(event.pointerId);
    controls.pointerDown(...point(event), event.button);
  });
  workerCanvas.addEventListener("pointermove", (event) => controls.pointerMove(...point(event)));
  const release = (event) => {
    controls.pointerUp(...point(event), activeButton ?? event.button);
    activeButton = null;
    if (workerCanvas.hasPointerCapture(event.pointerId))
      workerCanvas.releasePointerCapture(event.pointerId);
  };
  workerCanvas.addEventListener("pointerup", release);
  workerCanvas.addEventListener("pointercancel", release);
  workerCanvas.addEventListener("dblclick", controls.resetView);
  workerCanvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      controls.wheel(event.deltaY);
    },
    { passive: false },
  );
  const observer = new ResizeObserver(resize);
  observer.observe(workerCanvas);
  window.addEventListener("resize", resize);
  window.addEventListener(
    "pagehide",
    () => {
      observer.disconnect();
      window.removeEventListener("resize", resize);
      worker.terminate();
    },
    { once: true },
  );
  window.__ruviz3d.worker = worker;
}

window.__ruviz3d = { sdk };
setupMain().catch((error) => {
  mainStatus.textContent = errorMessage(error);
});
setupWorker();
