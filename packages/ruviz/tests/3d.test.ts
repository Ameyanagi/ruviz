import { afterEach, beforeEach, expect, mock, test } from "bun:test";

let frames = new Map();
let nextFrame = 0;
let last;
let failResize = false;
const sessions = [];

class Canvas {
  width = 320;
  height = 200;
  style = { touchAction: "auto" };
  listeners = new Map();
  getBoundingClientRect() {
    return { left: 0, top: 0, width: 320, height: 200 };
  }
  addEventListener(type, handler) {
    this.listeners.set(type, handler);
  }
  removeEventListener(type) {
    this.listeners.delete(type);
  }
  setPointerCapture() {}
  hasPointerCapture() {
    return false;
  }
}
class RawPlot {
  series = [];
  scatter3d(...values) {
    this.series.push(["scatter", ...values]);
  }
  line3d(...values) {
    this.series.push(["line", ...values]);
  }
  surface(...values) {
    this.series.push(["surface", ...values]);
  }
  wireframe(...values) {
    this.series.push(["wireframe", ...values]);
  }
  axis_aspect(...ratio) {
    this.aspect = ratio;
  }
  equal_scale() {
    this.aspect = "data";
  }
  stable_scale(enabled) {
    this.stable = enabled;
  }
  title() {}
  free() {}
}
class RawSession {
  calls = [];
  failure = null;
  freed = false;
  destroyed = false;
  static async create(_canvas, plot) {
    last = new RawSession();
    last.series = plot.series;
    last.aspect = plot.aspect;
    last.stable = plot.stable;
    return last;
  }
  render() {
    this.calls.push(["render"]);
    if (this.failure) throw this.failure;
    return true;
  }
  resize() {
    if (failResize) throw new Error("invalid size");
  }
  pointer_down(...args) {
    this.dragging = true;
    this.calls.push(["down", ...args]);
  }
  pointer_move(...args) {
    this.calls.push(["move", ...args]);
    if (this.moveFailure) throw this.moveFailure;
  }
  pointer_up(...args) {
    this.dragging = false;
    this.calls.push(["up", ...args]);
  }
  wheel(...args) {
    this.calls.push(["wheel", ...args]);
  }
  reset_view() {
    this.calls.push(["reset"]);
  }
  export_png() {
    return new Uint8Array([137, 80, 78, 71]);
  }
  needs_recreate() {
    return false;
  }
  destroy() {
    this.destroyed = true;
  }
  free() {
    this.freed = true;
  }
}
mock.module("../generated/raw/ruviz_web_raw.js", () => ({
  default: async () => {},
  register_default_browser_fonts_js() {},
  JsPlot3D: RawPlot,
  WebGPU3DCanvasSession: RawSession,
  OffscreenWebGPU3DCanvasSession: RawSession,
}));
const { createPlot3d } = await import("../src/3d.ts");

beforeEach(() => {
  frames = new Map();
  failResize = false;
  Object.defineProperty(globalThis, "navigator", { configurable: true, value: { gpu: {} } });
  globalThis.HTMLCanvasElement = Canvas;
  globalThis.requestAnimationFrame = (callback) => {
    frames.set(++nextFrame, callback);
    return nextFrame;
  };
  globalThis.cancelAnimationFrame = (id) => frames.delete(id);
});
afterEach(() => {
  for (const session of sessions.splice(0)) session.dispose();
});
function flush() {
  const pending = [...frames.values()];
  frames.clear();
  for (const callback of pending) callback();
}
async function mount(options = {}) {
  const canvas = new Canvas();
  const session = await createPlot3d()
    .line3d([0, 1], [0, 1], [0, 0])
    .mount(canvas, { autoResize: false, bindInput: false, ...options });
  sessions.push(session);
  flush();
  last.calls = [];
  return { session, canvas };
}

test("scheduled failure is retained, reported once, and recoverable without remounting", async () => {
  const errors = [];
  const { session } = await mount({ onError: (error) => errors.push(error.message) });
  last.failure = "temporary render failure";
  session.render();
  session.render();
  expect(() => flush()).not.toThrow();
  expect(errors).toEqual(["temporary render failure"]);
  expect(session.error?.message).toBe("temporary render failure");
  expect(last.destroyed).toBe(false);
  expect(session.needsRecreate()).toBe(false);
  last.failure = null;
  session.render();
  flush();
  expect(session.error).toBeNull();
  expect(last.calls).toEqual([["render"], ["render"]]);
});

