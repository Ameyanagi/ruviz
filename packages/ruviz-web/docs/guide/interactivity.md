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

## Reactive Data

Use `createObservable(...)` for mutable numeric series and
`createSineSignal(...)` for time-varying demo inputs. Those values can be
serialized into snapshots and rehydrated later.

Observable sources work with line, scatter, bar, histogram, boxplot, and
error-bar inputs. Sine signals can be used as the `y` source for line and
scatter plots. After mutating an observable or advancing session time, call
`session.render()` to draw the next frame.

<PlotGallery :categories="['interactive']" />
