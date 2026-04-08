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
const DEFAULT_FALLBACK_WIDGET_WIDTH_PX = 640;
const DEFAULT_FALLBACK_WIDGET_HEIGHT_PX = 480;
const CONTEXT_MENU_EDGE_MARGIN_PX = 8;
const CONTEXT_MENU_MIN_WIDTH_PX = 150;
const MIN_WIDGET_HEIGHT_PX = 160;
const MIN_WIDGET_WIDTH_PX = 220;
const RESIZE_HANDLE_SIZE_PX = 16;
const CONTEXT_MENU_BACKGROUND =
  "var(--vscode-menu-background, var(--jp-layout-color1, rgba(248, 250, 252, 0.98)))";
const CONTEXT_MENU_FOREGROUND = "var(--vscode-menu-foreground, var(--jp-ui-font-color1, #111827))";
const CONTEXT_MENU_BORDER =
  "var(--vscode-widget-border, var(--jp-border-color2, rgba(15, 23, 42, 0.32)))";
const CONTEXT_MENU_HOVER_BACKGROUND =
  "var(--vscode-list-hoverBackground, var(--jp-layout-color2, rgba(15, 23, 42, 0.08)))";
const CONTEXT_MENU_FOCUS_RING = "var(--vscode-focusBorder, rgba(37, 99, 235, 0.85))";
const CONTEXT_MENU_SHADOW = "0 18px 40px rgba(15, 23, 42, 0.28), 0 6px 14px rgba(15, 23, 42, 0.18)";
const RESIZE_HANDLE_BACKGROUND =
  "var(--vscode-button-secondaryBackground, var(--jp-layout-color2, rgba(15, 23, 42, 0.18)))";
const RESIZE_HANDLE_BACKGROUND_ACTIVE =
  "var(--vscode-button-background, var(--jp-brand-color1, rgba(37, 99, 235, 0.92)))";
const RESIZE_HANDLE_BORDER =
  "var(--vscode-widget-border, var(--jp-border-color2, rgba(15, 23, 42, 0.32)))";

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

function resolveWidgetDisplaySize(snapshot: PlotSnapshot | undefined): {
  width: number;
  height: number;
} {
  const sizePx = snapshot?.sizePx;
  if (
    Array.isArray(sizePx) &&
    sizePx.length === 2 &&
    Number.isFinite(sizePx[0]) &&
    Number.isFinite(sizePx[1]) &&
    sizePx[0] > 0 &&
    sizePx[1] > 0
  ) {
    return {
      width: Math.round(sizePx[0]),
      height: Math.round(sizePx[1]),
    };
  }

  return {
    width: DEFAULT_FALLBACK_WIDGET_WIDTH_PX,
    height: DEFAULT_FALLBACK_WIDGET_HEIGHT_PX,
  };
}

