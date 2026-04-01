import { createPlot } from "../src/index.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "pie",
  title: "Pie chart",
  summary: "A simple composition view with labels.",
  category: "categorical",
  createPlot() {
    return createPlot().sizePx(640, 320).title("Feature Mix").pie([30, 26, 24, 20], ["Exports", "Widgets", "WASM", "Docs"]);
  },
};
