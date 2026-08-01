export function toNumberArray(values) {
    return Array.from(values, (value) => Number(value));
}
function finiteNumber(value, fallback) {
    return Number.isFinite(value) ? Number(value) : fallback;
}
export function normalizeSineSignalOptions(options) {
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
export function cloneSourceSnapshot(source) {
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
function cloneSeriesSnapshot(series) {
    switch (series.kind) {
        case "line":
        case "scatter":
            return {
                kind: series.kind,
                x: cloneSourceSnapshot(series.x),
                y: cloneSourceSnapshot(series.y),
            };
        case "bar":
            return {
                kind: "bar",
                categories: [...series.categories],
                values: cloneSourceSnapshot(series.values),
            };
        case "histogram":
            return {
                kind: "histogram",
                data: cloneSourceSnapshot(series.data),
            };
        case "boxplot":
            return {
                kind: "boxplot",
                data: cloneSourceSnapshot(series.data),
            };
        case "heatmap":
            return {
                kind: "heatmap",
                values: [...series.values],
                rows: series.rows,
                cols: series.cols,
            };
        case "error-bars":
            return {
                kind: "error-bars",
                x: cloneSourceSnapshot(series.x),
                y: cloneSourceSnapshot(series.y),
                yErrors: cloneSourceSnapshot(series.yErrors),
            };
        case "error-bars-xy":
            return {
                kind: "error-bars-xy",
                x: cloneSourceSnapshot(series.x),
                y: cloneSourceSnapshot(series.y),
                xErrors: cloneSourceSnapshot(series.xErrors),
                yErrors: cloneSourceSnapshot(series.yErrors),
            };
        case "kde":
            return { kind: "kde", data: [...series.data] };
        case "ecdf":
            return { kind: "ecdf", data: [...series.data] };
        case "contour":
            return {
                kind: "contour",
                x: [...series.x],
                y: [...series.y],
                z: [...series.z],
            };
        case "pie":
            return {
                kind: "pie",
                values: [...series.values],
                labels: series.labels ? [...series.labels] : undefined,
            };
        case "radar":
            return {
                kind: "radar",
                labels: [...series.labels],
                series: series.series.map((item) => ({
                    name: item.name,
                    values: [...item.values],
                })),
            };
        case "violin":
            return { kind: "violin", data: [...series.data] };
        case "polar-line":
            return {
                kind: "polar-line",
                r: [...series.r],
                theta: [...series.theta],
            };
    }
}
export function clonePlotSnapshot(snapshot) {
    return {
        sizePx: snapshot.sizePx ? [...snapshot.sizePx] : undefined,
        theme: snapshot.theme,
        ticks: snapshot.ticks,
        title: snapshot.title,
        xLabel: snapshot.xLabel,
        yLabel: snapshot.yLabel,
        series: snapshot.series.map(cloneSeriesSnapshot),
    };
}
//# sourceMappingURL=shared.js.map