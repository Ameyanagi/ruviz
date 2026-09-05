import { expect, test } from "@playwright/test";
import { openDemo } from "./navigate.js";

const DEMO_READY_TIMEOUT_MS = 60_000;

test("direct 3d WebGPU renders and coalesces main and worker input", async ({
  browserName,
  page,
}) => {
  test.skip(browserName !== "chromium", "the required WebGPU lane runs in Chromium");

  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(String(error)));
  await page.addInitScript(() => {
    const original = CanvasRenderingContext2D.prototype.putImageData;
    window.__ruvizPutImageDataCalls = 0;
    CanvasRenderingContext2D.prototype.putImageData = function (...args) {
      window.__ruvizPutImageDataCalls += 1;
      return original.apply(this, args);
    };
  });

  await openDemo(page, "/3d-benchmark.html", "ruviz 3d WebGPU benchmark");
  await expect(page.locator("#main-3d-status")).toContainText("gpu3d-surface", {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
  await expect(page.locator("#main-3d-status")).toContainText("readback 0");
  await expect(page.locator("#main-3d-status")).toContainText("CPU frame upload 0");
  await expect(page.locator("#worker-3d-status")).toContainText("gpu3d-surface", {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
  await expect(page.locator("#main-3d-reset")).toBeVisible();
  await expect(page.locator("#worker-3d-reset")).toBeVisible();

  const canvas = page.locator("#main-3d");
  const result = await page.evaluate(async () => {
    const hook = window.__ruviz3d;
    if (!hook?.main || !hook.worker) {
      throw new Error("the 3d smoke hooks are unavailable");
    }
    const { main, worker } = hook;
    const hash = (bytes) => {
      let value = 2166136261;
      for (const byte of bytes) {
        value ^= byte;
        value = Math.imul(value, 16777619);
      }
      return value >>> 0;
    };
    const beforePngHash = hash(main.session.export_png());
    main.scheduler.wheel(-10_000);
    await new Promise((resolve) => requestAnimationFrame(resolve));
    await new Promise((resolve) => requestAnimationFrame(resolve));
    const boundedWheelPngHash = hash(main.session.export_png());
    document.querySelector("#main-3d-reset").click();
    await new Promise((resolve) => requestAnimationFrame(resolve));
    await new Promise((resolve) => requestAnimationFrame(resolve));
    const resetPngHash = hash(main.session.export_png());
    const beforeSurfacePresents = Number(main.session.surface_presents());
    const start = main.scheduler.metrics();
    main.scheduler.pointerDown(180, 180, 0);
    for (let index = 0; index < 500; index += 1) {
      main.scheduler.pointerMove(180 + index * 0.35, 180 + index * 0.08);
    }
    await new Promise((resolve) => requestAnimationFrame(resolve));
    await new Promise((resolve) => requestAnimationFrame(resolve));
    main.scheduler.pointerUp(355, 220, 0);
    await new Promise((resolve) => requestAnimationFrame(resolve));
    await new Promise((resolve) => requestAnimationFrame(resolve));
    const end = main.scheduler.metrics();
    const png = main.session.export_png();

    const workerStart = worker.scheduler.metrics();
    worker.scheduler.pointerDown(180, 180, 0);
    for (let index = 0; index < 500; index += 1) {
      worker.scheduler.pointerMove(180 + index * 0.35, 180 + index * 0.08);
    }
    await new Promise((resolve) => requestAnimationFrame(resolve));
    await new Promise((resolve) => requestAnimationFrame(resolve));
    worker.scheduler.pointerUp(355, 220, 0);
    await new Promise((resolve) => requestAnimationFrame(resolve));
    await new Promise((resolve) => requestAnimationFrame(resolve));
    const workerEnd = worker.scheduler.metrics();

    return {
      appliedMoves: end.appliedMoves - start.appliedMoves,
      renderCalls: end.renderCalls - start.renderCalls,
      presentedFrames: end.presentedFrames - start.presentedFrames,
      surfacePresents: Number(main.session.surface_presents()) - beforeSurfacePresents,
      workerSentMoves: workerEnd.sentMoves - workerStart.sentMoves,
      readbackBytes: Number(main.session.readback_bytes()),
      cpuFrameUploadBytes: Number(main.session.cpu_frame_upload_bytes()),
      needsRecreate: main.session.needs_recreate(),
      beforePngHash,
      boundedWheelPngHash,
      resetPngHash,
      afterPngHash: hash(png),
      pngSignature: Array.from(png.slice(0, 8)),
      putImageDataCalls: window.__ruvizPutImageDataCalls,
    };
  });
  await expect(page.locator("#worker-3d-status")).toContainText("moves 1");
  await canvas.screenshot();
  await page.evaluate(() => {
    const hook = window.__ruviz3d;
    hook.main.session.destroy();
    hook.main.session.free();
    hook.worker.worker.terminate();
  });

  expect(result.appliedMoves).toBeLessThanOrEqual(2);
  expect(result.renderCalls).toBeLessThanOrEqual(2);
  expect(result.presentedFrames).toBeGreaterThanOrEqual(1);
  expect(result.surfacePresents).toBe(1);
  expect(result.workerSentMoves).toBeLessThanOrEqual(2);
  expect(result.readbackBytes).toBe(0);
  expect(result.cpuFrameUploadBytes).toBe(0);
  expect(result.needsRecreate).toBeFalsy();
  expect(result.putImageDataCalls).toBe(0);
  expect(result.boundedWheelPngHash).not.toBe(result.beforePngHash);
  expect(result.resetPngHash).toBe(result.beforePngHash);
  expect(result.beforePngHash).not.toBe(result.afterPngHash);
  expect(result.pngSignature).toEqual([137, 80, 78, 71, 13, 10, 26, 10]);
  expect(pageErrors).toEqual([]);
});

test("public SDK demo exposes keyboard controls and preserves composed series", async ({
  browserName,
  page,
}) => {
  test.skip(browserName !== "chromium", "WebGPU is required");
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await openDemo(page, "/3d.html", "ruviz 3d WebGPU demo");
  await expect(page.locator("#main-3d-status")).toContainText("Ready", {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
  await expect(page.locator("#worker-3d-status")).toContainText("Ready", {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
  await expect(page.locator("#main-3d")).toHaveAccessibleName("Explore a surface");
  await expect(page.locator("#worker-3d")).toHaveAccessibleName("Follow a path");
  await page.evaluate(() => {
    window.__ruviz3d.worker.dispatchEvent(
      new MessageEvent("message", {
        data: { type: "error", message: "Simulated recoverable frame failure" },
      }),
    );
  });
  await expect(page.getByRole("button", { name: "Retry path rendering" })).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Reset follow a path", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Retry path rendering" }).click();
  await expect(page.locator("#worker-3d-status")).toContainText("Ready");
  await expect(page.getByRole("button", { name: "Retry path rendering" })).toBeHidden();
  await expect(
    page.getByRole("button", { name: "Reset follow a path", exact: true }),
  ).toBeEnabled();
  const bytes = () =>
    page.evaluate(async () => Array.from(await window.__ruviz3d.main.exportPng()));
  const before = await bytes();
  await page.getByRole("button", { name: "Rotate left: explore a surface", exact: true }).focus();
  await page.keyboard.press("Enter");
  expect(await bytes()).not.toEqual(before);
  await page.getByRole("button", { name: "Reset explore a surface", exact: true }).click();
  expect(await bytes()).toEqual(before);

  const composition = await page.evaluate(async () => {
    const { createPlot3d } = window.__ruviz3d.sdk;
    const makeCanvas = () => {
      const canvas = document.createElement("canvas");
      canvas.style.cssText = "width:320px;height:200px";
      document.body.append(canvas);
      return canvas;
    };
    const builder = createPlot3d()
      .surface(
        [-1, 1],
        [-1, 1],
        [
          [0, 1],
          [1, 0],
        ],
      )
      .line3d([-2, 2], [0, 0], [2, 2]);
    const firstCanvas = makeCanvas();
    const first = await builder.mount(firstCanvas, { autoResize: false, bindInput: false });
    const composed = Array.from(await first.exportPng());
    builder.clearSeries().line3d([-2, 2], [0, 0], [2, 2]);
    const secondCanvas = makeCanvas();
    const second = await builder.mount(secondCanvas, { autoResize: false, bindInput: false });
    const replaced = Array.from(await second.exportPng());
    const retained = Array.from(await first.exportPng());
    first.dispose();
    second.dispose();
    firstCanvas.remove();
    secondCanvas.remove();
    return { composed, replaced, retained };
  });
  expect(composition.composed).not.toEqual(composition.replaced);
  expect(composition.retained).toEqual(composition.composed);
  expect(errors).toEqual([]);
});
