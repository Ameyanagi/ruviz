# Scatter Benchmarks

Static scatter rendering compared against [reflex-dev/xy](https://github.com/reflex-dev/xy)
and matplotlib, using XY's own launch-benchmark contract so the numbers are
directly comparable with theirs: the same seeded correlated gaussian float32
data, a validated non-blank 900×420 PNG, and `render_ms` covering chart build
plus PNG encode — imports and data generation excluded. Cold rows run each
sample in a fresh process (mean of 3, guarded by a memory cap and timeout);
warm rows render fresh charts inside a warmed process (minimum of 3).

Benchmark results are environment-scoped; never merge rows from different
machines into one table.

## Results — 2026-08-15, Apple M4 (10 cores, 32 GiB), macOS 26.5.1

Versions: ruviz `perf/render-hot-paths` (post-0.8.0, release build),
xy 0.0.7a1 (built from `main`), matplotlib 3.11.1 (Agg), Python 3.12.

### Cold: fresh process per run, mean of 3 (ms)

| points | xy | ruviz fast | ruviz exact | matplotlib |
|---|---|---|---|---|
| 10,000 | 7.5 | 70 | 60 | 46 |
| 100,000 | 14.9 | 76 | 82 | 92 |
| 1,000,000 | 14.8 | 41.7 | 220 | 604 |
| 10,000,000 | 42 | 127 | 2,089 | 5,597 |

### Warm process: fresh chart per render, minimum of 3 (ms)

| points | xy | ruviz fast | ruviz exact | matplotlib |
|---|---|---|---|---|
| 10,000 | 3.7 | 8.8 | 7.6 | 30.7 |
| 100,000 | 10.3 | 23 | 19.7 | 73.1 |
| 1,000,000 | 6.5 | 16.7 | 149.8 | 559.3 |
| 10,000,000 | 21.6 | 71 | 1,988.7 | 5,343.8 |

Reading the table:

- ruviz fast and xy converge above 1M points because both aggregate to a
  density surface there — work scales with plot pixels, not points. XY
  switches automatically above its 2M-point soft ceiling; ruviz switches
  under `fast()` past one point per plot pixel, and never by default.
- Below the threshold, ruviz fast renders the exact marker output,
  byte-identical to exact mode.
- The cold-row gap between ruviz and xy at 10k–100k is dominated by
  first-render costs (marker sprite population; the system font list is
  already served from a disk cache) rather than per-point rendering — the
  warm rows show the marginal costs.
- Density timing is marker-shape independent within ~15%: at 10M points,
  circle 60 / square 63 / triangle 68 / plus 70 / cross 70 ms warm.

## Reproducing

```bash
# A venv holding the competitors (xy needs its Rust core; see the xy repo):
uv venv xy-venv --python 3.12
uv pip install -p xy-venv/bin/python xy matplotlib numpy pillow psutil

# ruviz's own venv (bindings/python/.venv) with a RELEASE build:
cd bindings/python && uv run maturin develop --release && cd ../..

# Cold suite — writes JSON next to the given path. Run once and discard to
# warm OS caches, then run again for numbers:
XY_PYTHON=xy-venv/bin/python \
RUVIZ_PYTHON=bindings/python/.venv/bin/python \
bindings/python/.venv/bin/python scripts/bench_scatter_vs.py --out cold.json

# Warm arm, one process per library:
for lib in xy ruviz ruviz-fast matplotlib; do
  case "$lib" in xy|matplotlib) py=xy-venv/bin/python ;; *) py=bindings/python/.venv/bin/python ;; esac
  "$py" scripts/bench_scatter_warm.py "$lib" 10000,100000,1000000,10000000
done
```

Both scripts pin the seed and validate every PNG's dimensions and
non-blankness, so a silently wrong output fails instead of producing a fast
number.

## Related

- [Performance guide](guide/08_performance.md) — density scatters, fast mode,
  and the marker footprint capability table.
