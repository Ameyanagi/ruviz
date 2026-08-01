export type NumericArray = number[] | Float64Array | ArrayLike<number>;
export type BackendPreference = "auto" | "cpu" | "svg" | "gpu";
export type SessionMode = "main-thread" | "worker";
export type PlotTheme = "light" | "dark";
export type PlotSaveFormat = "png" | "svg";
export interface RuntimeCapabilities {
    offscreenCanvasSupported: boolean;
    workerSupported: boolean;
    webgpuSupported: boolean;
    touchInputSupported: boolean;
    defaultBrowserFontRegistered: boolean;
    gpuCanvasFastPathAvailable: boolean;
}
export interface CanvasSessionOptions {
    backendPreference?: BackendPreference;
    autoResize?: boolean;
    bindInput?: boolean;
    initialTime?: number;
}
export interface WorkerSessionOptions extends CanvasSessionOptions {
    fallbackToMainThread?: boolean;
}
export interface PlotSaveOptions {
    fileName?: string;
    format?: PlotSaveFormat;
}
export interface SineSignalOptions {
    points: number;
    domain?: readonly [number, number];
    amplitude?: number;
    cycles?: number;
    phaseVelocity?: number;
    phaseOffset?: number;
    verticalOffset?: number;
}
export interface NormalizedSineSignalOptions {
    points: number;
    domainStart: number;
    domainEnd: number;
    amplitude: number;
    cycles: number;
    phaseVelocity: number;
    phaseOffset: number;
    verticalOffset: number;
}
export interface StaticSourceSnapshot {
    kind: "static";
    values: number[];
}
export interface ObservableSourceSnapshot {
    kind: "observable";
    values: number[];
}
export interface SignalSourceSnapshot {
    kind: "sine-signal";
    options: NormalizedSineSignalOptions;
}
export type NumericReactiveSourceSnapshot = StaticSourceSnapshot | ObservableSourceSnapshot;
export type XSourceSnapshot = NumericReactiveSourceSnapshot;
export type YSourceSnapshot = NumericReactiveSourceSnapshot | SignalSourceSnapshot;
export interface LineSeriesSnapshot {
    kind: "line";
    x: XSourceSnapshot;
    y: YSourceSnapshot;
}
export interface ScatterSeriesSnapshot {
    kind: "scatter";
    x: XSourceSnapshot;
    y: YSourceSnapshot;
}
export interface BarSeriesSnapshot {
    kind: "bar";
    categories: string[];
    values: NumericReactiveSourceSnapshot;
}
export interface HistogramSeriesSnapshot {
    kind: "histogram";
    data: NumericReactiveSourceSnapshot;
}
export interface BoxplotSeriesSnapshot {
    kind: "boxplot";
    data: NumericReactiveSourceSnapshot;
}
export interface HeatmapSeriesSnapshot {
    kind: "heatmap";
    values: number[];
    rows: number;
    cols: number;
}
export interface ErrorBarsSeriesSnapshot {
    kind: "error-bars";
    x: NumericReactiveSourceSnapshot;
    y: NumericReactiveSourceSnapshot;
    yErrors: NumericReactiveSourceSnapshot;
}
export interface ErrorBarsXYSeriesSnapshot {
    kind: "error-bars-xy";
    x: NumericReactiveSourceSnapshot;
    y: NumericReactiveSourceSnapshot;
    xErrors: NumericReactiveSourceSnapshot;
    yErrors: NumericReactiveSourceSnapshot;
}
export interface KdeSeriesSnapshot {
    kind: "kde";
    data: number[];
}
export interface EcdfSeriesSnapshot {
    kind: "ecdf";
    data: number[];
}
export interface ContourSeriesSnapshot {
    kind: "contour";
    x: number[];
    y: number[];
    z: number[];
}
export interface PieSeriesSnapshot {
    kind: "pie";
    values: number[];
    labels?: string[];
}
export interface RadarSeriesItemSnapshot {
    name?: string;
    values: number[];
}
export interface RadarSeriesSnapshot {
    kind: "radar";
    labels: string[];
    series: RadarSeriesItemSnapshot[];
}
export interface ViolinSeriesSnapshot {
    kind: "violin";
    data: number[];
}
export interface PolarLineSeriesSnapshot {
    kind: "polar-line";
    r: number[];
    theta: number[];
}
export type PlotSeriesSnapshot = LineSeriesSnapshot | ScatterSeriesSnapshot | BarSeriesSnapshot | HistogramSeriesSnapshot | BoxplotSeriesSnapshot | HeatmapSeriesSnapshot | ErrorBarsSeriesSnapshot | ErrorBarsXYSeriesSnapshot | KdeSeriesSnapshot | EcdfSeriesSnapshot | ContourSeriesSnapshot | PieSeriesSnapshot | RadarSeriesSnapshot | ViolinSeriesSnapshot | PolarLineSeriesSnapshot;
export interface PlotSnapshot {
    sizePx?: [number, number];
    theme?: PlotTheme;
    ticks?: boolean;
    title?: string;
    xLabel?: string;
    yLabel?: string;
    series: PlotSeriesSnapshot[];
}
export declare function toNumberArray(values: NumericArray): number[];
export declare function normalizeSineSignalOptions(options: SineSignalOptions): NormalizedSineSignalOptions;
export declare function cloneSourceSnapshot<T extends XSourceSnapshot | YSourceSnapshot>(source: T): T;
export declare function clonePlotSnapshot(snapshot: PlotSnapshot): PlotSnapshot;
