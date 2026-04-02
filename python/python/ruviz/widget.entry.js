import {
  configureNotebookRawModuleSource,
  createNotebookCanvasSession,
} from "../../../packages/ruviz-web/src/notebook.ts";
import wasmBytes from "virtual:ruviz-wasm";

let rawModuleConfigured = false;

function ensureRawModuleConfigured() {
  if (rawModuleConfigured) {
    return;
  }

  configureNotebookRawModuleSource(wasmBytes);
  rawModuleConfigured = true;
}

function triggerDownload(content, fileName, type) {
  const blob = new Blob([content], { type });
  const href = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = href;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  setTimeout(() => URL.revokeObjectURL(href), 0);
}

/** @param {{ model: import("@jupyter-widgets/base").DOMWidgetModel, el: HTMLElement }} context */
function render({ model, el }) {
  ensureRawModuleConfigured();

  const wrapper = document.createElement("div");
  wrapper.style.display = "grid";
  wrapper.style.gap = "0.5rem";

  const toolbar = document.createElement("div");
  toolbar.style.display = "flex";
  toolbar.style.gap = "0.5rem";

  const pngButton = document.createElement("button");
  pngButton.textContent = "Download PNG";

  const svgButton = document.createElement("button");
  svgButton.textContent = "Download SVG";

  toolbar.append(pngButton, svgButton);

  const canvas = document.createElement("canvas");
  canvas.style.width = "100%";
  canvas.style.height = "420px";
  canvas.style.border = "1px solid rgba(0, 0, 0, 0.12)";

  wrapper.append(toolbar, canvas);
  el.appendChild(wrapper);

  const sessionPromise = createNotebookCanvasSession(canvas);

  const syncPlot = async () => {
    try {
      const session = await sessionPromise;
      await session.setPlot(model.get("snapshot"));
      session.render();
    } catch (error) {
      console.error("Failed to sync plot from ruviz widget snapshot.", error);
    }
  };

  void syncPlot();

  const onSnapshotChange = () => {
    void syncPlot();
  };

  model.on("change:snapshot", onSnapshotChange);

  pngButton.addEventListener("click", () => {
    void (async () => {
      try {
        const session = await sessionPromise;
        const png = await session.exportPng();
        triggerDownload(png, "ruviz.png", "image/png");
      } catch (error) {
        console.error("Failed to export PNG from the ruviz widget.", error);
      }
    })();
  });

  svgButton.addEventListener("click", () => {
    void (async () => {
      try {
        const session = await sessionPromise;
        const svg = await session.exportSvg();
        triggerDownload(svg, "ruviz.svg", "image/svg+xml");
      } catch (error) {
        console.error("Failed to export SVG from the ruviz widget.", error);
      }
    })();
  });

  return () => {
    model.off("change:snapshot", onSnapshotChange);
    void sessionPromise.then((session) => session.dispose());
  };
}

export default { render };
