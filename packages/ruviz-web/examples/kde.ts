import { createPlot } from "../src/index.js";
import { sampleDistribution } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "kde",
  title: "Kernel density estimate",
  summary: "A smoothed density curve for a numeric sample.",
  category: "statistical",
  createPlot() {
    return createPlot().sizePx(640, 320).title("Kernel Density Estimate").xlabel("value").kde(sampleDistribution());
  },
};
