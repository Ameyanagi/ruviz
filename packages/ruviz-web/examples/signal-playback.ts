import { createPlot, createSineSignal } from "../src/index.js";
import type { PlotExampleDefinition } from "./types.js";

export const example: PlotExampleDefinition = {
  slug: "signal-playback",
  title: "Signal playback",
  summary: "A temporal plot driven by the built-in sine signal helper.",
  category: "interactive",
  async mount(canvas) {
    const points = 120;
    const x = Array.from({ length: points }, (_, index) => (6 * index) / (points - 1));
    const signal = createSineSignal({
      points,
      domain: [0, 6],
      amplitude: 0.8,
      cycles: 2.8,
      phaseVelocity: 1.6,
      verticalOffset: 0.1,
    });

    const session = await createPlot()
      .sizePx(640, 320)
      .theme("dark")
      .title("Signal Playback")
      .xlabel("x")
      .ylabel("signal")
      .line({ x, y: signal })
      .mount(canvas, { initialTime: 0 });

    let time = 0;
    const timer = window.setInterval(() => {
      time += 0.15;
      session.setTime(time);
      session.render();
    }, 120);

    return () => {
      window.clearInterval(timer);
      session.destroy();
    };
  },
};
