import { createPlot } from "../src/index.js";
import { contourGrid } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "contour",
  title: "Contour plot",
  summary: "Contours computed from a flattened z-grid over x/y axes.",
  category: "matrix",
  createPlot() {
    const { x, y, z } = contourGrid();
    return createPlot().sizePx(640, 320).title("Contour Plot").contour({ x, y, z });
  },
};
