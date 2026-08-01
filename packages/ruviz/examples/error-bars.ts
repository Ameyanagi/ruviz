import { createPlot } from "../src/index.js";
import { errorBarSeries } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "error-bars",
  title: "Vertical error bars",
  summary: "A series with y-direction uncertainty.",
  category: "statistical",
  createPlot() {
    const { x, y, errors } = errorBarSeries();
    return createPlot()
      .sizePx(640, 320)
      .title("Vertical Error Bars")
      .xlabel("trial")
      .ylabel("measurement")
      .errorBars({ x, y, yErrors: errors });
  },
};
