import { createPlot } from "../src/index.js";
import { sampleDistribution } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "boxplot",
  title: "Boxplot",
  summary: "Quartiles and outliers summarized as a boxplot.",
  category: "statistical",
  createPlot() {
    return createPlot().sizePx(640, 320).title("Boxplot").ylabel("value").boxplot(sampleDistribution());
  },
};
