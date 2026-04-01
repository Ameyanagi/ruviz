import { createPlot } from "../src/index.js";
import { sampleDistribution } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "ecdf",
  title: "ECDF",
  summary: "An empirical cumulative distribution plot for ranked samples.",
  category: "statistical",
  createPlot() {
    return createPlot()
      .sizePx(640, 320)
      .title("ECDF")
      .xlabel("value")
      .ylabel("probability")
      .ecdf(sampleDistribution());
  },
};
