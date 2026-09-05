import { line3d } from "ruviz/3d";

let session;
let initializing;
let target;
const ready = "Ready. Explore the helix with the view controls or drag to orbit.";
function status(type, message) {
  self.postMessage({ type, message });
}

async function initialize(data) {
  session?.dispose();
  session = undefined;
  const t = Array.from({ length: 300 }, (_, i) => i * 0.06);
  session = await line3d(
    t.map(Math.cos),
    t.map(Math.sin),
    t.map((v) => v * 0.08),
  )
    .equalScale()
    .stableScale()
    .title("Helical path")
    .mount(data.canvas, {
      scaleFactor: data.scale,
      onError(error) {
        status("error", `Rendering paused: ${error.message}`);
      },
    });
  status("ready", ready);
}

self.onmessage = async ({ data }) => {
  try {
    if (data.type === "initialize") {
      target = data;
      initializing = initialize(target);
      await initializing;
      return;
    }
    if (data.type === "retry") {
      await initializing?.catch(() => {});
      if (!session || session.needsRecreate()) {
        initializing = initialize(target);
        await initializing;
      } else {
        session.render();
        status("ready", ready);
      }
      return;
    }
    await initializing;
    if (!session) return;
    switch (data.type) {
      case "resize":
        target.scale = data.scale;
        session.resize(data.width, data.height, data.scale);
        break;
      case "pointerDown":
        session.pointerDown(data.x, data.y, data.button);
        break;
      case "pointerMove":
        session.pointerMove(data.x, data.y);
        break;
      case "pointerUp":
        session.pointerUp(data.x, data.y, data.button);
        break;
      case "wheel":
        session.wheel(data.delta);
        break;
      case "reset":
        session.resetView();
        break;
      default:
        throw new Error(`unknown 3d worker message: ${data.type}`);
    }
  } catch (error) {
    status("error", error instanceof Error ? error.message : String(error));
  }
};
