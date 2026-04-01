import type { JsPlot as RawJsPlot, ObservableVecF64 as RawObservableVecF64, SignalVecF64 as RawSignalVecF64, WebCanvasSession as RawWebCanvasSession } from "../generated/raw/ruviz_web_raw.js";
import { type BackendPreference, type CanvasSessionOptions, type ContourSeriesSnapshot, type EcdfSeriesSnapshot, type HeatmapSeriesSnapshot, type KdeSeriesSnapshot, type NormalizedSineSignalOptions, type NumericArray, type PieSeriesSnapshot, type PlotSnapshot, type PlotSaveOptions, type PlotTheme, type PolarLineSeriesSnapshot, type RadarSeriesSnapshot, type RuntimeCapabilities, type SessionMode, type SineSignalOptions, type ViolinSeriesSnapshot, type WorkerSessionOptions } from "./shared.js";
export type { BackendPreference, BarSeriesSnapshot, CanvasSessionOptions, BoxplotSeriesSnapshot, ContourSeriesSnapshot, EcdfSeriesSnapshot, ErrorBarsSeriesSnapshot, ErrorBarsXYSeriesSnapshot, HeatmapSeriesSnapshot, HistogramSeriesSnapshot, KdeSeriesSnapshot, NumericArray, PieSeriesSnapshot, PlotTheme, PlotSaveOptions, PlotSnapshot, PlotSeriesSnapshot, PolarLineSeriesSnapshot, RadarSeriesSnapshot, RuntimeCapabilities, SessionMode, SineSignalOptions, StaticSourceSnapshot, ViolinSeriesSnapshot, WorkerSessionOptions, } from "./shared.js";
type RawModule = typeof import("../generated/raw/ruviz_web_raw.js");
type StaticSourceDefinition = {
    kind: "static";
    values: number[];
};
type ObservableSourceDefinition = {
    kind: "observable";
    source: ObservableSeries;
};
type SignalSourceDefinition = {
    kind: "sine-signal";
    source: SineSignal;
};
type NumericReactiveSourceDefinition = StaticSourceDefinition | ObservableSourceDefinition;
type XSourceDefinition = NumericReactiveSourceDefinition;
type YSourceDefinition = NumericReactiveSourceDefinition | SignalSourceDefinition;
interface XYSeriesInput {
    x: NumericArray | ObservableSeries;
    y: NumericArray | ObservableSeries | SineSignal;
}
interface BarSeriesInput {
    categories: readonly string[] | ArrayLike<string>;
    values: NumericArray | ObservableSeries;
}
interface ErrorBarsInput {
    x: NumericArray | ObservableSeries;
    y: NumericArray | ObservableSeries;
    yErrors: NumericArray | ObservableSeries;
}
interface ErrorBarsXYInput {
    x: NumericArray | ObservableSeries;
    y: NumericArray | ObservableSeries;
    xErrors: NumericArray | ObservableSeries;
    yErrors: NumericArray | ObservableSeries;
}
interface RadarSeriesInput {
    name?: string;
    values: NumericArray;
}
interface RadarInput {
    labels: readonly string[] | ArrayLike<string>;
    series: readonly RadarSeriesInput[];
}
interface ContourInput {
    x: NumericArray;
    y: NumericArray;
    z: NumericArray;
}
interface PolarLineInput {
    r: NumericArray;
    theta: NumericArray;
}
interface LineSeriesDefinition {
    kind: "line";
    x: XSourceDefinition;
    y: YSourceDefinition;
}
interface ScatterSeriesDefinition {
    kind: "scatter";
    x: XSourceDefinition;
    y: YSourceDefinition;
}
interface BarSeriesDefinition {
    kind: "bar";
    categories: string[];
    values: NumericReactiveSourceDefinition;
}
interface HistogramSeriesDefinition {
    kind: "histogram";
    data: NumericReactiveSourceDefinition;
}
interface BoxplotSeriesDefinition {
    kind: "boxplot";
    data: NumericReactiveSourceDefinition;
}
interface ErrorBarsSeriesDefinition {
    kind: "error-bars";
    x: NumericReactiveSourceDefinition;
    y: NumericReactiveSourceDefinition;
    yErrors: NumericReactiveSourceDefinition;
}
interface ErrorBarsXYSeriesDefinition {
    kind: "error-bars-xy";
    x: NumericReactiveSourceDefinition;
    y: NumericReactiveSourceDefinition;
    xErrors: NumericReactiveSourceDefinition;
    yErrors: NumericReactiveSourceDefinition;
}
type PlotSeriesDefinition = LineSeriesDefinition | ScatterSeriesDefinition | BarSeriesDefinition | HistogramSeriesDefinition | BoxplotSeriesDefinition | HeatmapSeriesSnapshot | ErrorBarsSeriesDefinition | ErrorBarsXYSeriesDefinition | KdeSeriesSnapshot | EcdfSeriesSnapshot | ContourSeriesSnapshot | PieSeriesSnapshot | RadarSeriesSnapshot | ViolinSeriesSnapshot | PolarLineSeriesSnapshot;
interface PlotState {
    sizePx?: [number, number];
    theme?: PlotTheme;
    ticks?: boolean;
    title?: string;
    xLabel?: string;
    yLabel?: string;
    series: PlotSeriesDefinition[];
}
/** Mutable numeric data source for reactive plot updates. */
export declare class ObservableSeries {
    #private;
    constructor(values: NumericArray);
    get length(): number;
    replace(values: NumericArray): void;
    setAt(index: number, value: number): void;
    values(): Float64Array;
    snapshotValues(): number[];
    _toRawObservable(module?: RawModule): Promise<RawObservableVecF64>;
}
/** Procedural sine-wave signal for temporal playback in interactive sessions. */
export declare class SineSignal {
    #private;
    readonly options: NormalizedSineSignalOptions;
    constructor(options: SineSignalOptions);
    get length(): number;
    valuesAt(timeSeconds: number): Float64Array;
    _toRawSignal(module?: RawModule): Promise<RawSignalVecF64>;
}
/** Fluent plot builder for static export and interactive canvas mounting. */
export declare class PlotBuilder {
    #private;
    constructor(state?: PlotState);
    static fromSnapshot(snapshot: PlotSnapshot): PlotBuilder;
    sizePx(width: number, height: number): this;
    setSizePx(width: number, height: number): this;
    theme(theme: PlotTheme): this;
    setTheme(theme: PlotTheme): this;
    ticks(enabled: boolean): this;
    setTicks(enabled: boolean): this;
    title(title: string): this;
    setTitle(title: string): this;
    xlabel(label: string): this;
    setXLabel(label: string): this;
    ylabel(label: string): this;
    setYLabel(label: string): this;
    line(input: XYSeriesInput): this;
    addLine(input: XYSeriesInput): this;
    scatter(input: XYSeriesInput): this;
    addScatter(input: XYSeriesInput): this;
    bar(input: BarSeriesInput): this;
    histogram(input: NumericArray | ObservableSeries): this;
    boxplot(input: NumericArray | ObservableSeries): this;
    heatmap(input: ReadonlyArray<NumericArray>): this;
    errorBars(input: ErrorBarsInput): this;
    errorBarsXY(input: ErrorBarsXYInput): this;
    kde(input: NumericArray): this;
    ecdf(input: NumericArray): this;
    contour(input: ContourInput): this;
    pie(values: NumericArray, labelsInput?: readonly string[] | ArrayLike<string>): this;
    radar(input: RadarInput): this;
    violin(input: NumericArray): this;
    polarLine(input: PolarLineInput): this;
    clone(): PlotBuilder;
    toSnapshot(): PlotSnapshot;
    renderPng(): Promise<Uint8Array>;
    renderSvg(): Promise<string>;
    save(options?: PlotSaveOptions): Promise<void>;
    mount(canvas: HTMLCanvasElement, options?: CanvasSessionOptions): Promise<CanvasSession>;
    mountWorker(canvas: HTMLCanvasElement, options?: WorkerSessionOptions): Promise<WorkerSession>;
    _toRawPlot(module?: RawModule): Promise<RawJsPlot>;
}
/** Main-thread interactive canvas session. */
export declare class CanvasSession {
    #private;
    readonly mode: SessionMode;
    constructor(module: RawModule, rawSession: RawWebCanvasSession, canvas: HTMLCanvasElement);
    hasPlot(): boolean;
    setPlot(plot: PlotBuilder): Promise<void>;
    resize(width?: number, height?: number, scaleFactor?: number): void;
    setTime(timeSeconds: number): void;
    setBackendPreference(backendPreference: BackendPreference): void;
    render(): void;
    resetView(): void;
    pointerDown(x: number, y: number, button: number): void;
    pointerMove(x: number, y: number): void;
    pointerUp(x: number, y: number, button: number): void;
    pointerLeave(): void;
    wheel(deltaY: number, x: number, y: number): void;
    exportPng(): Promise<Uint8Array>;
    exportSvg(): Promise<string>;
    destroy(): void;
    dispose(): void;
    _pushCleanup(dispose: () => void): void;
}
/** Worker-backed interactive canvas session with main-thread fallback support. */
export declare class WorkerSession {
    #private;
    readonly mode: SessionMode;
    constructor(canvas: HTMLCanvasElement, mode: SessionMode, fallbackSession?: CanvasSession);
    get canvas(): HTMLCanvasElement;
    attachWorker(worker: Worker): void;
    hasPlot(): boolean;
    setPlot(plot: PlotBuilder): Promise<void>;
    resize(width?: number, height?: number, scaleFactor?: number): void;
    setTime(timeSeconds: number): void;
    setBackendPreference(backendPreference: BackendPreference): void;
    render(): void;
    resetView(): void;
    pointerDown(x: number, y: number, button: number): void;
    pointerMove(x: number, y: number): void;
    pointerUp(x: number, y: number, button: number): void;
    pointerLeave(): void;
    wheel(deltaY: number, x: number, y: number): void;
    exportPng(): Promise<Uint8Array>;
    exportSvg(): Promise<string>;
    ready(): Promise<void>;
    destroy(): void;
    dispose(): void;
    _pushCleanup(dispose: () => void): void;
}
/** Create a new fluent plot builder. */
export declare function createPlot(): PlotBuilder;
/** Rehydrate a plot builder from a serialized snapshot. */
export declare function createPlotFromSnapshot(snapshot: PlotSnapshot): PlotBuilder;
/** Create a mutable observable numeric series. */
export declare function createObservable(values: NumericArray): ObservableSeries;
/** Create a time-varying sine signal. */
export declare function createSineSignal(options: SineSignalOptions): SineSignal;
export declare function registerFont(bytes: Uint8Array | ArrayLike<number>): Promise<void>;
/** Inspect browser runtime capabilities used by the interactive renderer. */
export declare function getRuntimeCapabilities(): Promise<RuntimeCapabilities>;
export declare function createCanvasSession(canvas: HTMLCanvasElement, options?: CanvasSessionOptions): Promise<CanvasSession>;
export declare function createWorkerSession(canvas: HTMLCanvasElement, options?: WorkerSessionOptions): Promise<WorkerSession>;
