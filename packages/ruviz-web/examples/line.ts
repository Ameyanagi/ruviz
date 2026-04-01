import { createPlot } from "../src/index.js";
import { waveSeries } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "line",
  title: "Line plot",
  summary: "A basic fluent line plot built with chained JS methods.",
  category: "basic",
  createPlot() {
    const { x, y } = waveSeries();
    return createPlot().sizePx(640, 320).title("Line Plot").xlabel("x").ylabel("signal").line({ x, y });
  },
};
