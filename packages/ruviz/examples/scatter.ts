import { createPlot } from "../src/index.js";
import { scatterSeries } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "scatter",
  title: "Scatter plot",
  summary: "A scatter plot for irregular point clouds.",
  category: "basic",
  createPlot() {
    const { x, y } = scatterSeries();
    return createPlot()
      .sizePx(640, 320)
      .title("Scatter Plot")
      .xlabel("feature")
      .ylabel("response")
      .scatter({ x, y });
  },
};
