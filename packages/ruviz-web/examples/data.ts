export function waveSeries(points = 120, extent = 6): { x: number[]; y: number[] } {
  const x = Array.from({ length: points }, (_, index) => (index / (points - 1)) * extent);
  const y = x.map((value) => Math.sin(value) + 0.35 * Math.cos(value * 2.2));
  return { x, y };
}

export function scatterSeries(points = 48): { x: number[]; y: number[] } {
  const x = Array.from({ length: points }, (_, index) => -2.5 + (5 * index) / (points - 1));
  const y = x.map((value) => 0.4 * value ** 3 - 0.8 * value + Math.sin(value * 3) * 0.35);
  return { x, y };
}

export function categoricalSeries(): { categories: string[]; values: number[] } {
  return {
    categories: ["CPU", "SVG", "GPU", "WASM", "Jupyter"],
    values: [3.8, 2.6, 4.4, 4.9, 4.1],
  };
}

export function sampleDistribution(): number[] {
  const left = Array.from({ length: 72 }, (_, index) => {
    const base = -2.6 + (2.3 * index) / 71;
    return base + Math.sin((5 * index) / 71) * 0.14;
  });
  const middle = Array.from({ length: 72 }, (_, index) => {
    const base = -0.2 + (1.3 * index) / 71;
    return base + Math.cos((8 * index) / 71) * 0.09;
  });
  const right = Array.from({ length: 72 }, (_, index) => {
    const base = 1 + (1.9 * index) / 71;
    return base + Math.sin((6.5 * index) / 71) * 0.12;
  });
  return [...left, ...middle, ...right];
}

export function heatmapValues(rows = 7, cols = 7): number[][] {
  return Array.from({ length: rows }, (_, rowIndex) => {
    const y = -1.2 + (2.4 * rowIndex) / Math.max(1, rows - 1);
    return Array.from({ length: cols }, (_, colIndex) => {
      const x = -1.2 + (2.4 * colIndex) / Math.max(1, cols - 1);
      return Math.sin(y * 2) * Math.cos(x * 2.6) + y * x * 0.4;
    });
  });
}

export function errorBarSeries(): { x: number[]; y: number[]; errors: number[] } {
  const x = Array.from({ length: 7 }, (_, index) => (5 * index) / 6);
  const y = x.map((value) => 1.2 + Math.sin(value) * 0.9);
  const errors = x.map((value) => 0.12 + Math.abs(Math.cos(value)) * 0.18);
  return { x, y, errors };
}

export function errorBarXYSeries(): {
  x: number[];
  y: number[];
  xErrors: number[];
  yErrors: number[];
} {
  const x = Array.from({ length: 6 }, (_, index) => 0.8 + (4.8 * index) / 5);
  const y = x.map((value) => 1 + Math.cos(value * 0.8) * 0.75);
  const xErrors = Array.from({ length: 6 }, (_, index) => 0.1 + (0.06 * index) / 5);
  const yErrors = x.map((value) => 0.15 + Math.abs(Math.sin(value)) * 0.12);
  return { x, y, xErrors, yErrors };
}

export function contourGrid(): { x: number[]; y: number[]; z: number[] } {
  const x = Array.from({ length: 24 }, (_, index) => -2.5 + (5 * index) / 23);
  const y = Array.from({ length: 24 }, (_, index) => -2.5 + (5 * index) / 23);
  const z = y.flatMap((yValue) =>
    x.map((xValue) => {
      const radius = Math.hypot(xValue, yValue);
      return Math.sin(xValue * 1.8) * Math.cos(yValue * 1.5) - radius * 0.08;
    }),
  );
  return { x, y, z };
}

export function radarInputs(): {
  labels: string[];
  series: Array<{ name: string; values: number[] }>;
} {
  return {
    labels: ["API", "Docs", "Export", "Interactive", "Scale"],
    series: [
      { name: "Python", values: [4.5, 4.7, 4.8, 4.3, 4.0] },
      { name: "Web", values: [4.2, 4.1, 4.0, 4.8, 4.6] },
    ],
  };
}

export function polarSeries(points = 120): { r: number[]; theta: number[] } {
  const theta = Array.from({ length: points }, (_, index) => ((Math.PI * 4) * index) / (points - 1));
  const r = theta.map((value) => 0.35 + value * 0.09 + Math.sin(value * 3) * 0.08);
  return { r, theta };
}
