import { createPlotFromSnapshot, createWorkerSession } from "./web/dist/index.js";

function triggerDownload(content, fileName, type) {
  const blob = new Blob([content], { type });
  const href = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = href;
  link.download = fileName;
  link.click();
  URL.revokeObjectURL(href);
}

/** @param {{ model: import("@jupyter-widgets/base").DOMWidgetModel, el: HTMLElement }} context */
function render({ model, el }) {
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

  const sessionPromise = createWorkerSession(canvas, { fallbackToMainThread: true });

  const syncPlot = async () => {
    const session = await sessionPromise;
    await session.setPlot(createPlotFromSnapshot(model.get("snapshot")));
    session.render();
  };

  void syncPlot();

  const onSnapshotChange = () => {
    void syncPlot();
  };

  model.on("change:snapshot", onSnapshotChange);

  pngButton.addEventListener("click", async () => {
    const session = await sessionPromise;
    const png = await session.exportPng();
    triggerDownload(png, "ruviz.png", "image/png");
  });

  svgButton.addEventListener("click", async () => {
    const session = await sessionPromise;
    const svg = await session.exportSvg();
    triggerDownload(svg, "ruviz.svg", "image/svg+xml");
  });

  return () => {
    model.off("change:snapshot", onSnapshotChange);
    void sessionPromise.then((session) => session.dispose());
  };
}

export default { render };
