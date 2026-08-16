# Interactivity

The web SDK supports two interactive session modes plus reactive data helpers.

## Main-Thread Sessions

Use `createCanvasSession(...)` or `plot.mount(canvas)` when the plot should run
directly on a normal HTML canvas.

This path is the simplest option and is compatible with browsers that do not
support `OffscreenCanvas`.

## Worker Sessions

Use `createWorkerSession(...)` or `plot.mountWorker(canvas)` when you want the
interactive rendering lifecycle in a worker-backed session.

When `OffscreenCanvas` is unavailable, the SDK can fall back to a main-thread
session instead of failing immediately. The returned object is still a
`WorkerSession`, but `session.mode` is `"main-thread"`. Pass
`{ fallbackToMainThread: false }` to make unsupported worker rendering throw.

Worker sessions serialize plots with `plot.toSnapshot()` and rebuild them inside
`session-worker.js`. Observable sources are transferred as their current values,
and sine signals are transferred as normalized signal options.

## Input and Resize Wiring

By default session constructors:

- resize with the canvas element
- bind pointer and wheel input
- keep the session responsive to browser layout changes

You can disable that wiring through `CanvasSessionOptions` or
`WorkerSessionOptions` if you want to drive it yourself.

When input wiring is disabled, call `resize()`, `pointerDown()`,
`pointerMove()`, `pointerUp()`, `pointerLeave()`, and `wheel()` directly with
canvas pixel coordinates.

## Hit Queries and Series Visibility

The session surfaces what it already knows about the plot, so an app can
build its own hover readouts and clickable legends:

- `hitAt(x, y)` resolves the series data point near a canvas pixel to
  `{ seriesIndex, seriesLabel, pointIndex, dataX, dataY, distancePx }`, or
  `null` when nothing is close enough. Point-bearing series (line, scatter,
  error bars) answer; use it to render a DOM tooltip with full control over
  formatting.
- `legendEntryAt(x, y)` resolves a canvas pixel to the index of the series
  behind a legend entry, or `null`.
- `setSeriesVisible(index, visible)` shows or hides a series and re-renders.
  The hidden series keeps its colors and a dimmed legend entry, axis bounds
  hold still, and hit tests skip it; restoring reproduces the previous frame
  exactly. A series added inside a group toggles with its whole group, and
  the call reports `false` for an out-of-range index.
- `seriesCount()`, `seriesLabel(index)`, and `seriesVisible(index)` describe
  the attached plot.

On a `CanvasSession` these are synchronous; on a `WorkerSession` they return
promises because the answer comes from the worker. A typical legend toggle:

```ts
canvas.addEventListener("click", (event) => {
  const { x, y } = canvasPixel(event); // scale client coords by canvas.width / rect.width
  const entry = session.legendEntryAt(x, y);
  if (entry !== null) {
    session.setSeriesVisible(entry, !session.seriesVisible(entry));
  }
});
```

The built-in hover tooltip also improves on its own: it now leads with the
hovered series' label and formats values adaptively (scientific notation
outside `1e-3`–`1e5`), which log-scale plots need.

## Reactive Data

Use `createObservable(...)` for mutable numeric series and
`createSineSignal(...)` for time-varying demo inputs. Those values can be
serialized into snapshots and rehydrated later.

Observable sources work with line, scatter, bar, histogram, boxplot, and
error-bar inputs. Sine signals can be used as the `y` source for line and
scatter plots. After mutating an observable or advancing session time, call
`session.render()` to draw the next frame.

<PlotGallery :categories="['interactive']" />
