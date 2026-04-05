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
session instead of failing immediately.

## Input and Resize Wiring

By default session constructors:

- resize with the canvas element
- bind pointer and wheel input
- keep the session responsive to browser layout changes

You can disable that wiring through `CanvasSessionOptions` or
`WorkerSessionOptions` if you want to drive it yourself.

## Reactive Data

Use `createObservable(...)` for mutable numeric series and
`createSineSignal(...)` for time-varying demo inputs. Those values can be
serialized into snapshots and rehydrated later.

<PlotGallery :categories="['interactive']" />
