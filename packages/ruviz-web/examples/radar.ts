import { createPlot } from "../src/index.js";
import { radarInputs } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "radar",
  title: "Radar chart",
  summary: "Multi-axis comparison for runtime capabilities.",
  category: "categorical",
  createPlot() {
    return createPlot().sizePx(640, 320).title("Runtime Radar").radar(radarInputs());
  },
};
