import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";

const PYTHON_WIDGET_BUNDLE = readFileSync(
  fileURLToPath(new URL("../../../python/ruviz_py/ruviz/widget.js", import.meta.url)),
  "utf8",
);

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

test("canvas initialTime is preserved before the first plot attachment", async ({ page }) => {
  await waitForDemoReady(page);

  const changed = await page.evaluate(async () => {
    const demo = window.__ruvizDemo;
    if (!demo?.sdk) {
      throw new Error("SDK test hooks are unavailable");
    }

    const { createCanvasSession, createPlot, createSineSignal } = demo.sdk;
    const x = Array.from({ length: 96 }, (_, index) => (index / 95) * Math.PI * 4);

    const createCanvas = () => {
      const canvas = document.createElement("canvas");
      canvas.style.cssText = [
        "position: fixed",
        "left: -10000px",
        "top: -10000px",
        "width: 320px",
        "height: 180px",
      ].join(";");
      document.body.appendChild(canvas);
      return canvas;
    };

    const buildPlot = () =>
      createPlot()
        .setSizePx(320, 180)
        .setTheme("dark")
        .setTitle("initial-time regression")
        .addLine({
          x,
          y: createSineSignal({
            points: x.length,
            domain: [x[0], x[x.length - 1]],
            amplitude: 1,
            cycles: 2,
            phaseVelocity: 1.25,
          }),
        });

    const renderWithTime = async (initialTime) => {
      const canvas = createCanvas();
      const session = await createCanvasSession(canvas, {
        autoResize: false,
        bindInput: false,
        initialTime,
      });

      await session.setPlot(buildPlot());
      const png = await session.exportPng();
      session.dispose();
      canvas.remove();
      return png;
    };

    const [atZero, atOffset] = await Promise.all([renderWithTime(0), renderWithTime(1.5)]);
    if (atZero.length !== atOffset.length) {
      return true;
    }

    for (let index = 0; index < atZero.length; index += 1) {
      if (atZero[index] !== atOffset[index]) {
        return true;
      }
    }

    return false;
  });

  expect(changed).toBeTruthy();
});

test("worker destroy does not leave hasPlot stale after an in-flight setPlot", async ({ page }) => {
  await waitForDemoReady(page);

  const workerStatus = (await page.locator("#worker-status").textContent()) || "";
  test.skip(!workerStatus.includes("ready"), "worker mode is unavailable in this browser");

  const result = await page.evaluate(async () => {
    const demo = window.__ruvizDemo;
    if (!demo?.workerSession || !demo?.directExportPlot) {
      throw new Error("worker session test hook is unavailable");
    }

    const pending = demo.workerSession.setPlot(demo.directExportPlot.clone());
    demo.workerSession.destroy();
    await pending;

    const hasPlotAfterRace = demo.workerSession.hasPlot();
    await demo.workerSession.setPlot(demo.directExportPlot.clone());

    return {
      hasPlotAfterRace,
      hasPlotAfterReattach: demo.workerSession.hasPlot(),
    };
  });

  expect(result.hasPlotAfterRace).toBeFalsy();
  expect(result.hasPlotAfterReattach).toBeTruthy();
});