function applyWidgetLayout(
  wrapper: HTMLDivElement,
  viewport: HTMLDivElement,
  canvas: HTMLCanvasElement,
  snapshot: PlotSnapshot | undefined,
  sizeOverridePx: { width: number; height: number } | null = null,
): { width: number; height: number; naturalWidth: number; naturalHeight: number } {
  const { width: naturalWidth, height: naturalHeight } = resolveWidgetDisplaySize(snapshot);
  const width =
    sizeOverridePx && Number.isFinite(sizeOverridePx.width)
      ? Math.max(MIN_WIDGET_WIDTH_PX, Math.round(sizeOverridePx.width))
      : naturalWidth;
  const height =
    sizeOverridePx && Number.isFinite(sizeOverridePx.height)
      ? Math.max(MIN_WIDGET_HEIGHT_PX, Math.round(sizeOverridePx.height))
      : (width * naturalHeight) / naturalWidth;

  wrapper.style.width = `${width}px`;
  wrapper.style.maxWidth = "100%";
  wrapper.style.boxSizing = "border-box";

  viewport.style.width = "100%";
  viewport.style.display = "block";
  viewport.style.boxSizing = "border-box";
  viewport.style.position = "relative";

  canvas.style.display = "block";
  canvas.style.boxSizing = "border-box";
  canvas.style.width = "100%";
  canvas.style.height = "100%";
  if (sizeOverridePx) {
    viewport.style.aspectRatio = "";
    viewport.style.height = `${height}px`;
  } else {
    viewport.style.aspectRatio = `${naturalWidth} / ${naturalHeight}`;
    viewport.style.height = "";
  }
  return { width, height, naturalWidth, naturalHeight };
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
  let displaySizeOverridePx: { width: number; height: number } | null = null;
  applyWidgetLayout(wrapper, viewport, canvas, getSnapshot(model), displaySizeOverridePx);

  const resizeHandle = document.createElement("div");
  resizeHandle.dataset.ruvizWidgetResizeHandle = "true";
  resizeHandle.style.position = "absolute";
  resizeHandle.style.right = "0.5rem";
  resizeHandle.style.bottom = "0.5rem";
  resizeHandle.style.width = `${RESIZE_HANDLE_SIZE_PX}px`;
  resizeHandle.style.height = `${RESIZE_HANDLE_SIZE_PX}px`;
  resizeHandle.style.border = `1px solid ${RESIZE_HANDLE_BORDER}`;
  resizeHandle.style.borderRadius = "0.35rem";
  resizeHandle.style.background = RESIZE_HANDLE_BACKGROUND;
  resizeHandle.style.boxShadow = "0 2px 8px rgba(15, 23, 42, 0.14)";
  resizeHandle.style.cursor = "nwse-resize";
  resizeHandle.style.zIndex = "11";
  resizeHandle.style.boxSizing = "border-box";
  resizeHandle.style.touchAction = "none";
  resizeHandle.style.userSelect = "none";

  const menu = document.createElement("div");
  menu.dataset.ruvizWidgetMenu = "true";
  menu.hidden = true;
  menu.style.position = "absolute";
  menu.style.display = "none";
  menu.style.zIndex = "10";
  menu.style.minWidth = `${CONTEXT_MENU_MIN_WIDTH_PX}px`;
  menu.style.padding = "0.25rem";
  menu.style.border = `1px solid ${CONTEXT_MENU_BORDER}`;
  menu.style.borderRadius = "0.5rem";
  menu.style.background = CONTEXT_MENU_BACKGROUND;
  menu.style.color = CONTEXT_MENU_FOREGROUND;
  menu.style.boxShadow = CONTEXT_MENU_SHADOW;
  menu.style.backdropFilter = "blur(10px)";
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
    item.style.font = "500 13px/1.35 system-ui, -apple-system, BlinkMacSystemFont, sans-serif";
    item.style.outline = "none";

    const setDefaultItemStyle = (): void => {
      item.style.background = "transparent";
      item.style.boxShadow = "none";
    };

    const setHoveredItemStyle = (): void => {
      item.style.background = CONTEXT_MENU_HOVER_BACKGROUND;
    };

    const setFocusedItemStyle = (): void => {
      item.style.background = CONTEXT_MENU_HOVER_BACKGROUND;
      item.style.boxShadow = `inset 0 0 0 1px ${CONTEXT_MENU_FOCUS_RING}`;
    };

    setDefaultItemStyle();

    item.addEventListener("pointerenter", () => {
      setHoveredItemStyle();
    });

    item.addEventListener("pointerleave", () => {
      if (document.activeElement === item) {
        setFocusedItemStyle();
        return;
      }
      setDefaultItemStyle();
    });

    item.addEventListener("focus", () => {
      setFocusedItemStyle();
    });

    item.addEventListener("blur", () => {
      setDefaultItemStyle();
    });

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

  wrapper.append(viewport, resizeHandle, menu);
  el.appendChild(wrapper);

  const applyCurrentLayout = (snapshot: PlotSnapshot | undefined): void => {
    applyWidgetLayout(wrapper, viewport, canvas, snapshot, displaySizeOverridePx);
  };

  let resizeRenderScheduled = false;
  const scheduleResizeRender = (): void => {
    if (resizeRenderScheduled) {
      return;
    }
    resizeRenderScheduled = true;
    requestAnimationFrame(async () => {
      resizeRenderScheduled = false;
      try {
        const session = await sessionPromise;
        if (!session.hasPlot()) {
          return;
        }
        session.resize();
        session.render();
      } catch (error) {
        console.error("Failed to resize the ruviz widget after a layout change.", error);
      }
    });
  };

  let isResizingWidget = false;
  let resizeStartClientX = 0;
  let resizeStartClientY = 0;
  let resizeStartWidth = 0;
  let resizeStartHeight = 0;
  let resizeStartAspectRatio = 1;

  const finishResize = (): void => {
    isResizingWidget = false;
    resizeHandle.style.background = RESIZE_HANDLE_BACKGROUND;
  };

  const onResizeHandleMouseDown = (event: MouseEvent): void => {
    event.preventDefault();
    event.stopPropagation();
    closeContextMenu();
    isResizingWidget = true;
    resizeStartClientX = event.clientX;
    resizeStartClientY = event.clientY;
    resizeStartWidth = wrapper.getBoundingClientRect().width;
    resizeStartHeight = viewport.getBoundingClientRect().height;
    resizeStartAspectRatio = resizeStartHeight > 0 ? resizeStartWidth / resizeStartHeight : 1;
    resizeHandle.style.background = RESIZE_HANDLE_BACKGROUND_ACTIVE;
  };

  const onResizeHandleMouseMove = (event: MouseEvent): void => {
    if (!isResizingWidget) {
      return;
    }

    event.preventDefault();
    const rawWidth = resizeStartWidth + event.clientX - resizeStartClientX;
    const rawHeight = resizeStartHeight + event.clientY - resizeStartClientY;
    let nextWidth = Math.max(MIN_WIDGET_WIDTH_PX, rawWidth);
    let nextHeight = Math.max(MIN_WIDGET_HEIGHT_PX, rawHeight);

    if (event.shiftKey || event.ctrlKey) {
      const widthDelta = nextWidth - resizeStartWidth;
      const heightDelta = nextHeight - resizeStartHeight;
      if (Math.abs(widthDelta) >= Math.abs(heightDelta * resizeStartAspectRatio)) {
        nextHeight = Math.max(MIN_WIDGET_HEIGHT_PX, nextWidth / resizeStartAspectRatio);
      } else {
        nextWidth = Math.max(MIN_WIDGET_WIDTH_PX, nextHeight * resizeStartAspectRatio);
      }
    }

    displaySizeOverridePx = { width: nextWidth, height: nextHeight };
    applyCurrentLayout(getSnapshot(model));
    scheduleResizeRender();
  };

  const onResizeHandleMouseUp = (event: MouseEvent): void => {
    if (!isResizingWidget) {
      return;
    }

    event.preventDefault();
    finishResize();
  };

  resizeHandle.addEventListener("mousedown", onResizeHandleMouseDown);
  document.addEventListener("mousemove", onResizeHandleMouseMove, true);
  document.addEventListener("mouseup", onResizeHandleMouseUp, true);

  const syncPlot = async (): Promise<void> => {
    try {
      closeContextMenu();
      const snapshot = getSnapshot(model);
      applyCurrentLayout(snapshot);
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
    finishResize();
    document.removeEventListener("pointerdown", onDocumentPointerDown, true);
    document.removeEventListener("mousemove", onResizeHandleMouseMove, true);
    document.removeEventListener("mouseup", onResizeHandleMouseUp, true);
    document.removeEventListener("keydown", onDocumentKeyDown);
    resizeHandle.removeEventListener("mousedown", onResizeHandleMouseDown);
    model.off("change:snapshot", onSnapshotChange);
    void sessionPromise.then((session) => session.dispose());
  };
}

export default { render };
