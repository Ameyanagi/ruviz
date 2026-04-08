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
const DEFAULT_WIDGET_HEIGHT_PX = 420;
const CONTEXT_MENU_EDGE_MARGIN_PX = 8;
const CONTEXT_MENU_MIN_WIDTH_PX = 150;

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

function waitForLayoutPaint(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
}

function applyWidgetLayout(
  wrapper: HTMLDivElement,
  viewport: HTMLDivElement,
  canvas: HTMLCanvasElement,
  snapshot: PlotSnapshot | undefined,
): void {
  const sizePx = snapshot?.sizePx;
  wrapper.style.width = "100%";
  wrapper.style.boxSizing = "border-box";

  viewport.style.width = "100%";
  viewport.style.display = "block";
  viewport.style.boxSizing = "border-box";
  viewport.style.position = "relative";

  canvas.style.display = "block";
  canvas.style.boxSizing = "border-box";
  canvas.style.width = "100%";
  canvas.style.height = "100%";

  if (
    Array.isArray(sizePx) &&
    sizePx.length === 2 &&
    Number.isFinite(sizePx[0]) &&
    Number.isFinite(sizePx[1]) &&
    sizePx[0] > 0 &&
    sizePx[1] > 0
  ) {
    wrapper.style.maxWidth = `${sizePx[0]}px`;
    viewport.style.aspectRatio = `${sizePx[0]} / ${sizePx[1]}`;
    viewport.style.height = "";
    return;
  }

  wrapper.style.maxWidth = "";
  viewport.style.aspectRatio = "";
  viewport.style.height = `${DEFAULT_WIDGET_HEIGHT_PX}px`;
}

function render({ model, el }: RenderContext): () => void {
  ensureRawModuleConfigured();

  el.style.width = "100%";
  el.style.boxSizing = "border-box";

  const wrapper = document.createElement("div");
  wrapper.dataset.ruvizWidgetRoot = "true";
  wrapper.style.display = "block";
  wrapper.style.position = "relative";

  const viewport = document.createElement("div");
  viewport.dataset.ruvizWidgetViewport = "true";

  const canvas = document.createElement("canvas");
  canvas.style.border = "1px solid rgba(0, 0, 0, 0.12)";
  viewport.appendChild(canvas);
  applyWidgetLayout(wrapper, viewport, canvas, getSnapshot(model));

  const menu = document.createElement("div");
  menu.dataset.ruvizWidgetMenu = "true";
  menu.hidden = true;
  menu.style.position = "absolute";
  menu.style.display = "none";
  menu.style.zIndex = "10";
  menu.style.minWidth = `${CONTEXT_MENU_MIN_WIDTH_PX}px`;
  menu.style.padding = "0.25rem";
  menu.style.border = "1px solid rgba(0, 0, 0, 0.18)";
  menu.style.borderRadius = "0.5rem";
  menu.style.background = "rgba(255, 255, 255, 0.98)";
  menu.style.boxShadow = "0 8px 20px rgba(0, 0, 0, 0.16)";
  menu.style.boxSizing = "border-box";
  menu.style.gridAutoFlow = "row";
  menu.style.gap = "0.125rem";

  const buildMenuItem = (
    actionId: string,
    label: string,
    onActivate: () => Promise<void>,
  ): HTMLButtonElement => {
    const item = document.createElement("button");
    item.type = "button";
    item.dataset.ruvizWidgetMenuItem = actionId;
    item.textContent = label;
    item.style.display = "block";
    item.style.width = "100%";
    item.style.padding = "0.5rem 0.75rem";
    item.style.border = "none";
    item.style.borderRadius = "0.375rem";
    item.style.background = "transparent";
    item.style.color = "inherit";
    item.style.textAlign = "left";
    item.style.cursor = "pointer";

    item.addEventListener("click", () => {
      closeContextMenu();
      void onActivate();
    });

    return item;
  };

  const closeContextMenu = (): void => {
    menu.hidden = true;
    menu.style.display = "none";
  };

  const positionContextMenu = (clientX: number, clientY: number): void => {
    const rect = wrapper.getBoundingClientRect();
    const menuWidth = menu.offsetWidth || CONTEXT_MENU_MIN_WIDTH_PX;
    const menuHeight = menu.offsetHeight || 0;
    const proposedLeft = clientX - rect.left;
    const proposedTop = clientY - rect.top;
    const maxLeft = Math.max(
      CONTEXT_MENU_EDGE_MARGIN_PX,
      rect.width - menuWidth - CONTEXT_MENU_EDGE_MARGIN_PX,
    );
    const maxTop = Math.max(
      CONTEXT_MENU_EDGE_MARGIN_PX,
      rect.height - menuHeight - CONTEXT_MENU_EDGE_MARGIN_PX,
    );

    menu.style.left = `${Math.min(Math.max(CONTEXT_MENU_EDGE_MARGIN_PX, proposedLeft), maxLeft)}px`;
    menu.style.top = `${Math.min(Math.max(CONTEXT_MENU_EDGE_MARGIN_PX, proposedTop), maxTop)}px`;
  };

  const openContextMenu = (clientX: number, clientY: number): void => {
    menu.hidden = false;
    menu.style.display = "grid";
    positionContextMenu(clientX, clientY);
  };

  const sessionPromise = createNotebookCanvasSession(canvas, {
    onSecondaryClick: (clientX, clientY) => {
      openContextMenu(clientX, clientY);
    },
  });

  const triggerPngDownload = async (): Promise<void> => {
    try {
      const session = await sessionPromise;
      const png = await session.exportPng();
      triggerDownload(png, "ruviz.png", "image/png");
    } catch (error) {
      console.error("Failed to export PNG from the ruviz widget.", error);
    }
  };

  const triggerSvgDownload = async (): Promise<void> => {
    try {
      const session = await sessionPromise;
      const svg = await session.exportSvg();
      triggerDownload(svg, "ruviz.svg", "image/svg+xml");
    } catch (error) {
      console.error("Failed to export SVG from the ruviz widget.", error);
    }
  };

  menu.append(
    buildMenuItem("png", "Save PNG", triggerPngDownload),
    buildMenuItem("svg", "Save SVG", triggerSvgDownload),
  );

  wrapper.append(viewport, menu);
  el.appendChild(wrapper);

  const syncPlot = async (): Promise<void> => {
    try {
      closeContextMenu();
      const snapshot = getSnapshot(model);
      applyWidgetLayout(wrapper, viewport, canvas, snapshot);
      const session = await sessionPromise;
      await waitForLayoutPaint();
      session.resize();
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

  const onDocumentPointerDown = (event: PointerEvent): void => {
    if (menu.hidden) {
      return;
    }
    if (!(event.target instanceof Node) || !menu.contains(event.target)) {
      closeContextMenu();
    }
  };

  const onDocumentKeyDown = (event: KeyboardEvent): void => {
    if (event.key === "Escape") {
      closeContextMenu();
    }
  };

  document.addEventListener("pointerdown", onDocumentPointerDown, true);
  document.addEventListener("keydown", onDocumentKeyDown);

  return () => {
    closeContextMenu();
    document.removeEventListener("pointerdown", onDocumentPointerDown, true);
    document.removeEventListener("keydown", onDocumentKeyDown);
    model.off("change:snapshot", onSnapshotChange);
    void sessionPromise.then((session) => session.dispose());
  };
}

export default { render };
