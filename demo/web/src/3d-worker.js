import initRaw, {
  JsPlot3D,
  OffscreenWebGPU3DCanvasSession,
  register_default_browser_fonts_js,
} from "ruviz/raw";

let session;
let frame = 0;
let latestMove;
let wheelDelta = 0;
let appliedMoves = 0;
let renderCalls = 0;
let presentedFrames = 0;

function plot3d() {
  const count = 300;
  const t = Float64Array.from({ length: count }, (_, index) => index * 0.06);
  const x = Float64Array.from(t, (value) => Math.cos(value));
  const y = Float64Array.from(t, (value) => Math.sin(value));
  const z = Float64Array.from(t, (value) => value * 0.08);
  const plot = new JsPlot3D();
  plot.line3d(x, y, z);
  plot.title("Worker WebGPU helix");
  return plot;
}

function requestWorkerFrame(callback) {
  if (typeof self.requestAnimationFrame === "function") {
    return self.requestAnimationFrame(callback);
  }
  return self.setTimeout(callback, 16);
}

function status() {
  self.postMessage({
    type: "status",
    message: `${session.backend()} | readback ${session.readback_bytes()} | moves ${appliedMoves} | renders ${renderCalls} | presents ${presentedFrames}`,
  });
}

function flush() {
  frame = 0;
  if (latestMove) {
    session.pointer_move(latestMove.x, latestMove.y);
    latestMove = undefined;
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
  status();
}

function schedule() {
  if (frame === 0) {
    frame = requestWorkerFrame(flush);
  }
}

self.onmessage = async ({ data }) => {
  try {
    switch (data.type) {
      case "initialize": {
        await initRaw();
        register_default_browser_fonts_js();
        session = await OffscreenWebGPU3DCanvasSession.create(data.canvas, plot3d());
        session.resize(data.width, data.height, data.scale);
        schedule();
        break;
      }
      case "pointerDown":
        session.pointer_down(data.x, data.y, data.button);
        break;
      case "pointerMove":
        latestMove = data;
        schedule();
        break;
      case "pointerUp":
        if (latestMove) {
          session.pointer_move(latestMove.x, latestMove.y);
          latestMove = undefined;
          appliedMoves += 1;
        }
        session.pointer_up(data.x, data.y, data.button);
        schedule();
        break;
      case "wheel":
        wheelDelta += data.delta;
        schedule();
        break;
      default:
        throw new Error(`unknown 3d worker message: ${data.type}`);
    }
  } catch (error) {
    self.postMessage({
      type: "status",
      message: error instanceof Error ? error.message : String(error),
    });
  }
};
