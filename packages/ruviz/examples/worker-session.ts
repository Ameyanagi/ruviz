import { createPlot } from "../src/index.js";
import { waveSeries } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "worker-session",
  title: "Worker session",
  summary: "Mount the same builder through the worker-backed canvas session.",
  category: "interactive",
  async mount(canvas) {
    const { x, y } = waveSeries();
    const session = await createPlot()
      .sizePx(640, 320)
      .theme("dark")
      .title("Worker Session")
      .xlabel("x")
      .ylabel("signal")
      .line({ x, y })
      .mountWorker(canvas);

    return () => {
      session.destroy();
    };
  },
};
