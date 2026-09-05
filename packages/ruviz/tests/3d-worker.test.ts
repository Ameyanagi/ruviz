import { beforeEach, expect, mock, test } from "bun:test";

const messages = [];
const mounted = [];
let failMount = false;
let options;
let mountWait;
let mountCalls = 0;
const builder = {
  equalScale() {
    return this;
  },
  stableScale() {
    return this;
  },
  title() {
    return this;
  },
  async mount(canvas, config) {
    mountCalls += 1;
    options = config;
    await mountWait;
    if (failMount) {
      failMount = false;
      throw new Error("mount failed");
    }
    const session = {
      canvas,
      lost: false,
      disposed: false,
      renders: 0,
      dispose() {
        this.disposed = true;
      },
      needsRecreate() {
        return this.lost;
      },
      render() {
        this.renders += 1;
      },
      resize() {},
    };
    mounted.push(session);
    return session;
  },
};
mock.module("ruviz/3d", () => ({ line3d: () => builder }));
globalThis.self = { postMessage: (message) => messages.push(message) };
await import("../../../apps/web-demo/src/3d-worker.js");
const send = (data) => self.onmessage({ data });
beforeEach(() => {
  messages.length = 0;
  mounted.length = 0;
  failMount = false;
  mountWait = undefined;
  mountCalls = 0;
});

test("worker retry renders recoverable errors and remounts a lost session on the same canvas", async () => {
  const canvas = {};
  await send({ type: "initialize", canvas, scale: 2 });
  const original = mounted[0];
  options.onError(new Error("frame failed"));
  expect(messages.at(-1).type).toBe("error");
  await send({ type: "retry" });
  expect(original.renders).toBe(1);
  expect(mounted).toHaveLength(1);
  original.lost = true;
  await send({ type: "retry" });
  expect(original.disposed).toBe(true);
  expect(mounted).toHaveLength(2);
  expect(mounted[1].canvas).toBe(canvas);
  expect(options.scaleFactor).toBe(2);
  expect(messages.at(-1).type).toBe("ready");
});

test("worker can retry a failed initial mount", async () => {
  failMount = true;
  await send({ type: "initialize", canvas: {}, scale: 1 });
  expect(messages.at(-1).message).toBe("mount failed");
  await send({ type: "retry" });
  expect(mounted).toHaveLength(1);
  expect(messages.at(-1).type).toBe("ready");
});

test("concurrent retries create only one replacement session", async () => {
  await send({ type: "initialize", canvas: {}, scale: 1 });
  const original = mounted[0];
  original.lost = true;
  let release;
  mountWait = new Promise((resolve) => {
    release = resolve;
  });
  const first = send({ type: "retry" });
  const second = send({ type: "retry" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  options.onError(new Error("error while retrying"));
  const third = send({ type: "retry" });
  expect(mountCalls).toBe(2);
  release();
  await Promise.all([first, second, third]);
  expect(mountCalls).toBe(2);
  expect(mounted).toHaveLength(2);
  expect(original.disposed).toBe(true);
  expect(mounted[1].disposed).toBe(false);
  expect(mounted[1].renders).toBe(2);
});
