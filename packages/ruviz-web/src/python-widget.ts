import { configureNotebookRawModuleSource, createNotebookCanvasSession } from "./notebook.js";
import type { PlotSnapshot } from "./shared.js";
import wasmBytes from "virtual:ruviz-wasm";

interface WidgetModel {
  get(name: string): unknown;
  on(name: string, callback: () => void): void;
  off(name: string, callback: () => void): void;
}

interface RenderContext {
  model: WidgetModel;
  el: HTMLElement;
}

let rawModuleConfigured = false;

function cloneBytes(bytes: Uint8Array<ArrayBufferLike>): Uint8Array<ArrayBuffer> {
  return Uint8Array.from(bytes);
}

function ensureRawModuleConfigured(): void {
  if (rawModuleConfigured) {
    return;
  }

  configureNotebookRawModuleSource(cloneBytes(wasmBytes));
  rawModuleConfigured = true;
}

function triggerDownload(
  content: BlobPart | Uint8Array<ArrayBufferLike>,
  fileName: string,
  type: string,
): void {
  const payload = content instanceof Uint8Array ? cloneBytes(content) : content;
  const blob = new Blob([payload], { type });
  const href = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = href;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  setTimeout(() => URL.revokeObjectURL(href), 0);
}

function getSnapshot(model: WidgetModel): PlotSnapshot {
  return model.get("snapshot") as PlotSnapshot;
}

function applyCanvasSizing(canvas: HTMLCanvasElement, snapshot: PlotSnapshot | undefined): void {
  const sizePx = snapshot?.sizePx;
  canvas.style.display = "block";
  canvas.style.boxSizing = "border-box";
  canvas.style.width = "100%";

  if (
    Array.isArray(sizePx) &&
    sizePx.length === 2 &&
    Number.isFinite(sizePx[0]) &&
    Number.isFinite(sizePx[1]) &&
    sizePx[0] > 0 &&
    sizePx[1] > 0
  ) {
    canvas.style.aspectRatio = `${sizePx[0]} / ${sizePx[1]}`;
    canvas.style.height = "auto";
    return;
  }

  canvas.style.aspectRatio = "";
  canvas.style.height = "420px";
}

function render({ model, el }: RenderContext): () => void {
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
  canvas.style.border = "1px solid rgba(0, 0, 0, 0.12)";
  applyCanvasSizing(canvas, getSnapshot(model));

  wrapper.append(toolbar, canvas);
  el.appendChild(wrapper);

  const sessionPromise = createNotebookCanvasSession(canvas);

  const syncPlot = async (): Promise<void> => {
    try {
      const snapshot = getSnapshot(model);
      applyCanvasSizing(canvas, snapshot);
      const session = await sessionPromise;
      await session.setPlot(snapshot);
      session.render();
    } catch (error) {
      console.error("Failed to sync plot from ruviz widget snapshot.", error);
    }
  };

  void syncPlot();

  const onSnapshotChange = (): void => {
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
