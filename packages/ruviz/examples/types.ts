import type { PlotBuilder } from "../src/index.js";

export interface PlotExampleDefinition {
  slug: string;
  title: string;
  summary: string;
  category: string;
  createPlot?: () => PlotBuilder;
  mount?: (canvas: HTMLCanvasElement) => Promise<(() => void) | void> | (() => void) | void;
}

export interface PlotExample extends PlotExampleDefinition {
  source: string;
}
