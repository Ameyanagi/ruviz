import { createPlot } from "../src/index.js";
import { sampleDistribution } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "histogram",
  title: "Histogram",
  summary: "A distribution view built from a deterministic sample.",
  category: "statistical",
  createPlot() {
    return createPlot()
      .sizePx(640, 320)
      .title("Histogram")
      .xlabel("value")
      .histogram(sampleDistribution());
  },
};
