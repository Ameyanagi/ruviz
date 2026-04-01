import initRaw, * as raw from "../generated/raw/ruviz_web_raw.js";
import { clonePlotSnapshot, normalizeSineSignalOptions, toNumberArray, } from "./shared.js";
import { normalizeBackendPreference, toRawBackendPreference, } from "./plot-runtime.js";
let rawModulePromise = null;
function readBooleanProperty(record, snakeName, camelName) {
    if (!record || typeof record !== "object") {
        return false;
    }
    const snakeValue = record[snakeName];
    if (typeof snakeValue === "boolean") {
        return snakeValue;
    }
    const camelValue = record[camelName];
    return typeof camelValue === "boolean" ? camelValue : false;
}
async function ensureRawModule() {
    if (!rawModulePromise) {
        rawModulePromise = initRaw().then(() => {
            raw.register_default_browser_fonts_js();
            return raw;
        });
    }
    return rawModulePromise;
}
function normalizeCanvasSessionOptions(options) {
    const initialTime = options?.initialTime;
    return {
        backendPreference: normalizeBackendPreference(options?.backendPreference),
        autoResize: options?.autoResize ?? true,
        bindInput: options?.bindInput ?? true,
        initialTime: Number.isFinite(initialTime) ? initialTime : undefined,
    };
}
function normalizeWorkerSessionOptions(options) {
    const base = normalizeCanvasSessionOptions(options);
    return {
        ...base,
        fallbackToMainThread: options?.fallbackToMainThread ?? true,
    };
}
function getCanvasMetrics(canvas) {
    const rect = canvas.getBoundingClientRect();
    const scaleFactor = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(rect.width * scaleFactor));
    const height = Math.max(1, Math.round(rect.height * scaleFactor));
    return { width, height, scaleFactor, rect };
}
function pointerPosition(canvas, event) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = rect.width === 0 ? 1 : canvas.width / rect.width;
    const scaleY = rect.height === 0 ? 1 : canvas.height / rect.height;
    return {
        x: (event.clientX - rect.left) * scaleX,
        y: (event.clientY - rect.top) * scaleY,
    };
}
function installCanvasResize(canvas, onResize) {
    let lastWidth = 0;
    let lastHeight = 0;
    let lastScaleFactor = 0;
    const sync = () => {
        const { width, height, scaleFactor } = getCanvasMetrics(canvas);
        if (width === lastWidth &&
            height === lastHeight &&
            Math.abs(scaleFactor - lastScaleFactor) < 1e-6) {
            return;
        }
        lastWidth = width;
        lastHeight = height;
        lastScaleFactor = scaleFactor;
        onResize(width, height, scaleFactor);
    };
    sync();
    const observer = new ResizeObserver(sync);
    observer.observe(canvas);
    window.addEventListener("resize", sync);
    return () => {
        observer.disconnect();
        window.removeEventListener("resize", sync);
    };
}
function attachCanvasEvents(canvas, handlers) {
    const onContextMenu = (event) => {
        event.preventDefault();
    };
    const onPointerDown = (event) => {
        const point = pointerPosition(canvas, event);
        canvas.setPointerCapture(event.pointerId);
        handlers.pointerDown(point.x, point.y, event.button);
    };
    const onPointerMove = (event) => {
        const point = pointerPosition(canvas, event);
        handlers.pointerMove(point.x, point.y);
    };
    const onPointerUp = (event) => {
        const point = pointerPosition(canvas, event);
        if (canvas.hasPointerCapture(event.pointerId)) {
            canvas.releasePointerCapture(event.pointerId);
        }
        handlers.pointerUp(point.x, point.y, event.button);
    };
    const onPointerCancel = (event) => {
        if (canvas.hasPointerCapture(event.pointerId)) {
            canvas.releasePointerCapture(event.pointerId);
        }
        handlers.pointerLeave();
    };
    const onPointerLeave = () => {
        handlers.pointerLeave();
    };
    const onWheel = (event) => {
        event.preventDefault();
        const point = pointerPosition(canvas, event);
        handlers.wheel(event.deltaY, point.x, point.y);
    };
    canvas.addEventListener("contextmenu", onContextMenu);
    canvas.addEventListener("pointerdown", onPointerDown);
    canvas.addEventListener("pointermove", onPointerMove);
    canvas.addEventListener("pointerup", onPointerUp);
    canvas.addEventListener("pointercancel", onPointerCancel);
    canvas.addEventListener("pointerleave", onPointerLeave);
    canvas.addEventListener("wheel", onWheel, { passive: false });
    return () => {
        canvas.removeEventListener("contextmenu", onContextMenu);
        canvas.removeEventListener("pointerdown", onPointerDown);
        canvas.removeEventListener("pointermove", onPointerMove);
        canvas.removeEventListener("pointerup", onPointerUp);
        canvas.removeEventListener("pointercancel", onPointerCancel);
        canvas.removeEventListener("pointerleave", onPointerLeave);
        canvas.removeEventListener("wheel", onWheel);
    };
}
function workerCanvasSupported(canvas) {
    return (typeof window.Worker !== "undefined" &&
        typeof window.OffscreenCanvas !== "undefined" &&
        "transferControlToOffscreen" in canvas);
}
function toStringArray(values) {
    return Array.from(values, (value) => String(value));
}
function sourceLength(source) {
    switch (source.kind) {
        case "observable":
            return source.source.length;
        case "sine-signal":
            return source.source.length;
        case "static":
            return source.values.length;
    }
}
function normalizeXSource(source) {
    if (source instanceof ObservableSeries) {
        return { kind: "observable", source };
    }
    return { kind: "static", values: toNumberArray(source) };
}
function normalizeReactiveSource(source) {
    if (source instanceof ObservableSeries) {
        return { kind: "observable", source };
    }
    return { kind: "static", values: toNumberArray(source) };
}
function normalizeYSource(source) {
    if (source instanceof ObservableSeries) {
        return { kind: "observable", source };
    }
    if (source instanceof SineSignal) {
        return { kind: "sine-signal", source };
    }
    return { kind: "static", values: toNumberArray(source) };
}
async function ephemeralObservable(module, source) {
    if (source.kind === "observable") {
        return source.source._toRawObservable(module);
    }
    return new module.ObservableVecF64(Float64Array.from(source.values));
}
function applyPlotMetadata(rawPlot, state) {
    if (state.sizePx) {
        rawPlot.size_px(state.sizePx[0], state.sizePx[1]);
    }
    if (state.theme === "dark") {
        rawPlot.theme_dark();
    }
    else if (state.theme === "light") {
        rawPlot.theme_light();
    }
    if (typeof state.ticks === "boolean") {
        rawPlot.ticks(state.ticks);
    }
    if (state.title) {
        rawPlot.title(state.title);
    }
    if (state.xLabel) {
        rawPlot.xlabel(state.xLabel);
    }
    if (state.yLabel) {
        rawPlot.ylabel(state.yLabel);
    }
}
async function buildRawPlotFromState(state, module) {
    const rawPlot = new module.JsPlot();
    applyPlotMetadata(rawPlot, state);
    for (const series of state.series) {
        switch (series.kind) {
            case "line":
            case "scatter": {
                if (series.y.kind === "sine-signal") {
                    const signal = await series.y.source._toRawSignal(module);
                    const xValues = series.x.kind === "observable"
                        ? Float64Array.from(series.x.source.snapshotValues())
                        : Float64Array.from(series.x.values);
                    if (series.kind === "line") {
                        rawPlot.line_signal(xValues, signal);
                    }
                    else {
                        rawPlot.scatter_signal(xValues, signal);
                    }
                    break;
                }
                if (series.x.kind === "observable" || series.y.kind === "observable") {
                    const xObservable = await ephemeralObservable(module, series.x);
                    const yObservable = await ephemeralObservable(module, series.y);
                    if (series.kind === "line") {
                        rawPlot.line_observable(xObservable, yObservable);
                    }
                    else {
                        rawPlot.scatter_observable(xObservable, yObservable);
                    }
                    break;
                }
                const xValues = Float64Array.from(series.x.values);
                const yValues = Float64Array.from(series.y.values);
                if (series.kind === "line") {
                    rawPlot.line(xValues, yValues);
                }
                else {
                    rawPlot.scatter(xValues, yValues);
                }
                break;
            }
            case "bar": {
                if (series.values.kind === "observable") {
                    rawPlot.bar_observable(series.categories, await ephemeralObservable(module, series.values));
                }
                else {
                    rawPlot.bar(series.categories, Float64Array.from(series.values.values));
                }
                break;
            }
            case "histogram": {
                if (series.data.kind === "observable") {
                    rawPlot.histogram_observable(await ephemeralObservable(module, series.data));
                }
                else {
                    rawPlot.histogram(Float64Array.from(series.data.values));
                }
                break;
            }
            case "boxplot": {
                if (series.data.kind === "observable") {
                    rawPlot.boxplot_observable(await ephemeralObservable(module, series.data));
                }
                else {
                    rawPlot.boxplot(Float64Array.from(series.data.values));
                }
                break;
            }
            case "heatmap":
                rawPlot.heatmap(Float64Array.from(series.values), series.rows, series.cols);
                break;
            case "error-bars": {
                if (series.x.kind === "observable" ||
                    series.y.kind === "observable" ||
                    series.yErrors.kind === "observable") {
                    rawPlot.error_bars_observable(await ephemeralObservable(module, series.x), await ephemeralObservable(module, series.y), await ephemeralObservable(module, series.yErrors));
                }
                else {
                    rawPlot.error_bars(Float64Array.from(series.x.values), Float64Array.from(series.y.values), Float64Array.from(series.yErrors.values));
                }
                break;
            }
            case "error-bars-xy": {
                if (series.x.kind === "observable" ||
                    series.y.kind === "observable" ||
                    series.xErrors.kind === "observable" ||
                    series.yErrors.kind === "observable") {
                    rawPlot.error_bars_xy_observable(await ephemeralObservable(module, series.x), await ephemeralObservable(module, series.y), await ephemeralObservable(module, series.xErrors), await ephemeralObservable(module, series.yErrors));
                }
                else {
                    rawPlot.error_bars_xy(Float64Array.from(series.x.values), Float64Array.from(series.y.values), Float64Array.from(series.xErrors.values), Float64Array.from(series.yErrors.values));
                }
                break;
            }
            case "kde":
                rawPlot.kde(Float64Array.from(series.data));
                break;
            case "ecdf":
                rawPlot.ecdf(Float64Array.from(series.data));
                break;
            case "contour":
                rawPlot.contour(Float64Array.from(series.x), Float64Array.from(series.y), Float64Array.from(series.z));
                break;
            case "pie":
                if (series.labels && series.labels.length > 0) {
                    rawPlot.pie_with_labels(Float64Array.from(series.values), series.labels);
                }
                else {
                    rawPlot.pie(Float64Array.from(series.values));
                }
                break;
            case "radar": {
                const flattened = [];
                const names = [];
                for (const item of series.series) {
                    flattened.push(...item.values);
                    if (item.name) {
                        names.push(item.name);
                    }
                }
                rawPlot.radar(series.labels, names, Float64Array.from(flattened));
                break;
            }
            case "violin":
                rawPlot.violin(Float64Array.from(series.data));
                break;
            case "polar-line":
                rawPlot.polar_line(Float64Array.from(series.r), Float64Array.from(series.theta));
                break;
        }
    }
    return rawPlot;
}
export class ObservableSeries {
    #values;
    #rawHandle;
    constructor(values) {
        this.#values = toNumberArray(values);
        this.#rawHandle = null;
    }
    get length() {
        return this.#values.length;
    }
    replace(values) {
        const nextValues = toNumberArray(values);
        this.#values = nextValues;
        if (this.#rawHandle) {
            this.#rawHandle.replace(Float64Array.from(nextValues));
        }
    }
    setAt(index, value) {
        if (!Number.isInteger(index) || index < 0 || index >= this.#values.length) {
            throw new RangeError("observable index is out of bounds");
        }
        this.#values[index] = value;
        this.#rawHandle?.set_at(index, value);
    }
    values() {
        return Float64Array.from(this.#values);
    }
    snapshotValues() {
        return [...this.#values];
    }
    async _toRawObservable(module) {
        const currentModule = module ?? (await ensureRawModule());
        if (!this.#rawHandle) {
            this.#rawHandle = new currentModule.ObservableVecF64(Float64Array.from(this.snapshotValues()));
        }
        return this.#rawHandle;
    }
}
export class SineSignal {
    options;
    #rawHandle;
    constructor(options) {
        this.options = normalizeSineSignalOptions(options);
        this.#rawHandle = null;
    }
    get length() {
        return this.options.points;
    }
    valuesAt(timeSeconds) {
        const { amplitude, cycles, domainEnd, domainStart, phaseOffset, phaseVelocity, points, verticalOffset, } = this.options;
        const span = domainEnd - domainStart;
        const phase = phaseOffset + phaseVelocity * timeSeconds;
        const denominator = points - 1;
        return Float64Array.from(Array.from({ length: points }, (_, index) => {
            const progress = index / denominator;
            const x = domainStart + span * progress;
            return verticalOffset + amplitude * Math.sin(cycles * x + phase);
        }));
    }
    async _toRawSignal(module) {
        const currentModule = module ?? (await ensureRawModule());
        if (!this.#rawHandle) {
            const { amplitude, cycles, domainEnd, domainStart, phaseOffset, phaseVelocity, points, verticalOffset, } = this.options;
            this.#rawHandle = currentModule.SignalVecF64.sineWave(points, domainStart, domainEnd, amplitude, cycles, phaseVelocity, phaseOffset, verticalOffset);
        }
        return this.#rawHandle;
    }
}
function flattenHeatmapValues(input) {
    const rows = input.length;
    if (rows === 0) {
        throw new Error("heatmap input must contain at least one row");
    }
    const normalizedRows = input.map((row) => toNumberArray(row));
    const cols = normalizedRows[0]?.length ?? 0;
    if (cols === 0) {
        throw new Error("heatmap rows must contain at least one value");
    }
    if (normalizedRows.some((row) => row.length !== cols)) {
        throw new Error("heatmap rows must all have the same length");
    }
    return {
        values: normalizedRows.flat(),
        rows,
        cols,
    };
}
function defaultSaveFileName(format) {
    return format === "svg" ? "ruviz.svg" : "ruviz.png";
}
function downloadBlob(blob, fileName) {
    const href = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = href;
    anchor.download = fileName;
    anchor.click();
    URL.revokeObjectURL(href);
}
export class PlotBuilder {
    #state;
    constructor(state) {
        this.#state = state
            ? {
                sizePx: state.sizePx ? [...state.sizePx] : undefined,
                theme: state.theme,
                ticks: state.ticks,
                title: state.title,
                xLabel: state.xLabel,
                yLabel: state.yLabel,
                series: state.series.map((series) => PlotBuilder.#cloneSeries(series)),
            }
            : { series: [] };
    }
    static fromSnapshot(snapshot) {
        return new PlotBuilder(PlotBuilder.#stateFromSnapshot(snapshot));
    }
    static #stateFromSnapshot(snapshot) {
        return {
            sizePx: snapshot.sizePx ? [...snapshot.sizePx] : undefined,
            theme: snapshot.theme,
            ticks: snapshot.ticks,
            title: snapshot.title,
            xLabel: snapshot.xLabel,
            yLabel: snapshot.yLabel,
            series: snapshot.series.map((series) => {
                switch (series.kind) {
                    case "line":
                    case "scatter":
                        return {
                            kind: series.kind,
                            x: PlotBuilder.#sourceFromSnapshot(series.x),
                            y: PlotBuilder.#ySourceFromSnapshot(series.y),
                        };
                    case "bar":
                        return {
                            kind: "bar",
                            categories: [...series.categories],
                            values: PlotBuilder.#sourceFromSnapshot(series.values),
                        };
                    case "histogram":
                        return {
                            kind: "histogram",
                            data: PlotBuilder.#sourceFromSnapshot(series.data),
                        };
                    case "boxplot":
                        return {
                            kind: "boxplot",
                            data: PlotBuilder.#sourceFromSnapshot(series.data),
                        };
                    case "error-bars":
                        return {
                            kind: "error-bars",
                            x: PlotBuilder.#sourceFromSnapshot(series.x),
                            y: PlotBuilder.#sourceFromSnapshot(series.y),
                            yErrors: PlotBuilder.#sourceFromSnapshot(series.yErrors),
                        };
                    case "error-bars-xy":
                        return {
                            kind: "error-bars-xy",
                            x: PlotBuilder.#sourceFromSnapshot(series.x),
                            y: PlotBuilder.#sourceFromSnapshot(series.y),
                            xErrors: PlotBuilder.#sourceFromSnapshot(series.xErrors),
                            yErrors: PlotBuilder.#sourceFromSnapshot(series.yErrors),
                        };
                    default:
                        return clonePlotSnapshot({ series: [series] }).series[0];
                }
            }),
        };
    }
    static #sourceFromSnapshot(source) {
        if (source.kind === "observable") {
            return { kind: "observable", source: new ObservableSeries(source.values) };
        }
        return { kind: "static", values: [...source.values] };
    }
    static #ySourceFromSnapshot(source) {
        if (source.kind === "sine-signal") {
            return {
                kind: "sine-signal",
                source: new SineSignal({
                    points: source.options.points,
                    domain: [source.options.domainStart, source.options.domainEnd],
                    amplitude: source.options.amplitude,
                    cycles: source.options.cycles,
                    phaseVelocity: source.options.phaseVelocity,
                    phaseOffset: source.options.phaseOffset,
                    verticalOffset: source.options.verticalOffset,
                }),
            };
        }
        return PlotBuilder.#sourceFromSnapshot(source);
    }
    static #cloneSeries(series) {
        switch (series.kind) {
            case "line":
            case "scatter":
                return {
                    kind: series.kind,
                    x: series.x.kind === "observable"
                        ? series.x
                        : { kind: "static", values: [...series.x.values] },
                    y: series.y.kind === "static"
                        ? { kind: "static", values: [...series.y.values] }
                        : series.y,
                };
            case "bar":
                return {
                    kind: "bar",
                    categories: [...series.categories],
                    values: series.values.kind === "observable"
                        ? series.values
                        : { kind: "static", values: [...series.values.values] },
                };
            case "histogram":
                return {
                    kind: "histogram",
                    data: series.data.kind === "observable"
                        ? series.data
                        : { kind: "static", values: [...series.data.values] },
                };
            case "boxplot":
                return {
                    kind: "boxplot",
                    data: series.data.kind === "observable"
                        ? series.data
                        : { kind: "static", values: [...series.data.values] },
                };
            case "error-bars":
                return {
                    kind: "error-bars",
                    x: series.x.kind === "observable" ? series.x : { kind: "static", values: [...series.x.values] },
                    y: series.y.kind === "observable" ? series.y : { kind: "static", values: [...series.y.values] },
                    yErrors: series.yErrors.kind === "observable"
                        ? series.yErrors
                        : { kind: "static", values: [...series.yErrors.values] },
                };
            case "error-bars-xy":
                return {
                    kind: "error-bars-xy",
                    x: series.x.kind === "observable" ? series.x : { kind: "static", values: [...series.x.values] },
                    y: series.y.kind === "observable" ? series.y : { kind: "static", values: [...series.y.values] },
                    xErrors: series.xErrors.kind === "observable"
                        ? series.xErrors
                        : { kind: "static", values: [...series.xErrors.values] },
                    yErrors: series.yErrors.kind === "observable"
                        ? series.yErrors
                        : { kind: "static", values: [...series.yErrors.values] },
                };
            default:
                return clonePlotSnapshot({ series: [series] }).series[0];
        }
    }
    sizePx(width, height) {
        return this.setSizePx(width, height);
    }
    setSizePx(width, height) {
        this.#state.sizePx = [Math.max(1, Math.round(width)), Math.max(1, Math.round(height))];
        return this;
    }
    theme(theme) {
        return this.setTheme(theme);
    }
    setTheme(theme) {
        this.#state.theme = theme;
        return this;
    }
    ticks(enabled) {
        return this.setTicks(enabled);
    }
    setTicks(enabled) {
        this.#state.ticks = enabled;
        return this;
    }
    title(title) {
        return this.setTitle(title);
    }
    setTitle(title) {
        this.#state.title = title;
        return this;
    }
    xlabel(label) {
        return this.setXLabel(label);
    }
    setXLabel(label) {
        this.#state.xLabel = label;
        return this;
    }
    ylabel(label) {
        return this.setYLabel(label);
    }
    setYLabel(label) {
        this.#state.yLabel = label;
        return this;
    }
    line(input) {
        return this.#addXYSeries("line", input);
    }
    addLine(input) {
        return this.line(input);
    }
    scatter(input) {
        return this.#addXYSeries("scatter", input);
    }
    addScatter(input) {
        return this.scatter(input);
    }
    bar(input) {
        const categories = toStringArray(input.categories);
        const values = normalizeReactiveSource(input.values);
        if (categories.length !== sourceLength(values)) {
            throw new Error("bar categories and values must have the same length");
        }
        this.#state.series.push({ kind: "bar", categories, values });
        return this;
    }
    histogram(input) {
        const values = normalizeReactiveSource(input);
        this.#state.series.push({ kind: "histogram", data: values });
        return this;
    }
    boxplot(input) {
        const values = normalizeReactiveSource(input);
        this.#state.series.push({ kind: "boxplot", data: values });
        return this;
    }
    heatmap(input) {
        const { values, rows, cols } = flattenHeatmapValues(input);
        this.#state.series.push({ kind: "heatmap", values, rows, cols });
        return this;
    }
    errorBars(input) {
        const x = normalizeReactiveSource(input.x);
        const y = normalizeReactiveSource(input.y);
        const yErrors = normalizeReactiveSource(input.yErrors);
        if (sourceLength(x) !== sourceLength(y) ||
            sourceLength(x) !== sourceLength(yErrors)) {
            throw new Error("x, y, and yErrors must have the same length");
        }
        this.#state.series.push({ kind: "error-bars", x, y, yErrors });
        return this;
    }
    errorBarsXY(input) {
        const x = normalizeReactiveSource(input.x);
        const y = normalizeReactiveSource(input.y);
        const xErrors = normalizeReactiveSource(input.xErrors);
        const yErrors = normalizeReactiveSource(input.yErrors);
        const expected = sourceLength(x);
        if (expected !== sourceLength(y) ||
            expected !== sourceLength(xErrors) ||
            expected !== sourceLength(yErrors)) {
            throw new Error("x, y, xErrors, and yErrors must have the same length");
        }
        this.#state.series.push({ kind: "error-bars-xy", x, y, xErrors, yErrors });
        return this;
    }
    kde(input) {
        this.#state.series.push({ kind: "kde", data: toNumberArray(input) });
        return this;
    }
    ecdf(input) {
        this.#state.series.push({ kind: "ecdf", data: toNumberArray(input) });
        return this;
    }
    contour(input) {
        const x = toNumberArray(input.x);
        const y = toNumberArray(input.y);
        const z = toNumberArray(input.z);
        if (x.length === 0 || y.length === 0 || z.length !== x.length * y.length) {
            throw new Error("contour z must contain x.length * y.length values");
        }
        this.#state.series.push({ kind: "contour", x, y, z });
        return this;
    }
    pie(values, labelsInput) {
        const normalizedValues = toNumberArray(values);
        const labels = labelsInput ? toStringArray(labelsInput) : undefined;
        if (labels && labels.length !== normalizedValues.length) {
            throw new Error("pie values and labels must have the same length");
        }
        this.#state.series.push({ kind: "pie", values: normalizedValues, labels });
        return this;
    }
    radar(input) {
        const labels = toStringArray(input.labels);
        if (labels.length === 0) {
            throw new Error("radar labels must not be empty");
        }
        const series = input.series.map((item) => {
            const values = toNumberArray(item.values);
            if (values.length !== labels.length) {
                throw new Error("each radar series must match the labels length");
            }
            return { name: item.name, values };
        });
        if (series.length === 0) {
            throw new Error("radar input must contain at least one series");
        }
        this.#state.series.push({ kind: "radar", labels, series });
        return this;
    }
    violin(input) {
        this.#state.series.push({ kind: "violin", data: toNumberArray(input) });
        return this;
    }
    polarLine(input) {
        const r = toNumberArray(input.r);
        const theta = toNumberArray(input.theta);
        if (r.length !== theta.length) {
            throw new Error("polar r and theta must have the same length");
        }
        this.#state.series.push({ kind: "polar-line", r, theta });
        return this;
    }
    clone() {
        return PlotBuilder.fromSnapshot(this.toSnapshot());
    }
    toSnapshot() {
        const snapshot = {
            sizePx: this.#state.sizePx ? [...this.#state.sizePx] : undefined,
            theme: this.#state.theme,
            ticks: this.#state.ticks,
            title: this.#state.title,
            xLabel: this.#state.xLabel,
            yLabel: this.#state.yLabel,
            series: this.#state.series.map((series) => this.#serializeSeries(series)),
        };
        return clonePlotSnapshot(snapshot);
    }
    async renderPng() {
        const module = await ensureRawModule();
        const rawPlot = await this._toRawPlot(module);
        return rawPlot.render_png_bytes();
    }
    async renderSvg() {
        const module = await ensureRawModule();
        const rawPlot = await this._toRawPlot(module);
        return rawPlot.render_svg();
    }
    async save(options) {
        const format = options?.format ?? "png";
        const fileName = options?.fileName ?? defaultSaveFileName(format);
        if (format === "svg") {
            const svg = await this.renderSvg();
            downloadBlob(new Blob([svg], { type: "image/svg+xml" }), fileName);
            return;
        }
        const png = await this.renderPng();
        const blobBytes = new Uint8Array(png);
        downloadBlob(new Blob([blobBytes.buffer], { type: "image/png" }), fileName);
    }
    async mount(canvas, options) {
        const session = await createCanvasSession(canvas, options);
        await session.setPlot(this);
        session.render();
        return session;
    }
    async mountWorker(canvas, options) {
        const session = await createWorkerSession(canvas, options);
        await session.setPlot(this);
        session.render();
        return session;
    }
    async _toRawPlot(module) {
        const currentModule = module ?? (await ensureRawModule());
        return buildRawPlotFromState(this.#state, currentModule);
    }
    #addXYSeries(kind, input) {
        const x = normalizeXSource(input.x);
        const y = normalizeYSource(input.y);
        if (sourceLength(x) !== sourceLength(y)) {
            throw new Error("x and y must have the same length");
        }
        this.#state.series.push({ kind, x, y });
        return this;
    }
    #serializeSeries(series) {
        switch (series.kind) {
            case "line":
            case "scatter":
                return {
                    kind: series.kind,
                    x: this.#serializeXSource(series.x),
                    y: this.#serializeYSource(series.y),
                };
            case "bar":
                return {
                    kind: "bar",
                    categories: [...series.categories],
                    values: this.#serializeReactiveSource(series.values),
                };
            case "histogram":
                return {
                    kind: "histogram",
                    data: this.#serializeReactiveSource(series.data),
                };
            case "boxplot":
                return {
                    kind: "boxplot",
                    data: this.#serializeReactiveSource(series.data),
                };
            case "error-bars":
                return {
                    kind: "error-bars",
                    x: this.#serializeReactiveSource(series.x),
                    y: this.#serializeReactiveSource(series.y),
                    yErrors: this.#serializeReactiveSource(series.yErrors),
                };
            case "error-bars-xy":
                return {
                    kind: "error-bars-xy",
                    x: this.#serializeReactiveSource(series.x),
                    y: this.#serializeReactiveSource(series.y),
                    xErrors: this.#serializeReactiveSource(series.xErrors),
                    yErrors: this.#serializeReactiveSource(series.yErrors),
                };
            default:
                return clonePlotSnapshot({ series: [series] }).series[0];
        }
    }
    #serializeXSource(source) {
        return this.#serializeReactiveSource(source);
    }
    #serializeReactiveSource(source) {
        if (source.kind === "observable") {
            return { kind: "observable", values: source.source.snapshotValues() };
        }
        return { kind: "static", values: [...source.values] };
    }
    #serializeYSource(source) {
        if (source.kind === "sine-signal") {
            return { kind: "sine-signal", options: { ...source.source.options } };
        }
        return this.#serializeReactiveSource(source);
    }
}
export class CanvasSession {
    mode = "main-thread";
    #module;
    #rawSession;
    #canvas;
    #cleanup;
    constructor(module, rawSession, canvas) {
        this.#module = module;
        this.#rawSession = rawSession;
        this.#canvas = canvas;
        this.#cleanup = [];
    }
    hasPlot() {
        return this.#rawSession.has_plot();
    }
    #withAttachedPlot(fn) {
        if (!this.hasPlot()) {
            return;
        }
        fn();
    }
    #withAttachedPlotResult(fn) {
        if (!this.hasPlot()) {
            return null;
        }
        return fn();
    }
    async setPlot(plot) {
        const rawPlot = await plot._toRawPlot(this.#module);
        this.#rawSession.set_plot(rawPlot);
    }
    resize(width, height, scaleFactor) {
        const nextMetrics = typeof width === "number" && typeof height === "number" && typeof scaleFactor === "number"
            ? { width, height, scaleFactor }
            : getCanvasMetrics(this.#canvas);
        this.#rawSession.resize(nextMetrics.width, nextMetrics.height, nextMetrics.scaleFactor);
    }
    setTime(timeSeconds) {
        this.#rawSession.set_time(timeSeconds);
    }
    setBackendPreference(backendPreference) {
        this.#rawSession.set_backend_preference(toRawBackendPreference(this.#module, normalizeBackendPreference(backendPreference)));
    }
    render() {
        this.#withAttachedPlot(() => {
            this.#rawSession.render();
        });
    }
    resetView() {
        this.#withAttachedPlot(() => {
            this.#rawSession.reset_view();
        });
    }
    pointerDown(x, y, button) {
        this.#withAttachedPlot(() => {
            this.#rawSession.pointer_down(x, y, button);
        });
    }
    pointerMove(x, y) {
        this.#withAttachedPlot(() => {
            this.#rawSession.pointer_move(x, y);
        });
    }
    pointerUp(x, y, button) {
        this.#withAttachedPlot(() => {
            this.#rawSession.pointer_up(x, y, button);
        });
    }
    pointerLeave() {
        this.#withAttachedPlot(() => {
            this.#rawSession.pointer_leave();
        });
    }
    wheel(deltaY, x, y) {
        this.#withAttachedPlot(() => {
            this.#rawSession.wheel(deltaY, x, y);
        });
    }
    async exportPng() {
        const result = this.#withAttachedPlotResult(() => this.#rawSession.export_png());
        if (result === null) {
            throw new Error("no plot is attached to this session");
        }
        return result;
    }
    async exportSvg() {
        const result = this.#withAttachedPlotResult(() => this.#rawSession.export_svg());
        if (result === null) {
            throw new Error("no plot is attached to this session");
        }
        return result;
    }
    destroy() {
        this.#rawSession.destroy();
    }
    dispose() {
        for (const dispose of this.#cleanup.splice(0)) {
            dispose();
        }
        this.#rawSession.destroy();
    }
    _pushCleanup(dispose) {
        this.#cleanup.push(dispose);
    }
}
export class WorkerSession {
    mode;
    #canvas;
    #worker;
    #fallbackSession;
    #cleanup;
    #pendingRequests;
    #nextRequestId;
    #readyPromise;
    #resolveReady;
    #rejectReady;
    #hasPlot;
    #sessionEpoch;
    constructor(canvas, mode, fallbackSession) {
        this.mode = mode;
        this.#canvas = canvas;
        this.#worker = null;
        this.#fallbackSession = fallbackSession ?? null;
        this.#cleanup = [];
        this.#pendingRequests = new Map();
        this.#nextRequestId = 1;
        this.#hasPlot = false;
        this.#sessionEpoch = 0;
        this.#resolveReady = null;
        this.#rejectReady = null;
        if (this.#fallbackSession) {
            this.#readyPromise = Promise.resolve();
        }
        else {
            this.#readyPromise = new Promise((resolve, reject) => {
                this.#resolveReady = resolve;
                this.#rejectReady = reject;
            });
        }
    }
    get canvas() {
        return this.#canvas;
    }
    attachWorker(worker) {
        this.#worker = worker;
        worker.onmessage = (event) => this.#handleMessage(event);
        worker.onerror = (event) => {
            this.#failAll(new Error(event.message || "worker session error"));
        };
        worker.onmessageerror = () => {
            this.#failAll(new Error("worker session failed to decode a worker message"));
        };
    }
    hasPlot() {
        return this.#fallbackSession ? this.#fallbackSession.hasPlot() : this.#hasPlot;
    }
    #canDispatchPlotCommand() {
        return this.#fallbackSession ? this.#fallbackSession.hasPlot() : this.#hasPlot;
    }
    async setPlot(plot) {
        if (this.#fallbackSession) {
            await this.#fallbackSession.setPlot(plot);
            return;
        }
        const sessionEpoch = this.#sessionEpoch;
        try {
            await this.#request("setPlot", { snapshot: plot.toSnapshot() });
            if (sessionEpoch !== this.#sessionEpoch || !this.#worker) {
                return;
            }
            this.#hasPlot = true;
        }
        catch (error) {
            if (sessionEpoch === this.#sessionEpoch) {
                this.#hasPlot = false;
            }
            throw error;
        }
    }
    resize(width, height, scaleFactor) {
        if (this.#fallbackSession) {
            this.#fallbackSession.resize(width, height, scaleFactor);
            return;
        }
        const nextMetrics = typeof width === "number" && typeof height === "number" && typeof scaleFactor === "number"
            ? { width, height, scaleFactor }
            : getCanvasMetrics(this.#canvas);
        this.#post("resize", nextMetrics);
    }
    setTime(timeSeconds) {
        if (this.#fallbackSession) {
            this.#fallbackSession.setTime(timeSeconds);
            return;
        }
        if (!this.#worker) {
            return;
        }
        this.#post("setTime", { timeSeconds });
    }
    setBackendPreference(backendPreference) {
        if (this.#fallbackSession) {
            this.#fallbackSession.setBackendPreference(backendPreference);
            return;
        }
        this.#post("setBackendPreference", {
            backendPreference: normalizeBackendPreference(backendPreference),
        });
    }
    render() {
        if (this.#fallbackSession) {
            this.#fallbackSession.render();
            return;
        }
        if (!this.#canDispatchPlotCommand()) {
            return;
        }
        this.#post("render");
    }
    resetView() {
        if (this.#fallbackSession) {
            this.#fallbackSession.resetView();
            return;
        }
        if (!this.#canDispatchPlotCommand()) {
            return;
        }
        this.#post("resetView");
    }
    pointerDown(x, y, button) {
        if (this.#fallbackSession) {
            this.#fallbackSession.pointerDown(x, y, button);
            return;
        }
        if (!this.#canDispatchPlotCommand()) {
            return;
        }
        this.#post("pointerDown", { x, y, button });
    }
    pointerMove(x, y) {
        if (this.#fallbackSession) {
            this.#fallbackSession.pointerMove(x, y);
            return;
        }
        if (!this.#canDispatchPlotCommand()) {
            return;
        }
        this.#post("pointerMove", { x, y });
    }
    pointerUp(x, y, button) {
        if (this.#fallbackSession) {
            this.#fallbackSession.pointerUp(x, y, button);
            return;
        }
        if (!this.#canDispatchPlotCommand()) {
            return;
        }
        this.#post("pointerUp", { x, y, button });
    }
    pointerLeave() {
        if (this.#fallbackSession) {
            this.#fallbackSession.pointerLeave();
            return;
        }
        if (!this.#canDispatchPlotCommand()) {
            return;
        }
        this.#post("pointerLeave");
    }
    wheel(deltaY, x, y) {
        if (this.#fallbackSession) {
            this.#fallbackSession.wheel(deltaY, x, y);
            return;
        }
        if (!this.#canDispatchPlotCommand()) {
            return;
        }
        this.#post("wheel", { deltaY, x, y });
    }
    async exportPng() {
        if (this.#fallbackSession) {
            return this.#fallbackSession.exportPng();
        }
        if (!this.#canDispatchPlotCommand()) {
            throw new Error("no plot is attached to this worker session");
        }
        const payload = await this.#request("exportPng");
        return payload;
    }
    async exportSvg() {
        if (this.#fallbackSession) {
            return this.#fallbackSession.exportSvg();
        }
        if (!this.#canDispatchPlotCommand()) {
            throw new Error("no plot is attached to this worker session");
        }
        const payload = await this.#request("exportSvg");
        return payload;
    }
    async ready() {
        return this.#readyPromise;
    }
    destroy() {
        if (this.#fallbackSession) {
            this.#fallbackSession.destroy();
            this.#hasPlot = false;
            return;
        }
        this.#sessionEpoch += 1;
        this.#hasPlot = false;
        this.#post("destroy");
    }
    dispose() {
        for (const dispose of this.#cleanup.splice(0)) {
            dispose();
        }
        if (this.#fallbackSession) {
            this.#fallbackSession.dispose();
            this.#fallbackSession = null;
            this.#hasPlot = false;
            return;
        }
        this.#sessionEpoch += 1;
        this.#hasPlot = false;
        this.#worker?.terminate();
        this.#worker = null;
        this.#failAll(new Error("worker session was disposed"));
    }
    _pushCleanup(dispose) {
        this.#cleanup.push(dispose);
    }
    #post(type, payload) {
        this.#worker?.postMessage({ type, payload });
    }
    #request(type, payload) {
        const worker = this.#worker;
        if (!worker) {
            return Promise.reject(new Error("worker session is not active"));
        }
        const requestId = this.#nextRequestId++;
        return new Promise((resolve, reject) => {
            this.#pendingRequests.set(requestId, { resolve, reject });
            worker.postMessage({ type, payload, requestId });
        });
    }
    #handleMessage(event) {
        const { payload, requestId, type } = event.data;
        if (type === "ready") {
            this.#resolveReady?.();
            this.#resolveReady = null;
            this.#rejectReady = null;
            return;
        }
        if (type === "error") {
            const error = new Error(String(payload ?? "worker session failed"));
            if (typeof requestId === "number") {
                const pending = this.#pendingRequests.get(requestId);
                if (pending) {
                    this.#pendingRequests.delete(requestId);
                    pending.reject(error);
                    return;
                }
            }
            this.#failAll(error);
            return;
        }
        if (typeof requestId !== "number") {
            return;
        }
        const pending = this.#pendingRequests.get(requestId);
        if (!pending) {
            return;
        }
        this.#pendingRequests.delete(requestId);
        pending.resolve(payload);
    }
    #failAll(reason) {
        this.#hasPlot = false;
        this.#rejectReady?.(reason);
        this.#resolveReady = null;
        this.#rejectReady = null;
        for (const [requestId, pending] of this.#pendingRequests.entries()) {
            this.#pendingRequests.delete(requestId);
            pending.reject(reason);
        }
    }
}
export function createPlot() {
    return new PlotBuilder();
}
export function createPlotFromSnapshot(snapshot) {
    return PlotBuilder.fromSnapshot(snapshot);
}
export function createObservable(values) {
    return new ObservableSeries(values);
}
export function createSineSignal(options) {
    return new SineSignal(options);
}
export async function registerFont(bytes) {
    const module = await ensureRawModule();
    module.register_font_bytes_js(Uint8Array.from(bytes));
}
export async function getRuntimeCapabilities() {
    const module = await ensureRawModule();
    const capabilities = module.web_runtime_capabilities();
    return {
        offscreenCanvasSupported: readBooleanProperty(capabilities, "offscreen_canvas_supported", "offscreenCanvasSupported"),
        workerSupported: readBooleanProperty(capabilities, "worker_supported", "workerSupported"),
        webgpuSupported: readBooleanProperty(capabilities, "webgpu_supported", "webgpuSupported"),
        touchInputSupported: readBooleanProperty(capabilities, "touch_input_supported", "touchInputSupported"),
        defaultBrowserFontRegistered: readBooleanProperty(capabilities, "default_browser_font_registered", "defaultBrowserFontRegistered"),
        gpuCanvasFastPathAvailable: readBooleanProperty(capabilities, "gpu_canvas_fast_path_available", "gpuCanvasFastPathAvailable"),
    };
}
export async function createCanvasSession(canvas, options) {
    const normalized = normalizeCanvasSessionOptions(options);
    const module = await ensureRawModule();
    const rawSession = new module.WebCanvasSession(canvas);
    const session = new CanvasSession(module, rawSession, canvas);
    session.setBackendPreference(normalized.backendPreference);
    if (typeof normalized.initialTime === "number") {
        session.setTime(normalized.initialTime);
    }
    session.resize();
    if (normalized.autoResize) {
        session._pushCleanup(installCanvasResize(canvas, (width, height, scaleFactor) => {
            session.resize(width, height, scaleFactor);
        }));
    }
    if (normalized.bindInput) {
        session._pushCleanup(attachCanvasEvents(canvas, {
            pointerDown: (x, y, button) => session.pointerDown(x, y, button),
            pointerMove: (x, y) => session.pointerMove(x, y),
            pointerUp: (x, y, button) => session.pointerUp(x, y, button),
            pointerLeave: () => session.pointerLeave(),
            wheel: (deltaY, x, y) => session.wheel(deltaY, x, y),
        }));
    }
    return session;
}
export async function createWorkerSession(canvas, options) {
    const normalized = normalizeWorkerSessionOptions(options);
    if (!workerCanvasSupported(canvas)) {
        if (!normalized.fallbackToMainThread) {
            throw new Error("worker canvas support is unavailable in this browser");
        }
        const fallbackSession = await createCanvasSession(canvas, normalized);
        const session = new WorkerSession(canvas, "main-thread", fallbackSession);
        await session.ready();
        return session;
    }
    const session = new WorkerSession(canvas, "worker");
    const worker = new Worker(new URL("./session-worker.js", import.meta.url), {
        type: "module",
    });
    session.attachWorker(worker);
    const offscreen = canvas.transferControlToOffscreen();
    const initialMetrics = getCanvasMetrics(canvas);
    worker.postMessage({
        type: "init",
        canvas: offscreen,
        width: initialMetrics.width,
        height: initialMetrics.height,
        scaleFactor: initialMetrics.scaleFactor,
        backendPreference: normalized.backendPreference,
        initialTime: normalized.initialTime,
    }, [offscreen]);
    if (normalized.autoResize) {
        session._pushCleanup(installCanvasResize(canvas, (width, height, scaleFactor) => {
            session.resize(width, height, scaleFactor);
        }));
    }
    if (normalized.bindInput) {
        session._pushCleanup(attachCanvasEvents(canvas, {
            pointerDown: (x, y, button) => session.pointerDown(x, y, button),
            pointerMove: (x, y) => session.pointerMove(x, y),
            pointerUp: (x, y, button) => session.pointerUp(x, y, button),
            pointerLeave: () => session.pointerLeave(),
            wheel: (deltaY, x, y) => session.wheel(deltaY, x, y),
        }));
    }
    await session.ready();
    return session;
}
//# sourceMappingURL=index.js.map