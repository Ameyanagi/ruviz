import { beforeEach, expect, mock, test } from "bun:test";

const messages = [];
const mounted = [];
let failMount = false;
let options;
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
    options = config;
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
