import { expect, test } from "@playwright/test";

async function waitForDemoReady(page) {
  await page.goto("/");
  await expect(page.locator("#capability-status")).not.toHaveText("");
  await expect(page.locator("#export-status")).toContainText("ready");
  await expect(page.locator("#main-status")).toContainText("ready");
  await expect(page.locator("#temporal-status")).toContainText("ready");
  await expect(page.locator("#observable-status")).toContainText("ready");
  await expect(page.locator("#worker-status")).toHaveText(/ready|fallback|unavailable/);
  await page.waitForFunction(() => Boolean(window.__ruvizDemo?.mainSession));
}

test("renders the expanded demo panels and runtime diagnostics", async ({ page }) => {
  await waitForDemoReady(page);

  await expect(page.locator("#capability-status")).toContainText("default font: ready");
  await expect(page.locator("#capability-status")).toContainText("worker canvas:");
  await expect(page.locator("#capability-status")).toContainText("WebGPU:");
  await expect(page.locator("#main-status")).toContainText("backend auto");
  await expect(page.locator("#temporal-status")).toContainText("t=0.00 s");
  await expect(page.locator("#observable-status")).toContainText("markers");
});

test("main-thread session controls and zoom work", async ({ page }) => {
  await waitForDemoReady(page);

  await page.locator("#main-backend").selectOption("gpu");
  await expect(page.locator("#main-status")).toContainText("backend gpu");

  await page.locator("#main-destroy").click();
  await expect(page.locator("#main-status")).toContainText("detached");
  await expect(page.locator("#main-status")).toContainText("has_plot false");

  await page.locator("#main-reattach").click();
  await expect(page.locator("#main-status")).toContainText("ready");
  await expect(page.locator("#main-status")).toContainText("has_plot true");

  const canvas = page.locator("#main-canvas");
  const box = await canvas.boundingBox();
  expect(box).not.toBeNull();

  const beforeZoom = await canvas.screenshot();
  await page.evaluate(() => {
    const canvas = document.getElementById("main-canvas");
    const session = window.__ruvizDemo?.mainSession;
    if (!canvas || !session) {
      throw new Error("main session test hook is unavailable");
    }

    const startX = canvas.width * 0.32;
    const startY = canvas.height * 0.24;
    const endX = canvas.width * 0.7;
    const endY = canvas.height * 0.48;

    session.pointerDown(startX, startY, 2);
    session.pointerMove(endX, endY);
    session.pointerUp(endX, endY, 2);
  });

  const afterZoom = await canvas.screenshot();
  expect(beforeZoom.equals(afterZoom)).toBeFalsy();
});

test("temporal playback updates the signal-driven canvas", async ({ page }) => {
  await waitForDemoReady(page);

  const temporalCanvas = page.locator("#temporal-canvas");
  const before = await temporalCanvas.screenshot();

  await page.locator("#temporal-time").evaluate((input, value) => {
    input.value = value;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }, "2.40");

  await expect(page.locator("#temporal-status")).toContainText("t=2.40 s");
  const after = await temporalCanvas.screenshot();
  expect(before.equals(after)).toBeFalsy();
});

test("png and svg exports work for the demo panels", async ({ page }) => {
  await waitForDemoReady(page);

  const mainDownload = page.waitForEvent("download");
  await page.locator("#main-export").click();
  expect((await mainDownload).suggestedFilename()).toBe("ruviz-main-thread.png");

  const mainSvgDownload = page.waitForEvent("download");
  await page.locator("#main-export-svg").click();
  expect((await mainSvgDownload).suggestedFilename()).toBe("ruviz-main-thread.svg");

  const workerStatus = (await page.locator("#worker-status").textContent()) || "";
  if (workerStatus.includes("ready")) {
    const workerDownload = page.waitForEvent("download");
    await page.locator("#worker-export").click();
    expect((await workerDownload).suggestedFilename()).toBe("ruviz-offscreen.png");

    const workerSvgDownload = page.waitForEvent("download");
    await page.locator("#worker-export-svg").click();
    expect((await workerSvgDownload).suggestedFilename()).toBe("ruviz-offscreen.svg");
  }

  const temporalDownload = page.waitForEvent("download");
  await page.locator("#temporal-export").click();
  expect((await temporalDownload).suggestedFilename()).toBe("ruviz-temporal.png");

  const observableDownload = page.waitForEvent("download");
  await page.locator("#observable-export").click();
  expect((await observableDownload).suggestedFilename()).toBe("ruviz-observable.png");

  const directPngDownload = page.waitForEvent("download");
  await page.locator("#export-direct-png").click();
  expect((await directPngDownload).suggestedFilename()).toBe("ruviz-direct-export.png");

  const directSvgDownload = page.waitForEvent("download");
  await page.locator("#export-direct-svg").click();
  expect((await directSvgDownload).suggestedFilename()).toBe("ruviz-direct-export.svg");
});
