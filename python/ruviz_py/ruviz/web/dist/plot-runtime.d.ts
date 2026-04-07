import type { JsPlot as RawJsPlot } from "../generated/raw/ruviz_web_raw.js";
import type { BackendPreference, PlotSnapshot } from "./shared.js";
type RawModule = typeof import("../generated/raw/ruviz_web_raw.js");
export declare function normalizeBackendPreference(backendPreference: BackendPreference | undefined): BackendPreference;
export declare function toRawBackendPreference(module: RawModule, backendPreference: BackendPreference): number;
export declare function applySnapshotMetadata(rawPlot: RawJsPlot, snapshot: PlotSnapshot): void;
export declare function buildRawPlotFromSnapshot(snapshot: PlotSnapshot, module: RawModule): RawJsPlot;
export {};
