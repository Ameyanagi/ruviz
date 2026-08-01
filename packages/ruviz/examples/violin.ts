import { createPlot } from "../src/index.js";
import { sampleDistribution } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "violin",
  title: "Violin plot",
  summary: "A violin plot for density and spread in one view.",
  category: "statistical",
  createPlot() {
    return createPlot().sizePx(640, 320).title("Violin Plot").ylabel("value").violin(sampleDistribution());
  },
};
