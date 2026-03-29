import init, {
  JsPlot,
  OffscreenCanvasSession,
} from "../pkg/ruviz_web.js";

let session = null;

function buildDemoPlot() {
  const plot = new JsPlot();
  const points = 200;
  const x = Array.from({ length: points }, (_, i) => (i / (points - 1)) * Math.PI * 8);
  const y = x.map((value) => Math.sin(value) * Math.cos(value * 0.3));

  plot.size_px(640, 360);
  plot.ticks(false);
  plot.theme_dark();
  plot.xlabel("x");
  plot.ylabel("y");
  plot.line(x, y);

  return plot;
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

function reportError(error) {
  postMessage({ type: "error", payload: describeError(error) });
}

self.onmessage = async (event) => {
  try {
    const { type, payload, canvas, devicePixelRatio, width, height } = event.data;

    if (type === "init") {
      await init();
      session = new OffscreenCanvasSession(canvas);
      session.resize(width || 640, height || 360, devicePixelRatio || 1);
      session.set_plot(buildDemoPlot());
      postMessage({ type: "ready" });
      return;
    }

    if (!session) {
      throw new Error("worker session has not been initialized");
    }

    switch (type) {
      case "pointer_down":
        session.pointer_down(payload.x, payload.y, payload.button);
        break;
      case "pointer_move":
        session.pointer_move(payload.x, payload.y);
        break;
      case "pointer_up":
        session.pointer_up(payload.x, payload.y, payload.button);
        break;
      case "pointer_leave":
        session.pointer_leave();
        break;
      case "wheel":
        session.wheel(payload.deltaY, payload.x, payload.y);
        break;
      case "resize":
        session.resize(payload.width, payload.height, payload.scaleFactor);
        break;
      case "reset":
        session.reset_view();
        break;
      case "export_png":
        postMessage({ type: "png", payload: session.export_png() });
        break;
      default:
        break;
    }
  } catch (error) {
    reportError(error);
  }
};
