import type { PlotExample, PlotExampleDefinition } from "./types.js";

import { example as bar } from "./bar.js";
import barSource from "./bar.ts?raw";
import { example as boxplot } from "./boxplot.js";
import boxplotSource from "./boxplot.ts?raw";
import { example as contour } from "./contour.js";
import contourSource from "./contour.ts?raw";
import { example as ecdf } from "./ecdf.js";
import ecdfSource from "./ecdf.ts?raw";
import { example as errorBars } from "./error-bars.js";
import errorBarsSource from "./error-bars.ts?raw";
import { example as errorBarsXY } from "./error-bars-xy.js";
import errorBarsXYSource from "./error-bars-xy.ts?raw";
import { example as heatmap } from "./heatmap.js";
import heatmapSource from "./heatmap.ts?raw";
import { example as histogram } from "./histogram.js";
import histogramSource from "./histogram.ts?raw";
import { example as kde } from "./kde.js";
import kdeSource from "./kde.ts?raw";
import { example as line } from "./line.js";
import lineSource from "./line.ts?raw";
import { example as observableLine } from "./observable-line.js";
import observableLineSource from "./observable-line.ts?raw";
import { example as pie } from "./pie.js";
import pieSource from "./pie.ts?raw";
import { example as polarLine } from "./polar-line.js";
import polarLineSource from "./polar-line.ts?raw";
import { example as radar } from "./radar.js";
import radarSource from "./radar.ts?raw";
import { example as scatter } from "./scatter.js";
import scatterSource from "./scatter.ts?raw";
import { example as signalPlayback } from "./signal-playback.js";
import signalPlaybackSource from "./signal-playback.ts?raw";
import { example as violin } from "./violin.js";
import violinSource from "./violin.ts?raw";
import { example as workerSession } from "./worker-session.js";
import workerSessionSource from "./worker-session.ts?raw";

function attachSource(example: PlotExampleDefinition, source: string): PlotExample {
  return { ...example, source };
}

export const webExamples: PlotExample[] = [
  attachSource(line, lineSource),
  attachSource(scatter, scatterSource),
  attachSource(bar, barSource),
  attachSource(histogram, histogramSource),
  attachSource(boxplot, boxplotSource),
  attachSource(heatmap, heatmapSource),
  attachSource(errorBars, errorBarsSource),
  attachSource(errorBarsXY, errorBarsXYSource),
  attachSource(kde, kdeSource),
  attachSource(ecdf, ecdfSource),
  attachSource(contour, contourSource),
  attachSource(pie, pieSource),
  attachSource(radar, radarSource),
  attachSource(violin, violinSource),
  attachSource(polarLine, polarLineSource),
  attachSource(observableLine, observableLineSource),
  attachSource(signalPlayback, signalPlaybackSource),
  attachSource(workerSession, workerSessionSource),
];

export function examplesForCategory(category: string): PlotExample[] {
  return webExamples.filter((example) => example.category === category);
}
