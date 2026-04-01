# Getting Started

Import the high-level SDK from the package root:

```ts
import { createPlot } from "ruviz";

const plot = createPlot()
  .sizePx(800, 420)
  .title("Quadratic")
  .xlabel("x")
  .ylabel("y")
  .line({ x: [0, 1, 2, 3], y: [0, 1, 4, 9] });

await plot.save({ format: "png", fileName: "quadratic.png" });
```

## Runtime Sessions

For browser interactivity, mount the builder into a canvas session:

```ts
const canvas = document.querySelector("canvas")!;
const session = await plot.mount(canvas);
```

Use `mountWorker(canvas)` when you want an OffscreenCanvas worker session with main-thread fallback.
