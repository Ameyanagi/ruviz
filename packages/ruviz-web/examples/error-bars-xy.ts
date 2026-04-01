import { createPlot } from "../src/index.js";
import { errorBarXYSeries } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "error-bars-xy",
  title: "Horizontal and vertical error bars",
  summary: "A point series with uncertainty in both axes.",
  category: "statistical",
  createPlot() {
    const { x, y, xErrors, yErrors } = errorBarXYSeries();
    return createPlot()
      .sizePx(640, 320)
      .title("XY Error Bars")
      .xlabel("throughput")
      .ylabel("latency")
      .errorBarsXY({ x, y, xErrors, yErrors });
  },
};
