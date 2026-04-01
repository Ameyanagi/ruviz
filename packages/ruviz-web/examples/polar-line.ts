import { createPlot } from "../src/index.js";
import { polarSeries } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "polar-line",
  title: "Polar line",
  summary: "A polar line rendered from radius and angle vectors.",
  category: "specialized",
  createPlot() {
    const { r, theta } = polarSeries();
    return createPlot().sizePx(640, 320).title("Polar Line").polarLine({ r, theta });
  },
};
