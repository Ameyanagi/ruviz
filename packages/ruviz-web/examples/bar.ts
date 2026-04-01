import { createPlot } from "../src/index.js";
import { categoricalSeries } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "bar",
  title: "Bar chart",
  summary: "Categorical metrics rendered as a bar chart.",
  category: "basic",
  createPlot() {
    const { categories, values } = categoricalSeries();
    return createPlot().sizePx(640, 320).title("Runtime Coverage").ylabel("score").bar({ categories, values });
  },
};
