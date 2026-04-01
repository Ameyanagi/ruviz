# Interactivity

The web runtime supports three main interactive flows:

- `plot.mount(canvas)` for standard main-thread canvas sessions
- `plot.mountWorker(canvas)` for worker-backed sessions with fallback
- `createObservable(...)` and `createSineSignal(...)` for reactive and temporal data

<PlotGallery :categories="['interactive']" />
