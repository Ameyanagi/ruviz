import { createPlot } from "../src/index.js";
import { heatmapValues } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "heatmap",
  title: "Heatmap",
  summary: "A rectangular numeric matrix rendered as a heatmap.",
  category: "matrix",
  createPlot() {
    return createPlot().sizePx(640, 320).theme("dark").title("Heatmap").heatmap(heatmapValues());
  },
};