test("python widget bundle renders when loaded from a blob-backed module", async ({ page }) => {
  await waitForDemoReady(page);

  const consoleErrors = [];
  const pageErrors = [];
  page.on("console", (message) => {
    if (message.type() === "error") {
      consoleErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    pageErrors.push(String(error));
  });

  const result = await page.evaluate(async (widgetSource) => {
    const demo = window.__ruvizDemo;
    if (!demo?.sdk) {
      throw new Error("SDK test hooks are unavailable");
    }

    const buildSnapshot = (title, values, sizePx = [640, 360]) =>
      demo.sdk
        .createPlot()
        .setSizePx(sizePx[0], sizePx[1])
        .setTitle(title)
        .setXLabel("x")
        .setYLabel("y")
        .addLine({
          x: values.map((_, index) => index),
          y: values,
        })
        .toSnapshot();

    const listeners = new Map();
    const model = {
      snapshot: buildSnapshot("widget initial", [0.2, 0.9, 0.3, 1.1]),
      get(name) {
        return name === "snapshot" ? this.snapshot : undefined;
      },
      on(name, callback) {
        const callbacks = listeners.get(name) ?? [];
        callbacks.push(callback);
        listeners.set(name, callbacks);
      },
      off(name, callback) {
        const callbacks = listeners.get(name);
        if (!callbacks) {
          return;
        }
        listeners.set(
          name,
          callbacks.filter((registered) => registered !== callback),
        );
      },
      setSnapshot(snapshot) {
        this.snapshot = snapshot;
        for (const callback of listeners.get("change:snapshot") ?? []) {
          callback();
        }
      },
    };

    const waitForNextPaint = () =>
      new Promise((resolve) => {
        requestAnimationFrame(() => {
          requestAnimationFrame(resolve);
        });
      });

    const waitForCanvasChange = async (canvas, previousData) => {
      for (let attempt = 0; attempt < 120; attempt += 1) {
        await waitForNextPaint();
        const currentData = canvas.toDataURL("image/png");

        if (previousData === undefined) {
          const blankCanvas = document.createElement("canvas");
          blankCanvas.width = canvas.width;
          blankCanvas.height = canvas.height;
          if (currentData !== blankCanvas.toDataURL("image/png")) {
            return currentData;
          }
          continue;
        }

        if (currentData !== previousData) {
          return currentData;
        }
      }

      throw new Error("widget canvas did not update");
    };

    const waitForCanvasBox = async (canvas, previousHeight) => {
      for (let attempt = 0; attempt < 120; attempt += 1) {
        await waitForNextPaint();
        const rect = canvas.getBoundingClientRect();
        if (rect.width <= 0 || rect.height <= 0) {
          continue;
        }
        if (previousHeight === undefined || Math.abs(rect.height - previousHeight) > 0.5) {
          return { width: rect.width, height: rect.height };
        }
      }

      throw new Error("widget canvas size did not update");
    };

    const mount = document.createElement("div");
    mount.id = "python-widget-test-host";
    mount.style.width = "600px";
    document.body.appendChild(mount);

    const moduleUrl = URL.createObjectURL(new Blob([widgetSource], { type: "text/javascript" }));

    try {
      const mod = await import(moduleUrl);
      const cleanup = mod.default.render({ model, el: mount });
      const canvas = mount.querySelector("canvas");
      if (!(canvas instanceof HTMLCanvasElement)) {
        throw new Error("widget bundle did not create a canvas");
      }

      const initialImage = await waitForCanvasChange(canvas);
      const initialBox = await waitForCanvasBox(canvas);
      model.setSnapshot(buildSnapshot("widget updated", [1.1, 0.4, 1.0, 0.2], [360, 360]));
      const updatedImage = await waitForCanvasChange(canvas, initialImage);
      const updatedBox = await waitForCanvasBox(canvas, initialBox.height);

      if (typeof cleanup === "function") {
        cleanup();
      }

      mount.remove();
      return { initialBox, initialImage, updatedBox, updatedImage };
    } finally {
      URL.revokeObjectURL(moduleUrl);
    }
  }, PYTHON_WIDGET_BUNDLE);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
  expect(result.initialBox.width / result.initialBox.height).toBeCloseTo(640 / 360, 1);
  expect(result.updatedBox.width / result.updatedBox.height).toBeCloseTo(1, 1);
  expect(result.initialImage).not.toEqual(result.updatedImage);
});

test("python widget bundle handles a single-point sine signal snapshot", async ({ page }) => {
  await waitForDemoReady(page);

  const consoleErrors = [];
  const pageErrors = [];
  page.on("console", (message) => {
    if (message.type() === "error") {
      consoleErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    pageErrors.push(String(error));
  });

  const result = await page.evaluate(async (widgetSource) => {
    const model = {
      snapshot: {
        sizePx: [480, 320],
        title: "single-point sine signal",
        xLabel: "x",
        yLabel: "y",
        series: [
          {
            kind: "line",
            x: { kind: "static", values: [0] },
            y: {
              kind: "sine-signal",
              options: {
                points: 1,
                domainStart: 0,
                domainEnd: 0,
                amplitude: 1,
                cycles: 1,
                phaseVelocity: 0,
                phaseOffset: 0,
                verticalOffset: 0,
              },
            },
          },
        ],
      },
      get(name) {
        return name === "snapshot" ? this.snapshot : undefined;
      },
      on() {},
      off() {},
    };

    const waitForNextPaint = () =>
      new Promise((resolve) => {
        requestAnimationFrame(() => {
          requestAnimationFrame(resolve);
        });
      });

    const waitForCanvasChange = async (canvas) => {
      for (let attempt = 0; attempt < 120; attempt += 1) {
        await waitForNextPaint();
        const currentData = canvas.toDataURL("image/png");
        const blankCanvas = document.createElement("canvas");
        blankCanvas.width = canvas.width;
        blankCanvas.height = canvas.height;
        if (currentData !== blankCanvas.toDataURL("image/png")) {
          return currentData;
        }
      }

      throw new Error("widget canvas did not render");
    };

    const mount = document.createElement("div");
    document.body.appendChild(mount);

    const moduleUrl = URL.createObjectURL(new Blob([widgetSource], { type: "text/javascript" }));

    try {
      const mod = await import(moduleUrl);
      const cleanup = mod.default.render({ model, el: mount });
      const canvas = mount.querySelector("canvas");
      if (!(canvas instanceof HTMLCanvasElement)) {
        throw new Error("widget bundle did not create a canvas");
      }

      const image = await waitForCanvasChange(canvas);

      if (typeof cleanup === "function") {
        cleanup();
      }

      mount.remove();
      return { imageLength: image.length };
    } finally {
      URL.revokeObjectURL(moduleUrl);
    }
  }, PYTHON_WIDGET_BUNDLE);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
  expect(result.imageLength).toBeGreaterThan(0);
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

test("direct exports match snapshot and canvas-session exports", async ({ page }) => {
  await waitForDemoReady(page);

  const result = await page.evaluate(async () => {
    const demo = window.__ruvizDemo;
    if (!demo?.sdk) {
      throw new Error("SDK test hooks are unavailable");
    }

    const { createCanvasSession, createPlot, createPlotFromSnapshot } = demo.sdk;

    const sameBytes = (left, right) => {
      if (left.length !== right.length) {
        return {
          equal: false,
          reason: `length mismatch (${left.length} !== ${right.length})`,
        };
      }

      for (let index = 0; index < left.length; index += 1) {
        if (left[index] !== right[index]) {
          return {
            equal: false,
            reason: `first diff at byte ${index} (${left[index]} !== ${right[index]})`,
          };
        }
      }

      return { equal: true };
    };

    const createHiddenCanvas = () => {
      const canvas = document.createElement("canvas");
      canvas.width = 640;
      canvas.height = 320;
      canvas.style.cssText = [
        "position: fixed",
        "left: -10000px",
        "top: -10000px",
        "width: 640px",
        "height: 320px",
      ].join(";");
      document.body.appendChild(canvas);
      return canvas;
    };

    const cases = [
      {
        slug: "line-scatter",
        plot: createPlot()
          .sizePx(640, 320)
          .title("Parity Line + Scatter")
          .xlabel("x")
          .ylabel("y")
          .line({ x: [0, 1, 2, 3], y: [0.2, 1.0, 0.7, 1.5] })
          .scatter({ x: [0, 1, 2, 3], y: [0.3, 0.9, 0.8, 1.3] }),
      },
      {
        slug: "bar",
        plot: createPlot()
          .sizePx(640, 320)
          .title("Parity Bar")
          .ylabel("score")
          .bar({
            categories: ["CPU", "SVG", "GPU", "WASM"],
            values: [3.8, 2.6, 4.4, 4.9],
          }),
      },
      {
        slug: "heatmap",
        plot: createPlot()
          .sizePx(640, 320)
          .theme("dark")
          .title("Parity Heatmap")
          .heatmap([
            [0.1, 0.4, 0.8],
            [0.3, 0.5, 0.7],
            [0.2, 0.6, 0.9],
          ]),
      },
      {
        slug: "radar-mixed-names",
        plot: createPlot()
          .sizePx(640, 320)
          .title("Parity Radar")
          .radar({
            labels: ["Speed", "Power", "Defense", "Magic"],
            series: [
              { name: "Named", values: [0.9, 0.7, 0.8, 0.6] },
              { values: [0.4, 0.6, 0.5, 0.7] },
            ],
          }),
      },
    ];

    const comparisons = [];

    for (const entry of cases) {
      const direct = await entry.plot.renderPng();
      const fromSnapshot = await createPlotFromSnapshot(entry.plot.toSnapshot()).renderPng();

      const canvas = createHiddenCanvas();
      const session = await createCanvasSession(canvas, {
        autoResize: false,
        bindInput: false,
      });
      await session.setPlot(entry.plot.clone());
      session.resize(640, 320, 1);
      session.render();
      const fromSession = await session.exportPng();
      session.dispose();
      canvas.remove();

      comparisons.push({
        slug: entry.slug,
        directVsSnapshot: sameBytes(direct, fromSnapshot),
        directVsSession: sameBytes(direct, fromSession),
      });
    }

    return comparisons;
  });

  for (const entry of result) {
    expect(
      entry.directVsSnapshot.equal,
      `${entry.slug}: ${entry.directVsSnapshot.reason || "snapshot mismatch"}`,
    ).toBeTruthy();
    expect(
      entry.directVsSession.equal,
      `${entry.slug}: ${entry.directVsSession.reason || "session mismatch"}`,
    ).toBeTruthy();
  }
});
