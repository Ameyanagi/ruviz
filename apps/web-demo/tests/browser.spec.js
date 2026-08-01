import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";

const PYTHON_WIDGET_BUNDLE = readFileSync(
  fileURLToPath(new URL("../../../python/ruviz_py/ruviz/widget.js", import.meta.url)),
  "utf8",
);
const DEMO_READY_TIMEOUT_MS = 45_000;

async function waitForDemoReady(page) {
  await page.goto("/");
  await expect(page).toHaveTitle("ruviz wasm demo", { timeout: DEMO_READY_TIMEOUT_MS });
  await expect(page.locator("#capability-status")).toContainText("default font:", {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
  await expect(page.locator("#export-status")).toContainText("ready", {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
  await expect(page.locator("#main-status")).toContainText("ready", {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
  await expect(page.locator("#temporal-status")).toContainText("ready", {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
  await expect(page.locator("#observable-status")).toContainText("ready", {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
  await expect(page.locator("#worker-status")).toHaveText(/ready|fallback|unavailable/, {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
  await page.waitForFunction(() => Boolean(window.__ruvizDemo?.mainSession), undefined, {
    timeout: DEMO_READY_TIMEOUT_MS,
  });
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

    const waitForElementBox = async (element, previousBox) => {
      for (let attempt = 0; attempt < 120; attempt += 1) {
        await waitForNextPaint();
        const rect = element.getBoundingClientRect();
        if (rect.width <= 0 || rect.height <= 0) {
          continue;
        }
        if (
          previousBox === undefined ||
          Math.abs(rect.width - previousBox.width) > 0.5 ||
          Math.abs(rect.height - previousBox.height) > 0.5
        ) {
          return { width: rect.width, height: rect.height };
        }
      }

      throw new Error("widget element size did not update");
    };

    const frame = document.createElement("div");
    frame.id = "python-widget-test-frame";
    frame.style.width = "1000px";
    document.body.appendChild(frame);

    const shell = document.createElement("div");
    shell.id = "python-widget-test-shell";
    shell.style.display = "flex";
    shell.style.flexDirection = "column";
    shell.style.alignItems = "stretch";
    shell.style.width = "100%";
    shell.style.background = "rgb(255, 255, 255)";
    frame.appendChild(shell);

    const mount = document.createElement("div");
    mount.id = "python-widget-test-host";
    shell.appendChild(mount);

    const moduleUrl = URL.createObjectURL(new Blob([widgetSource], { type: "text/javascript" }));

    try {
      const mod = await import(moduleUrl);
      const cleanup = mod.default.render({ model, el: mount });
      const canvas = mount.querySelector("canvas");
      const host = mount;
      const viewport = mount.querySelector("[data-ruviz-widget-viewport='true']");
      const shellBackground = getComputedStyle(shell).backgroundColor;
      const bodyBackground = getComputedStyle(document.body).backgroundColor;
      const htmlBackground = getComputedStyle(document.documentElement).backgroundColor;
      if (!(canvas instanceof HTMLCanvasElement)) {
        throw new Error("widget bundle did not create a canvas");
      }
      if (!(host instanceof HTMLDivElement) || host.dataset.ruvizWidgetHost !== "true") {
        throw new Error("widget bundle did not create a host");
      }
      if (!(viewport instanceof HTMLDivElement)) {
        throw new Error("widget bundle did not create a viewport");
      }

      const initialImage = await waitForCanvasChange(canvas);
      const initialBox = await waitForElementBox(viewport);
      const initialHostBox = await waitForElementBox(host);
      const initialShellBox = await waitForElementBox(shell);
      frame.style.width = "600px";
      const shrunkBox = await waitForElementBox(viewport, initialBox);
      const shrunkHostBox = await waitForElementBox(host, initialHostBox);
      const shrunkShellBox = await waitForElementBox(shell, initialShellBox);
      const shrunkImage = await waitForCanvasChange(canvas, initialImage);
      model.setSnapshot(buildSnapshot("widget updated", [1.1, 0.4, 1.0, 0.2], [360, 360]));
      const updatedImage = await waitForCanvasChange(canvas, shrunkImage);
      const updatedBox = await waitForElementBox(viewport, shrunkBox);
      const updatedHostBox = await waitForElementBox(host, shrunkHostBox);
      const updatedShellBox = await waitForElementBox(shell, shrunkShellBox);

      if (typeof cleanup === "function") {
        cleanup();
      }

      frame.remove();
      shell.remove();
      mount.remove();
      return {
        bodyBackground,
        htmlBackground,
        initialBox,
        initialHostBox,
        initialShellBox,
        initialImage,
        shellBackground,
        shrunkBox,
        shrunkHostBox,
        shrunkShellBox,
        shrunkImage,
        updatedBox,
        updatedHostBox,
        updatedShellBox,
        updatedImage,
      };
    } finally {
      URL.revokeObjectURL(moduleUrl);
    }
  }, PYTHON_WIDGET_BUNDLE);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
  expect(result.initialBox.width).toBeCloseTo(640, 0);
  expect(result.initialBox.height).toBeCloseTo(360, 0);
  expect(result.initialHostBox.width).toBeCloseTo(640, 0);
  expect(result.initialHostBox.height).toBeCloseTo(result.initialBox.height, 0);
  expect(result.initialShellBox.width).toBeCloseTo(640, 0);
  expect(result.initialShellBox.height).toBeCloseTo(result.initialHostBox.height, 0);
  expect(result.shrunkBox.width).toBeCloseTo(600, 0);
  expect(result.shrunkBox.height).toBeCloseTo(337.5, 0);
  expect(result.shrunkHostBox.width).toBeCloseTo(600, 0);
  expect(result.shrunkHostBox.height).toBeCloseTo(result.shrunkBox.height, 0);
  expect(result.shrunkShellBox.width).toBeCloseTo(600, 0);
  expect(result.shrunkShellBox.height).toBeCloseTo(result.shrunkHostBox.height, 0);
  expect(result.initialImage).not.toEqual(result.shrunkImage);
  expect(result.updatedBox.width).toBeCloseTo(360, 0);
  expect(result.updatedBox.height).toBeCloseTo(360, 0);
  expect(result.updatedHostBox.width).toBeCloseTo(360, 0);
  expect(result.updatedHostBox.height).toBeCloseTo(result.updatedBox.height, 0);
  expect(result.updatedShellBox.width).toBeCloseTo(360, 0);
  expect(result.updatedShellBox.height).toBeCloseTo(result.updatedHostBox.height, 0);
  expect(result.shellBackground).toContain("0, 0, 0, 0");
  expect(result.bodyBackground).toContain("0, 0, 0, 0");
  expect(result.htmlBackground).toContain("0, 0, 0, 0");
  expect(result.shrunkImage).not.toEqual(result.updatedImage);
});

test("python widget bundle uses the default PNG size when sizePx is omitted", async ({ page }) => {
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

    const snapshot = demo.sdk
      .createPlot()
      .setTitle("widget default size")
      .setXLabel("x")
      .setYLabel("y")
      .addLine({
        x: Array.from({ length: 60 }, (_, index) => index / 10),
        y: Array.from({ length: 60 }, (_, index) => Math.sin(index / 10)),
      })
      .toSnapshot();

    const model = {
      snapshot,
      get(name) {
        return name === "snapshot" ? this.snapshot : undefined;
      },
      on() {},
      off() {},
    };

    const waitForElementBox = async (element, previousBox = null) => {
      for (let attempt = 0; attempt < 30; attempt += 1) {
        await new Promise((resolve) => setTimeout(resolve, 50));
        const box = element.getBoundingClientRect();
        const candidate = { width: box.width, height: box.height };
        if (
          candidate.width > 0 &&
          candidate.height > 0 &&
          (!previousBox ||
            Math.abs(candidate.width - previousBox.width) > 0.5 ||
            Math.abs(candidate.height - previousBox.height) > 0.5)
        ) {
          return candidate;
        }
      }
      throw new Error("widget element size did not settle");
    };

    const frame = document.createElement("div");
    frame.id = "python-widget-default-size-frame";
    frame.style.width = "1000px";
    document.body.appendChild(frame);

    const shell = document.createElement("div");
    shell.id = "python-widget-default-size-shell";
    shell.style.display = "flex";
    shell.style.flexDirection = "column";
    shell.style.alignItems = "stretch";
    shell.style.width = "100%";
    shell.style.background = "rgb(255, 255, 255)";
    frame.appendChild(shell);

    const mount = document.createElement("div");
    mount.id = "python-widget-default-size-host";
    shell.appendChild(mount);

    const moduleUrl = URL.createObjectURL(new Blob([widgetSource], { type: "text/javascript" }));

    try {
      const mod = await import(moduleUrl);
      const cleanup = mod.default.render({ model, el: mount });
      const host = mount;
      const viewport = mount.querySelector("[data-ruviz-widget-viewport='true']");
      const shellBackground = getComputedStyle(shell).backgroundColor;
      if (!(host instanceof HTMLDivElement) || host.dataset.ruvizWidgetHost !== "true") {
        throw new Error("widget bundle did not create a host");
      }
      if (!(viewport instanceof HTMLDivElement)) {
        throw new Error("widget bundle did not create a viewport");
      }

      const roomyBox = await waitForElementBox(viewport);
      const roomyHostBox = await waitForElementBox(host);
      const roomyShellBox = await waitForElementBox(shell);
      frame.style.width = "500px";
      const constrainedBox = await waitForElementBox(viewport, roomyBox);
      const constrainedHostBox = await waitForElementBox(host, roomyHostBox);
      const constrainedShellBox = await waitForElementBox(shell, roomyShellBox);

      if (typeof cleanup === "function") {
        cleanup();
      }

      frame.remove();
      shell.remove();
      mount.remove();
      return {
        roomyBox,
        roomyHostBox,
        roomyShellBox,
        shellBackground,
        constrainedBox,
        constrainedHostBox,
        constrainedShellBox,
      };
    } finally {
      URL.revokeObjectURL(moduleUrl);
    }
  }, PYTHON_WIDGET_BUNDLE);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
  expect(result.roomyBox.width).toBeCloseTo(640, 0);
  expect(result.roomyBox.height).toBeCloseTo(480, 0);
  expect(result.roomyHostBox.width).toBeCloseTo(640, 0);
  expect(result.roomyHostBox.height).toBeCloseTo(result.roomyBox.height, 0);
  expect(result.roomyShellBox.width).toBeCloseTo(640, 0);
  expect(result.roomyShellBox.height).toBeCloseTo(result.roomyHostBox.height, 0);
  expect(result.constrainedBox.width).toBeCloseTo(500, 0);
  expect(result.constrainedBox.height).toBeCloseTo(375, 0);
  expect(result.constrainedHostBox.width).toBeCloseTo(500, 0);
  expect(result.constrainedHostBox.height).toBeCloseTo(result.constrainedBox.height, 0);
  expect(result.constrainedShellBox.width).toBeCloseTo(500, 0);
  expect(result.constrainedShellBox.height).toBeCloseTo(result.constrainedHostBox.height, 0);
  expect(result.shellBackground).toContain("0, 0, 0, 0");
});

test("python widget bundle can be resized by dragging the resize handle", async ({ page }) => {
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

  await page.evaluate(async (widgetSource) => {
    const demo = window.__ruvizDemo;
    if (!demo?.sdk) {
      throw new Error("SDK test hooks are unavailable");
    }

    const snapshot = demo.sdk
      .createPlot()
      .setSizePx(640, 360)
      .setTitle("widget resize handle")
      .setXLabel("x")
      .setYLabel("y")
      .addLine({
        x: Array.from({ length: 120 }, (_, index) => index / 8),
        y: Array.from({ length: 120 }, (_, index) => Math.sin(index / 8)),
      })
      .toSnapshot();

    const model = {
      snapshot,
      get(name) {
        return name === "snapshot" ? this.snapshot : undefined;
      },
      on() {},
      off() {},
    };

    const frame = document.createElement("div");
    frame.id = "python-widget-resize-frame";
    frame.style.width = "900px";
    document.body.appendChild(frame);

    const shell = document.createElement("div");
    shell.id = "python-widget-resize-shell";
    shell.style.display = "flex";
    shell.style.flexDirection = "column";
    shell.style.alignItems = "stretch";
    shell.style.width = "100%";
    shell.style.background = "rgb(255, 255, 255)";
    frame.appendChild(shell);

    const mount = document.createElement("div");
    mount.id = "python-widget-resize-host";
    shell.appendChild(mount);

    const moduleUrl = URL.createObjectURL(new Blob([widgetSource], { type: "text/javascript" }));
    const mod = await import(moduleUrl);
    const cleanup = mod.default.render({ model, el: mount });

    window.__ruvizWidgetResizeTest = {
      dispatchGesture: ({
        dx,
        dy,
        pointerId = 1,
        pointerType = "mouse",
        shiftKey = false,
        ctrlKey = false,
      }) => {
        const handle = document.querySelector("[data-ruviz-widget-resize-handle='true']");
        if (!(handle instanceof HTMLElement)) {
          throw new Error("widget bundle did not create a resize handle");
        }
        const rect = handle.getBoundingClientRect();
        const startX = rect.left + rect.width / 2;
        const startY = rect.top + rect.height / 2;
        const baseEvent = {
          bubbles: true,
          cancelable: true,
          composed: true,
          button: 0,
          isPrimary: true,
          pointerId,
          pointerType,
          shiftKey,
          ctrlKey,
        };
        handle.dispatchEvent(
          new PointerEvent("pointerdown", {
            ...baseEvent,
            buttons: 1,
            clientX: startX,
            clientY: startY,
          }),
        );
        document.dispatchEvent(
          new PointerEvent("pointermove", {
            ...baseEvent,
            buttons: 1,
            clientX: startX + dx,
            clientY: startY + dy,
          }),
        );
        document.dispatchEvent(
          new PointerEvent("pointerup", {
            ...baseEvent,
            buttons: 0,
            clientX: startX + dx,
            clientY: startY + dy,
          }),
        );
      },
      cleanup: () => {
        if (typeof cleanup === "function") {
          cleanup();
        }
        frame.remove();
        shell.remove();
        mount.remove();
        URL.revokeObjectURL(moduleUrl);
      },
    };
  }, PYTHON_WIDGET_BUNDLE);

  try {
    const host = page.locator("#python-widget-resize-host");
    const widgetShell = page.locator("#python-widget-resize-shell");
    const widgetHost = page.locator("#python-widget-resize-host[data-ruviz-widget-host='true']");
    const viewport = host.locator("[data-ruviz-widget-viewport='true']");

    const initialBox = await viewport.boundingBox();
    expect(initialBox).not.toBeNull();
    const initialRatio = (initialBox?.width ?? 1) / (initialBox?.height ?? 1);
    const initialHostBox = await widgetHost.boundingBox();
    const initialShellBox = await widgetShell.boundingBox();
    expect(initialHostBox).not.toBeNull();
    expect(initialShellBox).not.toBeNull();
    expect(initialHostBox.width).toBeCloseTo(initialBox.width, 0);
    expect(initialShellBox.width).toBeCloseTo(initialBox.width, 0);

    await page.evaluate(() => {
      window.__ruvizWidgetResizeTest.dispatchGesture({ dx: 120, dy: 60, pointerId: 1 });
    });

    await expect
      .poll(async () => (await viewport.boundingBox())?.width ?? 0)
      .toBeGreaterThan((initialBox?.width ?? 0) + 80);

    const freeformBox = await viewport.boundingBox();
    expect(freeformBox).not.toBeNull();
    const freeformHostBox = await widgetHost.boundingBox();
    const freeformShellBox = await widgetShell.boundingBox();
    expect(freeformHostBox).not.toBeNull();
    expect(freeformShellBox).not.toBeNull();
    expect(freeformBox.width).toBeCloseTo(760, 0);
    expect(freeformBox.height).toBeCloseTo(420, 0);
    expect(freeformHostBox.width).toBeCloseTo(freeformBox.width, 0);
    expect(freeformShellBox.width).toBeCloseTo(freeformBox.width, 0);
    expect(Math.abs(freeformBox.width / freeformBox.height - initialRatio)).toBeGreaterThan(0.01);

    await page.evaluate(() => {
      window.__ruvizWidgetResizeTest.dispatchGesture({
        dx: -40,
        dy: 50,
        pointerId: 2,
        pointerType: "touch",
      });
    });

    await expect
      .poll(async () => (await viewport.boundingBox())?.width ?? 0)
      .toBeLessThan(freeformBox.width - 20);

    const touchBox = await viewport.boundingBox();
    expect(touchBox).not.toBeNull();
    const touchHostBox = await widgetHost.boundingBox();
    const touchShellBox = await widgetShell.boundingBox();
    expect(touchHostBox).not.toBeNull();
    expect(touchShellBox).not.toBeNull();
    expect(touchBox.height).toBeCloseTo(470, 0);
    expect(touchHostBox.width).toBeCloseTo(touchBox.width, 0);
    expect(touchShellBox.width).toBeCloseTo(touchBox.width, 0);
    expect(Math.abs(touchBox.width / touchBox.height - initialRatio)).toBeGreaterThan(0.01);

    const touchRatio = touchBox.width / touchBox.height;
    await page.evaluate(() => {
      window.__ruvizWidgetResizeTest.dispatchGesture({
        dx: 100,
        dy: 20,
        pointerId: 3,
        shiftKey: true,
      });
    });

    await expect
      .poll(async () => (await viewport.boundingBox())?.width ?? 0)
      .toBeGreaterThan(touchBox.width + 60);

    const lockedBox = await viewport.boundingBox();
    expect(lockedBox).not.toBeNull();
    const lockedHostBox = await widgetHost.boundingBox();
    const lockedShellBox = await widgetShell.boundingBox();
    expect(lockedHostBox).not.toBeNull();
    expect(lockedShellBox).not.toBeNull();
    expect(lockedHostBox.width).toBeCloseTo(lockedBox.width, 0);
    expect(lockedShellBox.width).toBeCloseTo(lockedBox.width, 0);
    expect(Math.abs(lockedBox.width / lockedBox.height - touchRatio)).toBeLessThan(0.02);

    await page.evaluate(() => {
      window.__ruvizWidgetResizeTest.dispatchGesture({
        dx: -2000,
        dy: 10,
        pointerId: 4,
        ctrlKey: true,
      });
    });

    await expect
      .poll(async () => (await viewport.boundingBox())?.height ?? 0)
      .toBeLessThan(lockedBox.height - 200);

    const minLockedBox = await viewport.boundingBox();
    expect(minLockedBox).not.toBeNull();
    const minLockedHostBox = await widgetHost.boundingBox();
    const minLockedShellBox = await widgetShell.boundingBox();
    expect(minLockedHostBox).not.toBeNull();
    expect(minLockedShellBox).not.toBeNull();
    const minLockedRatio = lockedBox.width / lockedBox.height;
    const expectedMinWidth = Math.max(220, 160 * minLockedRatio);
    expect(minLockedBox.height).toBeCloseTo(expectedMinWidth / minLockedRatio, 0);
    expect(minLockedBox.width).toBeCloseTo(expectedMinWidth, 0);
    expect(minLockedHostBox.width).toBeCloseTo(minLockedBox.width, 0);
    expect(minLockedShellBox.width).toBeCloseTo(minLockedBox.width, 0);
    expect(Math.abs(minLockedBox.width / minLockedBox.height - minLockedRatio)).toBeLessThan(0.02);
  } finally {
    await page.evaluate(() => {
      window.__ruvizWidgetResizeTest?.cleanup?.();
      delete window.__ruvizWidgetResizeTest;
    });
  }

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});

test("python widget bundle uses a right-click export menu without breaking zoom", async ({
  page,
}) => {
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

  await page.evaluate(async (widgetSource) => {
    const demo = window.__ruvizDemo;
    if (!demo?.sdk) {
      throw new Error("SDK test hooks are unavailable");
    }

    const snapshot = demo.sdk
      .createPlot()
      .setSizePx(640, 360)
      .setTitle("widget export menu")
      .setXLabel("x")
      .setYLabel("y")
      .addLine({
        x: Array.from({ length: 120 }, (_, index) => index / 8),
        y: Array.from({ length: 120 }, (_, index) => Math.sin(index / 8)),
      })
      .toSnapshot();

    const model = {
      snapshot,
      get(name) {
        return name === "snapshot" ? this.snapshot : undefined;
      },
      on() {},
      off() {},
    };

    const mount = document.createElement("div");
    mount.id = "python-widget-context-menu-host";
    mount.style.width = "640px";
    document.body.appendChild(mount);

    const downloads = [];
    const originalClick = HTMLAnchorElement.prototype.click;
    HTMLAnchorElement.prototype.click = function click() {
      downloads.push({ download: this.download, href: this.href });
    };

    const moduleUrl = URL.createObjectURL(new Blob([widgetSource], { type: "text/javascript" }));
    const mod = await import(moduleUrl);
    const cleanup = mod.default.render({ model, el: mount });

    window.__ruvizWidgetContextMenuTest = {
      downloads,
      cleanup: () => {
        HTMLAnchorElement.prototype.click = originalClick;
        if (typeof cleanup === "function") {
          cleanup();
        }
        mount.remove();
        URL.revokeObjectURL(moduleUrl);
      },
    };
  }, PYTHON_WIDGET_BUNDLE);

  try {
    const host = page.locator("#python-widget-context-menu-host");
    const canvas = host.locator("canvas");
    const menu = host.locator("[data-ruviz-widget-menu='true']");
    const savePng = host.locator("[data-ruviz-widget-menu-item='png']");
    const saveSvg = host.locator("[data-ruviz-widget-menu-item='svg']");

    await expect(host).not.toContainText("Download PNG");
    await expect(host).not.toContainText("Download SVG");
    await expect(menu).toBeHidden();

    await canvas.click({ button: "right", position: { x: 140, y: 100 } });
    await expect(menu).toBeVisible();
    await expect(savePng).toBeVisible();
    await expect(saveSvg).toBeVisible();

    await page.mouse.click(8, 8);
    await expect(menu).toBeHidden();

    await canvas.click({ button: "right", position: { x: 160, y: 120 } });
    await expect(menu).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(menu).toBeHidden();

    await canvas.click({ button: "right", position: { x: 180, y: 130 } });
    await savePng.click();
    await page.waitForFunction(
      () =>
        window.__ruvizWidgetContextMenuTest?.downloads?.some(
          (entry) => entry.download === "ruviz.png",
        ) ?? false,
    );
    await expect(menu).toBeHidden();

    await canvas.click({ button: "right", position: { x: 200, y: 145 } });
    await saveSvg.click();
    await page.waitForFunction(
      () =>
        window.__ruvizWidgetContextMenuTest?.downloads?.some(
          (entry) => entry.download === "ruviz.svg",
        ) ?? false,
    );

    const beforeZoom = await canvas.screenshot();
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();

    await page.mouse.move(box.x + box.width * 0.32, box.y + box.height * 0.24);
    await page.mouse.down({ button: "right" });
    await page.mouse.move(box.x + box.width * 0.7, box.y + box.height * 0.48);
    await page.mouse.up({ button: "right" });

    await expect(menu).toBeHidden();
    const afterZoom = await canvas.screenshot();
    expect(beforeZoom.equals(afterZoom)).toBeFalsy();

    const downloads = await page.evaluate(
      () => window.__ruvizWidgetContextMenuTest?.downloads?.map((entry) => entry.download) ?? [],
    );
    expect(downloads).toEqual(["ruviz.png", "ruviz.svg"]);
  } finally {
    await page.evaluate(() => {
      window.__ruvizWidgetContextMenuTest?.cleanup?.();
      delete window.__ruvizWidgetContextMenuTest;
    });
  }

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
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

test("python widget bundle renders representative large snapshots without blacking out", async ({
  page,
}) => {
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

    const waitForNextPaint = () =>
      new Promise((resolve) => {
        requestAnimationFrame(() => {
          requestAnimationFrame(resolve);
        });
      });

    const waitForCanvasChange = async (canvas, previousData) => {
      for (let attempt = 0; attempt < 180; attempt += 1) {
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

      throw new Error("widget canvas did not update for large snapshot");
    };

    const analyzeCanvas = (canvas) => {
      const context = canvas.getContext("2d");
      if (!context) {
        throw new Error("widget canvas context is unavailable");
      }
      const { data } = context.getImageData(0, 0, canvas.width, canvas.height);
      const total = data.length / 4;
      let dark = 0;
      let ink = 0;
      for (let index = 0; index < data.length; index += 4) {
        const r = data[index];
        const g = data[index + 1];
        const b = data[index + 2];
        const a = data[index + 3];
        if (r < 32 && g < 32 && b < 32) {
          dark += 1;
        }
        if (a > 0 && (r < 248 || g < 248 || b < 248)) {
          ink += 1;
        }
      }
      return {
        darkFraction: dark / total,
        inkFraction: ink / total,
      };
    };

    const x = Array.from({ length: 100_000 }, (_, index) => index * 0.0001);
    const lineY = x.map((value) => Math.sin(value) + 0.2 * Math.cos(value * 3.0));
    const heatmap = Array.from({ length: 320 }, (_, rowIndex) => {
      const y = -1 + (2 * rowIndex) / 319;
      return Array.from({ length: 320 }, (_, colIndex) => {
        const xValue = -1 + (2 * colIndex) / 319;
        const ridge = Math.exp(-(((xValue - 0.25) ** 2 + (y + 0.1) ** 2) * 9.0));
        const waves = 0.35 * Math.sin(xValue * 8.0) * Math.cos(y * 6.0);
        return ridge + waves;
      });
    });
    const categories = Array.from({ length: 20_000 }, (_, index) => `c${index}`);
    const barValues = Array.from(
      { length: 20_000 },
      (_, index) => 1 + 0.45 * Math.sin(index * 0.001) + 0.1 * Math.cos(index * 0.004),
    );

    const snapshots = [
      {
        slug: "line",
        snapshot: demo.sdk
          .createPlot()
          .setSizePx(320, 200)
          .setTicks(false)
          .setTitle("large widget line")
          .addLine({ x, y: lineY })
          .toSnapshot(),
      },
      {
        slug: "heatmap",
        snapshot: demo.sdk
          .createPlot()
          .setSizePx(320, 200)
          .setTicks(false)
          .setTitle("large widget heatmap")
          .heatmap(heatmap)
          .toSnapshot(),
      },
      {
        slug: "bar",
        snapshot: demo.sdk
          .createPlot()
          .setSizePx(320, 200)
          .setTicks(false)
          .setTitle("large widget bar")
          .bar({ categories, values: barValues })
          .toSnapshot(),
      },
    ];

    const listeners = new Map();
    const model = {
      snapshot: snapshots[0].snapshot,
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

    const mount = document.createElement("div");
    mount.id = "python-widget-large-snapshots-host";
    mount.style.width = "720px";
    document.body.appendChild(mount);

    const moduleUrl = URL.createObjectURL(new Blob([widgetSource], { type: "text/javascript" }));

    try {
      const mod = await import(moduleUrl);
      const cleanup = mod.default.render({ model, el: mount });
      const canvas = mount.querySelector("canvas");
      const viewport = mount.querySelector("[data-ruviz-widget-viewport='true']");
      if (!(canvas instanceof HTMLCanvasElement)) {
        throw new Error("widget bundle did not create a canvas");
      }
      if (!(viewport instanceof HTMLDivElement)) {
        throw new Error("widget bundle did not create a viewport");
      }

      const metrics = [];
      let previousImage;
      for (const entry of snapshots) {
        if (entry !== snapshots[0]) {
          model.setSnapshot(entry.snapshot);
        }
        previousImage = await waitForCanvasChange(canvas, previousImage);
        metrics.push({
          slug: entry.slug,
          ...analyzeCanvas(canvas),
          width: viewport.getBoundingClientRect().width,
          height: viewport.getBoundingClientRect().height,
        });
      }

      if (typeof cleanup === "function") {
        cleanup();
      }
      mount.remove();
      return metrics;
    } finally {
      URL.revokeObjectURL(moduleUrl);
    }
  }, PYTHON_WIDGET_BUNDLE);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);

  for (const entry of result) {
    expect(entry.width, `${entry.slug} widget width`).toBeCloseTo(320, 0);
    expect(entry.height, `${entry.slug} widget height`).toBeCloseTo(200, 0);
    expect(entry.darkFraction, `${entry.slug} widget should not black out`).toBeLessThan(0.8);
    expect(
      entry.inkFraction,
      `${entry.slug} widget should contain visible plot ink`,
    ).toBeGreaterThan(0.001);
  }
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
