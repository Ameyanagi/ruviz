export type NumericArray = number[] | Float64Array | ArrayLike<number>;
export type BackendPreference = "auto" | "cpu" | "svg" | "gpu";
export type SessionMode = "main-thread" | "worker";
export type PlotTheme = "light" | "dark";

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

export type XSourceSnapshot = StaticSourceSnapshot | ObservableSourceSnapshot;
export type YSourceSnapshot =
  | StaticSourceSnapshot
  | ObservableSourceSnapshot
  | SignalSourceSnapshot;

export interface PlotSeriesSnapshot {
  kind: "line" | "scatter";
  x: XSourceSnapshot;
  y: YSourceSnapshot;
}

export interface PlotSnapshot {
  sizePx?: [number, number];
  theme?: PlotTheme;
  ticks?: boolean;
  title?: string;
  xLabel?: string;
  yLabel?: string;
  series: PlotSeriesSnapshot[];
}

export function toNumberArray(values: NumericArray): number[] {
  return Array.from(values, (value) => Number(value));
}

function finiteNumber(value: number | undefined, fallback: number): number {
  return Number.isFinite(value) ? Number(value) : fallback;
}

export function normalizeSineSignalOptions(
  options: SineSignalOptions,
): NormalizedSineSignalOptions {
  const points = Math.max(2, Math.floor(finiteNumber(options.points, 2)));
  const domainStart = finiteNumber(options.domain?.[0], 0);
  const defaultDomainEnd = domainStart + Math.PI * 2;
  const rawDomainEnd = finiteNumber(options.domain?.[1], defaultDomainEnd);
  const domainEnd = rawDomainEnd === domainStart ? defaultDomainEnd : rawDomainEnd;

  return {
    points,
    domainStart,
    domainEnd,
    amplitude: finiteNumber(options.amplitude, 1),
    cycles: finiteNumber(options.cycles, 1),
    phaseVelocity: finiteNumber(options.phaseVelocity, 0),
    phaseOffset: finiteNumber(options.phaseOffset, 0),
    verticalOffset: finiteNumber(options.verticalOffset, 0),
  };
}

export function cloneSourceSnapshot(
  source: XSourceSnapshot | YSourceSnapshot,
): XSourceSnapshot | YSourceSnapshot {
  if (source.kind === "sine-signal") {
    return {
      kind: "sine-signal",
      options: { ...source.options },
    };
  }

  return {
    kind: source.kind,
    values: [...source.values],
  };
}

export function clonePlotSnapshot(snapshot: PlotSnapshot): PlotSnapshot {
  return {
    sizePx: snapshot.sizePx ? ([...snapshot.sizePx] as [number, number]) : undefined,
    theme: snapshot.theme,
    ticks: snapshot.ticks,
    title: snapshot.title,
    xLabel: snapshot.xLabel,
    yLabel: snapshot.yLabel,
    series: snapshot.series.map((series) => ({
      kind: series.kind,
      x: cloneSourceSnapshot(series.x) as XSourceSnapshot,
      y: cloneSourceSnapshot(series.y) as YSourceSnapshot,
    })),
  };
}
