import { createObservable, createPlot } from "../src/index.js";
import { waveSeries } from "./data.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "observable-line",
  title: "Observable updates",
  summary: "A live line plot driven by an observable source.",
  category: "interactive",
  async mount(canvas) {
    const { x, y } = waveSeries();
    const observable = createObservable(y);
    const session = await createPlot()
      .sizePx(640, 320)
      .title("Observable Updates")
      .xlabel("x")
      .ylabel("signal")
      .line({ x, y: observable })
      .mount(canvas);

    let phase = 0;
    const timer = window.setInterval(() => {
      phase += 0.25;
      observable.replace(x.map((value) => Math.sin(value + phase) + 0.35 * Math.cos(value * 2.2 - phase)));
      session.render();
    }, 900);

    return () => {
      window.clearInterval(timer);
      session.destroy();
    };
  },
};
