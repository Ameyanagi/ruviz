import initRaw, * as raw from "../generated/raw/ruviz_web_raw.js";
import { type BackendPreference, type PlotSnapshot } from "./shared.js";
import {
  buildRawPlotFromSnapshot,
  normalizeBackendPreference,
  toRawBackendPreference,
} from "./plot-runtime.js";

type RawModule = typeof import("../generated/raw/ruviz_web_raw.js");

interface WorkerEnvelope {
  type: string;
  requestId?: number;
  payload?: unknown;
  canvas?: OffscreenCanvas;
  width?: number;
  height?: number;
  scaleFactor?: number;
  backendPreference?: BackendPreference;
  initialTime?: number;
}

let rawModulePromise: Promise<RawModule> | null = null;
let session: raw.OffscreenCanvasSession | null = null;

async function ensureRawModule(): Promise<RawModule> {
  if (!rawModulePromise) {
    rawModulePromise = initRaw().then(() => {
      raw.register_default_browser_fonts_js();
      return raw;
    });
  }

  return rawModulePromise;
}

function postReady(): void {
  self.postMessage({ type: "ready" } satisfies WorkerEnvelope);
}

function postResponse(type: string, requestId: number | undefined, payload?: unknown): void {
  self.postMessage({ type, requestId, payload } satisfies WorkerEnvelope);
}

function postError(error: unknown, requestId?: number): void {
  const payload =
    error instanceof Error ? error.stack || error.message : String(error ?? "unknown worker error");
  postResponse("error", requestId, payload);
}

function getSession(): raw.OffscreenCanvasSession {
  if (!session) {
    throw new Error("worker session has not been initialized");
  }

  return session;
}

self.onmessage = async (event: MessageEvent<WorkerEnvelope>) => {
  const {
    backendPreference,
    canvas,
    height,
    initialTime,
    payload,
    requestId,
    scaleFactor,
    type,
    width,
  } = event.data;

  try {
    const module = await ensureRawModule();

    switch (type) {
      case "init": {
        if (!canvas) {
          throw new Error("worker init payload did not include an OffscreenCanvas");
        }

        session = new module.OffscreenCanvasSession(canvas);
        session.resize(width ?? 1, height ?? 1, scaleFactor ?? 1);
        session.set_backend_preference(
          toRawBackendPreference(module, normalizeBackendPreference(backendPreference)),
        );
        if (typeof initialTime === "number" && Number.isFinite(initialTime)) {
          session.set_time(initialTime);
        }
        postReady();
        return;
      }
      case "setPlot": {
        const currentSession = getSession();
        const snapshot = (payload as { snapshot: PlotSnapshot }).snapshot;
        currentSession.set_plot(buildRawPlotFromSnapshot(snapshot, module));
        postResponse("ack", requestId);
        return;
      }
      case "resize": {
        const currentSession = getSession();
        const metrics = payload as { width: number; height: number; scaleFactor: number };
        currentSession.resize(metrics.width, metrics.height, metrics.scaleFactor);
        postResponse("ack", requestId);
        return;
      }
      case "setTime": {
        const currentSession = getSession();
        currentSession.set_time((payload as { timeSeconds: number }).timeSeconds);
        postResponse("ack", requestId);
        return;
      }
      case "setBackendPreference": {
        const currentSession = getSession();
        currentSession.set_backend_preference(
          toRawBackendPreference(
            module,
            normalizeBackendPreference(
              (payload as { backendPreference: BackendPreference }).backendPreference,
            ),
          ),
        );
        postResponse("ack", requestId);
        return;
      }
      case "render": {
        getSession().render();
        postResponse("ack", requestId);
        return;
      }
      case "flush": {
        postResponse("ack", requestId);
        return;
      }
      case "resetView": {
        getSession().reset_view();
        postResponse("ack", requestId);
        return;
      }
      case "pointerDown": {
        const currentSession = getSession();
        const point = payload as { x: number; y: number; button: number };
        currentSession.pointer_down(point.x, point.y, point.button);
        return;
      }
      case "pointerMove": {
        const currentSession = getSession();
        const point = payload as { x: number; y: number };
        currentSession.pointer_move(point.x, point.y);
        return;
      }
      case "pointerUp": {
        const currentSession = getSession();
        const point = payload as { x: number; y: number; button: number };
        currentSession.pointer_up(point.x, point.y, point.button);
        return;
      }
      case "pointerLeave": {
        getSession().pointer_leave();
        return;
      }
      case "wheel": {
        const currentSession = getSession();
        const wheel = payload as { deltaY: number; x: number; y: number };
        currentSession.wheel(wheel.deltaY, wheel.x, wheel.y);
        return;
      }
      case "exportPng": {
        postResponse("exportPng", requestId, getSession().export_png());
        return;
      }
      case "exportSvg": {
        postResponse("exportSvg", requestId, getSession().export_svg());
        return;
      }
      case "destroy": {
        getSession().destroy();
        postResponse("ack", requestId);
        return;
      }
      default:
        throw new Error(`unsupported worker command: ${type}`);
    }
  } catch (error) {
    postError(error, requestId);
  }
};
