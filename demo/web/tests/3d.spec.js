import { expect, test } from "@playwright/test";

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

  await page.goto("/3d.html");
  await expect(page).toHaveTitle("ruviz 3d WebGPU demo");
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