test("a burst coalesces moves and wheels and applies the final move before release", async () => {
  const { session } = await mount();
  session.pointerDown(10, 20, 0);
  for (let i = 0; i < 500; i++) session.pointerMove(i, i + 1);
  session.wheel(1);
  session.wheel(2);
  session.pointerUp(499, 500, 0);
  flush();
  expect(last.calls).toEqual([
    ["down", 10, 20, 0],
    ["move", 499, 500],
    ["wheel", 3],
    ["up", 499, 500, 0],
    ["render"],
  ]);
});

test("reset discards queued input and disposal cancels queued renders and event bindings", async () => {
  const { session, canvas } = await mount({ bindInput: true });
  expect(canvas.style.touchAction).toBe("none");
  session.pointerMove(50, 50);
  session.wheel(50);
  session.resetView();
  flush();
  expect(last.calls).toEqual([["reset"], ["render"]]);
  session.render();
  session.dispose();
  session.dispose();
  flush();
  expect(last.calls).toHaveLength(2);
  expect(last.freed).toBe(true);
  expect(canvas.listeners.size).toBe(0);
  expect(canvas.style.touchAction).toBe("auto");
  expect(() => session.render()).toThrow("destroyed");
});

test("mount failure releases the created raw session", async () => {
  failResize = true;
  await expect(
    createPlot3d().line3d([0, 1], [0, 1], [0, 0]).mount(new Canvas(), { autoResize: false }),
  ).rejects.toThrow("invalid size");
  expect(last.destroyed).toBe(true);
  expect(last.freed).toBe(true);
  expect(frames.size).toBe(0);
});

test("series append, failed additions are atomic, and replacement is explicit", async () => {
  const builder = createPlot3d()
    .surface(
      [0, 1],
      [0, 1],
      [
        [0, 1],
        [1, 0],
      ],
    )
    .line3d([0, 1], [0, 1], [0, 0]);
  expect(() => builder.wireframe([0, 1], [0, 1], [[1], [2]])).toThrow();
  sessions.push(await builder.mount(new Canvas(), { autoResize: false, bindInput: false }));
  expect(last.series.map(([kind]) => kind)).toEqual(["surface", "line"]);
  builder.clearSeries().scatter3d([0], [0], [0]);
  sessions.push(await builder.mount(new Canvas(), { autoResize: false, bindInput: false }));
  expect(last.series.map(([kind]) => kind)).toEqual(["scatter"]);
});

test("fixed ratios and stable scale survive replacement, and invalid ratios are atomic", async () => {
  const builder = createPlot3d().scatter3d([0], [0], [0]).axisAspect(1, 2, 3).stableScale();
  for (const value of [0, -1, NaN, Infinity, 1e-100, 1e100]) {
    expect(() => builder.axisAspect(value, 1, 1)).toThrow("axisAspect");
  }
  builder.clearSeries().line3d([0, 1], [0, 1], [0, 0]);
  sessions.push(await builder.mount(new Canvas(), { autoResize: false, bindInput: false }));
  expect(last.aspect).toEqual([1, 2, 3]);
  expect(last.stable).toBe(true);
  sessions.push(
    await builder
      .equalScale()
      .stableScale(false)
      .mount(new Canvas(), { autoResize: false, bindInput: false }),
  );
  expect(last.aspect).toBe("data");
  expect(last.stable).toBe(false);
});

test("a failed final move still releases the retained drag and reports the failure", async () => {
  const errors = [];
  const { session, canvas } = await mount({
    bindInput: true,
    onError: (error) => errors.push(error),
  });
  const event = { clientX: 10, clientY: 20, pointerId: 1, button: 0 };
  canvas.listeners.get("pointerdown")(event);
  canvas.listeners.get("pointermove")({ ...event, clientX: 30 });
  last.moveFailure = new Error("move failed");
  canvas.listeners.get("pointerup")({ ...event, clientX: 30 });
  expect(last.dragging).toBe(false);
  expect(last.calls.at(-1)).toEqual(["up", 30, 20, 0]);
  expect(session.error?.message).toBe("move failed");
  expect(errors).toHaveLength(1);
  last.moveFailure = null;
  flush();
  expect(session.error).toBeNull();
});
