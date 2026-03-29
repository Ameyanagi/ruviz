import { expect, test } from "@playwright/test";

async function waitForDemoReady(page) {
  await page.goto("/");
  await expect(page.locator("#capability-status")).not.toHaveText("");
  await expect(page.locator("#export-status")).toHaveText("ready");
  await expect(page.locator("#main-status")).toHaveText("ready");
  await expect(page.locator("#observable-status")).toContainText("ready");
  await expect(page.locator("#worker-status")).toHaveText(/ready|unavailable/);
}

async function readAveragePixel(page, selector, xRatio, yRatio, radius = 3) {
  return page.$eval(
    selector,
    (canvas, payload) => {
      const [nextXRatio, nextYRatio, nextRadius] = payload;
      const context = canvas.getContext("2d");
      const centerX = Math.round(canvas.width * nextXRatio);
      const centerY = Math.round(canvas.height * nextYRatio);
      const startX = Math.max(0, centerX - nextRadius);
      const startY = Math.max(0, centerY - nextRadius);
      const endX = Math.min(canvas.width - 1, centerX + nextRadius);
      const endY = Math.min(canvas.height - 1, centerY + nextRadius);

      let samples = 0;
      let red = 0;
      let green = 0;
      let blue = 0;
      let alpha = 0;

      for (let y = startY; y <= endY; y += 1) {
        for (let x = startX; x <= endX; x += 1) {
          const [r, g, b, a] = context.getImageData(x, y, 1, 1).data;
          red += r;
          green += g;
          blue += b;
          alpha += a;
          samples += 1;
        }
      }

      return [
        red / samples,
        green / samples,
        blue / samples,
        alpha / samples,
      ];
    },
    [xRatio, yRatio, radius],
  );
}

function colorDelta(before, after) {
  return (
    Math.abs(after[0] - before[0]) +
    Math.abs(after[1] - before[1]) +
    Math.abs(after[2] - before[2])
  );
}

test("renders the demo panels and runtime diagnostics", async ({ page }) => {
  await waitForDemoReady(page);

  await expect(page.locator("#capability-status")).toContainText("default font: ready");
  await expect(page.locator("#capability-status")).toContainText("worker canvas:");
  await expect(page.locator("#capability-status")).toContainText("WebGPU:");
});

test("main-thread hover feedback and box zoom stay aligned", async ({ page }) => {
  await waitForDemoReady(page);

  const canvas = page.locator("#main-canvas");
  const box = await canvas.boundingBox();
  expect(box).not.toBeNull();

  const baseline = await canvas.screenshot();
  let hoverChanged = false;

  for (const yRatio of [0.25, 0.35, 0.45, 0.55, 0.65]) {
    for (const xRatio of [0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85]) {
      await page.mouse.move(box.x + box.width * xRatio, box.y + box.height * yRatio);
      const current = await canvas.screenshot();
      if (!baseline.equals(current)) {
        hoverChanged = true;
        break;
      }
    }

    if (hoverChanged) {
      break;
    }
  }

  expect(hoverChanged).toBeTruthy();

  await page.mouse.move(box.x - 12, box.y - 12);
  const beforeZoom = await canvas.screenshot();
  const beforeBrush = await readAveragePixel(page, "#main-canvas", 0.51, 0.36, 4);
  const startX = box.x + box.width * 0.32;
  const startY = box.y + box.height * 0.24;
  const endX = box.x + box.width * 0.7;
  const endY = box.y + box.height * 0.48;

  await page.mouse.move(startX, startY);
  await page.mouse.down({ button: "right" });
  await page.mouse.move(endX, endY, { steps: 12 });

  const brushFill = await readAveragePixel(page, "#main-canvas", 0.51, 0.36, 4);
  expect(colorDelta(beforeBrush, brushFill)).toBeGreaterThan(35);
  expect(brushFill[2]).toBeGreaterThan(brushFill[0] - 5);
  expect(brushFill[3]).toBeGreaterThan(240);

  await page.mouse.up({ button: "right" });
  const afterZoom = await canvas.screenshot();
  expect(beforeZoom.equals(afterZoom)).toBeFalsy();
});

test("png export works for available demos", async ({ page }) => {
  await waitForDemoReady(page);

  const mainDownload = page.waitForEvent("download");
  await page.locator("#main-export").click();
  expect((await mainDownload).suggestedFilename()).toBe("ruviz-main-thread.png");

  const workerStatus = (await page.locator("#worker-status").textContent()) || "";
  if (workerStatus.includes("ready")) {
    const workerDownload = page.waitForEvent("download");
    await page.locator("#worker-export").click();
    expect((await workerDownload).suggestedFilename()).toBe("ruviz-offscreen.png");
  }

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
