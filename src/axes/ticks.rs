//! Tick generation and tick label formatting for axis labeling
//!
//! This module owns the **single** tick generator and the **single** tick label
//! formatter used by every backend. Raster, SVG and the layout measurement pass
//! all call [`generate_ticks_for_scale`] and [`format_tick_labels_for_scale`],
//! so an axis can never be ticked or labelled one way in PNG and another in SVG.
//!
//! Tick *positions* come from a candidate-scoring search (see
//! [`generate_ticks`]); tick *labels* come from a single shared
//! [`TickFormatter`], which picks precision per axis rather than per value so a
//! single axis can never mix plain and scientific notation.

use super::AxisScale;
use crate::core::TickFormatter;

/// Generate tick positions using matplotlib's MaxNLocator algorithm
///
/// Produces ticks at "nice" numbers (1, 2, 2.5, 5 multiples of a power of ten).
/// Rather than rounding the rough step up a fixed ladder, several candidate
/// steps are generated and scored on how many ticks actually land inside the
/// data range, how round the resulting values are, and how much of the range
/// they span. The best-scoring candidate wins, which is what keeps short axes
/// (colorbars in particular) from degenerating to two ticks.
///
/// `target_count` is a **hard ceiling**, not a bullseye — it is
/// `MaxNLocator(nbins = target_count - 1)`, and matplotlib's `nbins` is a
/// maximum. A candidate that emits more ticks than the ceiling is not eligible
/// at all, however round its step is; scoring only chooses among the candidates
/// that fit, where it prefers the densest. That ordering matters, because the
/// rungs of the ladder are roughly a factor of two apart: if the ceiling merely
/// *bent*, the next rung down would double the tick count for a relative
/// overshoot small enough to be bought back by a rounder mantissa, and every
/// axis would come out twice as dense as matplotlib's.
///
/// The ceiling drops further when the candidate's own labels are long, because
/// label width, not tick count, is what actually makes an axis illegible.
///
/// # Arguments
/// * `min` - Minimum data value
/// * `max` - Maximum data value
/// * `target_count` - Maximum number of ticks (clamped to 3-10)
///
/// # Returns
/// Vector of tick positions, all within `[min, max]`
pub fn generate_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if target_count == 0 || (max - min).abs() < f64::EPSILON {
        return vec![min, max];
    }

    let (min, max) = if min <= max { (min, max) } else { (max, min) };

    let max_ticks = target_count.clamp(3, 10);
    generate_nice_ticks(min, max, max_ticks)
}

/// Generate minor ticks between major tick positions
pub fn generate_minor_ticks(major_ticks: &[f64], count: usize) -> Vec<f64> {
    if major_ticks.len() < 2 || count == 0 {
        return Vec::new();
    }

    let mut minor_ticks = Vec::new();
    for window in major_ticks.windows(2) {
        let start = window[0];
        let end = window[1];
        let step = (end - start) / (count + 1) as f64;

        for i in 1..=count {
            minor_ticks.push(start + step * i as f64);
        }
    }
    minor_ticks
}

/// Generate tick positions for a specific axis scale type
///
/// Automatically selects the appropriate tick generation algorithm based on the scale.
///
/// # Arguments
/// * `min` - Minimum data value
/// * `max` - Maximum data value
/// * `target_count` - Target number of ticks
/// * `scale` - The axis scale type
///
/// # Returns
/// Vector of tick positions in data coordinates
pub fn generate_ticks_for_scale(
    min: f64,
    max: f64,
    target_count: usize,
    scale: &AxisScale,
) -> Vec<f64> {
    match scale {
        AxisScale::Linear => generate_ticks(min, max, target_count),
        AxisScale::Log => generate_log_ticks(min, max, target_count),
        AxisScale::SymLog { linthresh } => {
            generate_symlog_ticks(min, max, *linthresh, target_count)
        }
    }
}

/// Generate logarithmic tick positions at powers of 10
///
/// # Arguments
/// * `min` - Minimum data value (must be > 0)
/// * `max` - Maximum data value (must be > 0)
/// * `target_count` - Target number of ticks
///
/// # Returns
/// Vector of tick positions at powers of 10 (1, 10, 100, 1000, ...)
pub fn generate_log_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    let (min, max) = if min <= max { (min, max) } else { (max, min) };

    if min <= 0.0 || max <= 0.0 || !min.is_finite() || !max.is_finite() {
        let mut fallback = vec![min, max];
        fallback.retain(|tick| tick.is_finite() && *tick > 0.0);
        fallback.dedup_by(|left, right| left == right);
        return fallback;
    }
    if target_count == 0 || min == max {
        return vec![min, max];
    }

    let log_min = min.log10().floor() as i32;
    let log_max = max.log10().ceil() as i32;
    let decades = (log_max - log_min) as usize;

    let mut ticks = Vec::new();

    if decades <= target_count {
        // Few decades: include all powers of 10
        for exp in log_min..=log_max {
            let tick = 10.0_f64.powi(exp);
            if tick >= min && tick <= max {
                ticks.push(tick);
            }
        }

        // Decades alone are the labelled ticks: an axis that also labelled the
        // 2/5 intermediates read in two formats at once ("10⁻¹" next to "0.5").
        // Only a range too short to produce three decades keeps them, because
        // there the alternative is an axis with one or two ticks on it — and
        // the label formatter puts *those* ticks in the same notation as the
        // decades beside them.
        if ticks.len() < 3 {
            for exp in log_min..log_max {
                let base = 10.0_f64.powi(exp);
                for &mult in &[2.0, 5.0] {
                    let tick = base * mult;
                    if tick >= min && tick <= max {
                        ticks.push(tick);
                    }
                }
            }
        }
    } else {
        // Many decades: show every Nth decade
        let step = ((decades as f64) / (target_count as f64)).ceil() as i32;
        let start_exp = log_min;
        let mut exp = start_exp;
        while exp <= log_max {
            let tick = 10.0_f64.powi(exp);
            if tick >= min && tick <= max {
                ticks.push(tick);
            }
            exp += step;
        }
    }

    if ticks.len() < 2 {
        ticks.extend([min, max]);
    }
    ticks.sort_by(f64::total_cmp);
    ticks.dedup_by(|left, right| left == right);
    ticks
}

/// Generate symmetric logarithmic tick positions
///
/// Creates ticks that work well with symlog scale: linear ticks near zero,
/// logarithmic ticks outside the linear threshold.
///
/// # Arguments
/// * `min` - Minimum data value
/// * `max` - Maximum data value
/// * `linthresh` - Linear threshold
/// * `target_count` - Target number of ticks
pub fn generate_symlog_ticks(min: f64, max: f64, linthresh: f64, target_count: usize) -> Vec<f64> {
    if target_count == 0 || (max - min).abs() < f64::EPSILON {
        return vec![min, max];
    }

    let (min, max) = if min <= max { (min, max) } else { (max, min) };

    let mut ticks = Vec::new();

    // Linear region ticks: within ±linthresh
    let lin_min = min.max(-linthresh);
    let lin_max = max.min(linthresh);
    if lin_min < lin_max {
        // Generate a few linear ticks including 0
        if min < 0.0 && max > 0.0 {
            ticks.push(0.0);
        }
        if lin_min < 0.0 {
            ticks.push(lin_min);
        }
        if lin_max > 0.0 {
            ticks.push(lin_max);
        }
    }

    // Positive logarithmic region
    if max > linthresh {
        let log_ticks = generate_log_ticks(linthresh, max, target_count / 2);
        for tick in log_ticks {
            if tick > linthresh && tick <= max {
                ticks.push(tick);
            }
        }
    }

    // Negative logarithmic region
    if min < -linthresh {
        let log_ticks = generate_log_ticks(linthresh, -min, target_count / 2);
        for tick in log_ticks {
            let neg_tick = -tick;
            if neg_tick < -linthresh && neg_tick >= min {
                ticks.push(neg_tick);
            }
        }
    }

    ticks.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ticks.dedup_by(|a, b| relative_ticks_overlap(*a, *b));
    ticks
}

fn relative_ticks_overlap(left: f64, right: f64) -> bool {
    left == right || (left - right).abs() <= left.abs().max(right.abs()) * f64::EPSILON * 8.0
}

/// Generate minor ticks for logarithmic scales
///
/// Creates ticks at 2, 3, 4, 5, 6, 7, 8, 9 × 10^n between major ticks
pub fn generate_log_minor_ticks(major_ticks: &[f64]) -> Vec<f64> {
    if major_ticks.len() < 2 {
        return Vec::new();
    }

    let mut minor_ticks = Vec::new();
    for window in major_ticks.windows(2) {
        let start = window[0];
        let end = window[1];

        // Check if these are consecutive decades
        if (end / start - 10.0).abs() < 0.01 {
            // Add minor ticks at 2, 3, 4, 5, 6, 7, 8, 9
            for mult in 2..=9 {
                let tick = start * mult as f64;
                if tick > start && tick < end {
                    minor_ticks.push(tick);
                }
            }
        }
    }
    minor_ticks
}

/// Safety cap on the number of steps walked between the nice bounds.
///
/// Shared with `core::tick_formatter` (rather than mirrored there) so both
/// emission loops degrade identically on pathological ranges. Nice-number
/// selection keeps the real count well under this, so it is only ever reached
/// by degenerate input.
pub(crate) const MAX_TICK_STEPS: usize = 100;

/// Candidate step mantissas, each paired with a "niceness" weight.
///
/// Same ladder matplotlib's `MaxNLocator` uses. `10` is intentionally absent:
/// the exponent sweep in [`best_step_and_ticks`] already covers it as `1` one
/// decade up. `2.5` is worth having (it is what turns a 4-tick axis into a
/// 5-tick one) but scores lowest, so it only wins when it is clearly better.
const STEP_LADDER: [(f64, f64); 4] = [(1.0, 1.0), (2.0, 0.9), (2.5, 0.45), (5.0, 0.8)];

/// How many decades either side of the rough step to consider.
///
/// The rough step is a lower bound on a sensible step, so the sweep leans
/// upward: one decade down is enough to recover a denser axis, two up are
/// needed when nothing near the rough step can be represented (huge magnitudes
/// where the step falls below one ULP).
const STEP_EXPONENT_BELOW: i32 = 1;
const STEP_EXPONENT_ABOVE: i32 = 2;

/// Scoring weights. They sum to 1, so a perfect candidate scores 1.0.
const WEIGHT_DENSITY: f64 = 0.55;
const WEIGHT_SIMPLICITY: f64 = 0.30;
const WEIGHT_COVERAGE: f64 = 0.15;

/// How steeply candidates that overflow the ceiling are ranked against each
/// other.
///
/// This is *not* a soft ceiling: [`best_step_and_ticks`] prefers any candidate
/// that fits over every candidate that does not, so this penalty is only
/// consulted when nothing fits at all — a range so awkward that even the
/// coarsest rung within the exponent sweep overshoots. It then orders that bad
/// set least-overflowing-first, quadratically in the relative excess, so the
/// fallback axis is the most legible of the available failures.
///
/// Any value above zero preserves that ordering; the magnitude only decides how
/// much roundness may be traded for an extra tick *among already-overflowing*
/// candidates, and 3.0 keeps that trade small.
const OVERSHOOT_PENALTY: f64 = 3.0;

/// Roughly how many label characters fit side by side along one axis.
///
/// Tick *positions* are chosen before anything has measured a glyph, so the
/// only handle on label width available here is how many characters the labels
/// take. The budget is calibrated on the default figure: a ~640px plot area at
/// the default ~10px label font fits about 90 characters, and labels have to be
/// separated by roughly their own width again to read as separate blobs, which
/// leaves about 60 characters of actual label ink.
///
/// This is deliberately a coarse, font-independent rule. Doing it properly
/// needs real text metrics, which this module cannot obtain — see the note on
/// [`tick_ceiling`].
const AXIS_LABEL_BUDGET_CHARS: f64 = 60.0;

/// Blank characters kept between two adjacent labels, so they read as two.
const LABEL_GAP_CHARS: f64 = 2.0;

/// The shared formatter's plain-decimal cap (`TickFormatter::default().max_decimals`).
///
/// Beyond it the formatter switches to scientific notation, whose labels are
/// short and bounded however small the step is.
const MAX_PLAIN_DECIMALS: i32 = 6;

/// Characters in a scientific label such as `2×10⁻⁹`, excluding any sign.
const SCIENTIFIC_LABEL_CHARS: usize = 6;

/// Internal function implementing nice number selection
///
/// Degenerate ranges collapse to a single tick; everything else that scoring
/// cannot handle (non-finite range, or a magnitude where no step is
/// representable) falls back to the endpoints.
fn generate_nice_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if max - min <= 0.0 {
        return vec![min];
    }

    best_step_and_ticks(min, max, target_count)
        .map(|(_, ticks)| ticks)
        .unwrap_or_else(|| vec![min, max])
}

/// Pick the best-scoring nice step for `[min, max]`, plus the ticks it emits.
///
/// Returns `None` when no candidate is viable, which happens only for
/// degenerate ranges (non-finite, empty, or so large in magnitude that every
/// step falls below one ULP of the axis start).
pub(crate) fn select_nice_step(min: f64, max: f64, target_count: usize) -> Option<f64> {
    best_step_and_ticks(min, max, target_count).map(|(step, _)| step)
}

fn best_step_and_ticks(min: f64, max: f64, target_count: usize) -> Option<(f64, Vec<f64>)> {
    let range = max - min;
    if !range.is_finite() || range <= 0.0 {
        return None;
    }

    let target = target_count.max(2) as f64;
    let rough_step = range / (target - 1.0);
    if !rough_step.is_finite() || rough_step <= f64::EPSILON {
        return None;
    }

    let base_exponent = rough_step.log10().floor();
    if !base_exponent.is_finite() {
        return None;
    }
    let base_exponent = base_exponent as i32;

    // Candidates that respect the ceiling and candidates that blow through it
    // are ranked separately, and a fitting candidate always beats an
    // overflowing one however well the latter scores. Scoring only ever
    // chooses *among* the axes that are legible, it can never buy legibility
    // back with roundness.
    let mut best_fitting: Option<(f64, f64, Vec<f64>)> = None;
    let mut best_overflowing: Option<(f64, f64, Vec<f64>)> = None;
    for exponent in (base_exponent - STEP_EXPONENT_BELOW)..=(base_exponent + STEP_EXPONENT_ABOVE) {
        let magnitude = 10.0_f64.powi(exponent);
        if !magnitude.is_finite() || magnitude <= 0.0 {
            continue;
        }

        for (mantissa, simplicity) in STEP_LADDER {
            let step = mantissa * magnitude;
            if !step.is_finite() || step <= 0.0 {
                continue;
            }
            let Some(ticks) = ticks_for_step(min, max, step) else {
                continue;
            };
            // A single tick is not an axis; reject rather than score it.
            if ticks.len() < 2 {
                continue;
            }

            let ceiling = tick_ceiling(target, &ticks, mantissa, exponent);
            let score = score_candidate(&ticks, range, ceiling, simplicity);
            let bucket = if ticks.len() as f64 <= ceiling {
                &mut best_fitting
            } else {
                &mut best_overflowing
            };
            if bucket
                .as_ref()
                .is_none_or(|(best_score, _, _)| score > *best_score)
            {
                *bucket = Some((score, step, ticks));
            }
        }
    }

    best_fitting
        .or(best_overflowing)
        .map(|(_, step, ticks)| (step, ticks))
}

/// Score a candidate on density, niceness and coverage.
///
/// * density  - how close the emitted tick count sits *under* the ceiling
/// * simplicity - how round the step mantissa is
/// * coverage - how much of the data range the ticks actually span
///
/// Density rewards filling the ceiling: a candidate that emits exactly
/// `ceiling` ticks scores 1.0 and one that emits half that scores 0.0. That is
/// what keeps a short axis (a colorbar in particular) from settling for two
/// ticks when a finer rung of the ladder would also have fitted.
///
/// The overshoot branch is only ever reached when *nothing* fits the ceiling
/// (see [`best_step_and_ticks`], which prefers any fitting candidate over every
/// overflowing one). It exists solely to rank the overflowing candidates
/// sensibly against each other — least-overflowing first — so the fallback is
/// still the most legible of a bad set.
fn score_candidate(ticks: &[f64], range: f64, ceiling: f64, simplicity: f64) -> f64 {
    let count = ticks.len() as f64;
    let density = if count <= ceiling {
        2.0 - ceiling / count
    } else {
        let excess = count / ceiling - 1.0;
        1.0 - OVERSHOOT_PENALTY * excess * excess
    };

    let span = ticks[ticks.len() - 1] - ticks[0];
    let coverage = if range > 0.0 && span.is_finite() {
        (span / range).clamp(0.0, 1.0)
    } else {
        0.0
    };

    WEIGHT_DENSITY * density + WEIGHT_SIMPLICITY * simplicity + WEIGHT_COVERAGE * coverage
}

/// The most ticks this candidate may emit: the caller's target, lowered when
/// the candidate's own labels are wide.
///
/// Label *width* is the real constraint on how many ticks an axis can carry:
/// "25000000" and "-10000000" collide at counts that "0.4" tolerates. Tick
/// selection runs before any glyph has been measured, so the width is
/// approximated by the character count of the labels the candidate would
/// produce, and the ceiling becomes "how many of those fit in
/// [`AXIS_LABEL_BUDGET_CHARS`], separated by [`LABEL_GAP_CHARS`]". The result
/// only ever *lowers* the caller's target, and never below two.
///
/// This is a font-independent approximation on purpose. Measuring properly
/// would need `render.rs` to pass in the axis length in pixels and the widest
/// label advance width for the resolved font and DPI — neither of which exists
/// at this point in the pipeline — so the budget is stated in characters and
/// calibrated on the default figure rather than guessed at in ems here.
fn tick_ceiling(target: f64, ticks: &[f64], mantissa: f64, exponent: i32) -> f64 {
    let chars = estimated_label_chars(ticks, mantissa, exponent) as f64;
    let allowed = (AXIS_LABEL_BUDGET_CHARS / (chars + LABEL_GAP_CHARS)).floor();
    target.min(allowed).max(2.0)
}

/// How many characters the widest label of this candidate takes.
///
/// Mirrors what the shared [`TickFormatter`] will actually print: one shared
/// precision per axis, driven by the step, and a switch to scientific notation
/// once plain decimals run past [`MAX_PLAIN_DECIMALS`].
fn estimated_label_chars(ticks: &[f64], mantissa: f64, exponent: i32) -> usize {
    let sign = usize::from(ticks.iter().any(|tick| *tick < 0.0));
    let decimals = step_decimals(mantissa, exponent);
    if decimals > MAX_PLAIN_DECIMALS {
        return SCIENTIFIC_LABEL_CHARS + sign;
    }

    let max_abs = ticks
        .iter()
        .fold(0.0_f64, |widest, tick| widest.max(tick.abs()));
    let integer_digits = if max_abs >= 1.0 && max_abs.is_finite() {
        max_abs.log10().floor() as usize + 1
    } else {
        1
    };
    let decimals = decimals as usize;

    sign + integer_digits + if decimals > 0 { decimals + 1 } else { 0 }
}

/// Decimal places a step of `mantissa × 10^exponent` needs to print exactly.
///
/// Every mantissa on [`STEP_LADDER`] is an integer except `2.5`, which costs
/// one place more than the bare power of ten.
fn step_decimals(mantissa: f64, exponent: i32) -> i32 {
    let fractional = i32::from((mantissa - mantissa.round()).abs() > 1e-9);
    (fractional - exponent).max(0)
}

/// Emit every multiple of `step` that lies inside `[min, max]`.
///
/// Returns `None` when the step cannot produce a bounded, terminating walk.
/// Both guards matter and must not be removed:
///
/// * `start + step <= start` catches steps smaller than one ULP of the axis
///   start (e.g. `min = 1e16` with `step = 0.5`), where an accumulating walk
///   would never advance.
/// * the [`MAX_TICK_STEPS`] cap bounds the walk for pathological ranges.
///
/// The loop itself is indexed by an integer, so it terminates by construction
/// and does not accumulate rounding drift.
fn ticks_for_step(min: f64, max: f64, step: f64) -> Option<Vec<f64>> {
    let first_index = (min / step).floor();
    let last_index = (max / step).ceil();
    if !first_index.is_finite() || !last_index.is_finite() {
        return None;
    }

    let start = first_index * step;
    if !start.is_finite() || start + step <= start {
        return None;
    }

    let steps = last_index - first_index;
    if !steps.is_finite() || steps < 0.0 || steps > MAX_TICK_STEPS as f64 {
        return None;
    }
    let steps = steps.round() as usize;

    let mut ticks = Vec::with_capacity(steps + 1);
    let epsilon = step * 1e-10;

    for i in 0..=steps {
        let tick = start + (i as f64) * step;
        // `max + epsilon` can itself overflow to infinity near `f64::MAX`, so
        // the finiteness check has to be explicit rather than implied.
        if tick.is_finite() && tick >= min - epsilon && tick <= max + epsilon {
            // Clean up floating point errors by rounding to appropriate precision
            ticks.push(clean_float(tick, step));
        }
    }

    // At extreme magnitudes two adjacent multiples can round to the same
    // representable value; a repeated tick is always a bug, never a feature.
    ticks.dedup();

    Some(ticks)
}

/// Clean up floating point errors by rounding to appropriate precision based on step size
pub(crate) fn clean_float(value: f64, step: f64) -> f64 {
    // Round to a precision appropriate for the step size
    let decimals = if step >= 1.0 {
        // `2.5` is the one ladder mantissa that is at least one but is not a
        // whole number. Rounding its ticks to integers does not clean up float
        // noise, it *moves* them: 2.5 becomes 3 while 5 stays 5, and the axis
        // comes out unevenly spaced ([0, 3, 5, 8, 10]).
        i32::from((step - step.round()).abs() > 1e-9)
    } else {
        (-step.log10().floor()) as i32 + 1
    };
    let mult = 10.0_f64.powi(decimals);
    if !mult.is_finite() || mult <= 0.0 {
        return value;
    }

    let cleaned = (value * mult).round() / mult;
    if !cleaned.is_finite() {
        // Sub-normal steps overflow the scaling; the raw value is better than
        // an infinity or a NaN.
        return value;
    }

    // Rounding a tiny negative residual yields -0.0, which formats as "-0".
    if cleaned == 0.0 { 0.0 } else { cleaned }
}

// ---------------------------------------------------------------------------
// Tick label formatting — the canonical path for every backend
// ---------------------------------------------------------------------------

/// The one formatter instance every backend shares.
fn shared_formatter() -> &'static TickFormatter {
    static FORMATTER: std::sync::LazyLock<TickFormatter> =
        std::sync::LazyLock::new(TickFormatter::default);
    &FORMATTER
}

/// Format a single tick value.
///
/// Prefer [`format_tick_labels`] whenever a whole axis is available: precision
/// chosen per value cannot be consistent across an axis.
pub fn format_tick_label(value: f64) -> String {
    shared_formatter().format_tick(value)
}

/// Format one axis worth of tick values with a single shared precision.
///
/// Precision is decided **per axis**, not per value, so the labels line up and
/// no axis ever mixes plain with scientific notation.
pub fn format_tick_labels(values: &[f64]) -> Vec<String> {
    shared_formatter().format_ticks(values)
}

/// Format one axis worth of tick values for the given scale.
///
/// This is the canonical tick label formatter: raster, SVG and the layout
/// measurement pass all go through it, so a figure's axis reads identically in
/// every backend.
///
/// A log axis picks **one** notation for the whole axis. While every ticked
/// magnitude stays inside [`LOG_PLAIN_LABEL_MIN`]..=[`LOG_PLAIN_LABEL_MAX`] the
/// decades read as plain decimals (`0.01`, `0.1`, `1`, `10`), because that is
/// what a reader expects of an ordinary range; outside that window every label
/// switches to the `10ⁿ` / `2×10ⁿ` exponent form together. An axis is never
/// allowed to show `10⁻¹` next to `0.5`.
///
/// A symlog axis applies that rule to its logarithmic regions only — the linear
/// region around zero keeps linear labels, which is the whole point of the
/// scale.
pub fn format_tick_labels_for_scale(values: &[f64], scale: &AxisScale) -> Vec<String> {
    match scale {
        AxisScale::Linear => format_tick_labels(values),
        AxisScale::Log => format_log_labels(values, 0.0),
        AxisScale::SymLog { linthresh } => format_log_labels(values, linthresh.abs()),
    }
}

/// Smallest magnitude a log axis still labels as a plain decimal (`0.0001`).
const LOG_PLAIN_LABEL_MIN: f64 = 1e-4;

/// Largest magnitude a log axis still labels as a plain decimal (`100000`).
const LOG_PLAIN_LABEL_MAX: f64 = 1e5;

/// Format one axis worth of log (or symlog) ticks.
///
/// `linthresh` is the symlog linear threshold; `0.0` means every tick belongs
/// to the logarithmic region, which is the plain log case.
fn format_log_labels(values: &[f64], linthresh: f64) -> Vec<String> {
    let plain = format_tick_labels(values);
    let in_log_region = |value: f64| value.abs() > linthresh;

    let reads_plain = values
        .iter()
        .zip(&plain)
        .all(|(&value, label)| !in_log_region(value) || plain_log_label_is_nice(value, label));
    if reads_plain {
        return plain;
    }

    values
        .iter()
        .zip(plain)
        .map(|(&value, plain)| {
            if in_log_region(value) {
                log_exponent_label(value).unwrap_or(plain)
            } else {
                plain
            }
        })
        .collect()
}

/// Would this tick still read well as the plain decimal `label`?
fn plain_log_label_is_nice(value: f64, label: &str) -> bool {
    let magnitude = value.abs();
    if !magnitude.is_finite() {
        return false;
    }
    if magnitude == 0.0 {
        // Zero reads the same in either notation, so it never forces a switch.
        return true;
    }

    // The shared formatter falling back to `1.00e-8` is itself proof that plain
    // decimals cannot carry this axis.
    !label.contains(['e', 'E'])
        && (LOG_PLAIN_LABEL_MIN * (1.0 - 1e-9)..=LOG_PLAIN_LABEL_MAX * (1.0 + 1e-9))
            .contains(&magnitude)
}

/// Format a single log-scale tick value.
pub fn format_log_tick_label(value: f64) -> String {
    format_tick_labels_for_scale(std::slice::from_ref(&value), &AxisScale::Log)
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// Render `value` in exponent form: `10ⁿ` on a decade, `2×10ⁿ` off one.
///
/// The sign is carried through so a symlog axis can label its negative
/// logarithmic region as `-10³`.
fn log_exponent_label(value: f64) -> Option<String> {
    if !value.is_finite() || value == 0.0 {
        return None;
    }

    let sign = if value < 0.0 { "-" } else { "" };
    let magnitude = value.abs();
    let log = magnitude.log10();
    if (log.round() - log).abs() < 1e-10 {
        return Some(format!(
            "{sign}10{}",
            superscript_exponent(log.round() as i32)
        ));
    }

    let mut exponent = log.floor() as i32;
    let mut mantissa = (magnitude / 10.0_f64.powi(exponent) * 100.0).round() / 100.0;
    if mantissa >= 10.0 {
        mantissa /= 10.0;
        exponent += 1;
    }
    let mantissa = TickFormatter::trim_trailing_zeros(&format!("{mantissa:.2}"));

    Some(format!(
        "{sign}{mantissa}×10{}",
        superscript_exponent(exponent)
    ))
}

fn superscript_exponent(exponent: i32) -> String {
    let exponent = exponent as i64;
    let mut formatted = String::new();
    if exponent < 0 {
        formatted.push('⁻');
    }

    for digit in exponent.abs().to_string().chars() {
        let superscript = match digit {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            _ => digit,
        };
        formatted.push(superscript);
    }

    formatted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ticks_basic() {
        let ticks = generate_ticks(0.0, 10.0, 5);
        assert!(!ticks.is_empty());
        assert!(ticks.len() <= 10);
        assert!(ticks[0] >= 0.0);
        assert!(*ticks.last().unwrap() <= 10.0);
    }

    #[test]
    fn test_generate_ticks_nice_numbers() {
        let ticks = generate_ticks(0.0, 100.0, 6);
        // Should produce nice numbers like 0, 20, 40, 60, 80, 100
        for tick in &ticks {
            let tick_int = *tick as i64;
            // Nice numbers are divisible by 1, 2, 5, or 10
            assert!(tick_int % 10 == 0 || tick_int % 5 == 0 || tick_int % 2 == 0);
        }
    }

    #[test]
    fn test_generate_minor_ticks() {
        let major = vec![0.0, 10.0, 20.0];
        let minor = generate_minor_ticks(&major, 4);
        assert_eq!(minor.len(), 8); // 4 between each pair of major ticks
        assert!(minor[0] > 0.0 && minor[0] < 10.0);
    }

    #[test]
    fn test_invalid_range() {
        let ticks = generate_ticks(10.0, 5.0, 5);
        assert!(!ticks.is_empty());
        assert!(ticks.windows(2).all(|window| window[0] <= window[1]));
        assert!(ticks[0] >= 5.0);
        assert!(*ticks.last().unwrap() <= 10.0);
    }

    #[test]
    fn test_generate_ticks_ordinary_ranges() {
        // Baseline for candidate scoring. Every one of these is the densest
        // rung of the ladder that still fits under the requested ceiling, which
        // is what `MaxNLocator(nbins = target - 1)` picks.

        // Five slots, and the 2.5 rung fills all five exactly. The 2-step
        // ([0, 2, … 10]) would need six and is therefore not eligible at all,
        // however much rounder a mantissa of 2 is than 2.5.
        assert_eq!(generate_ticks(0.0, 10.0, 5), vec![0.0, 2.5, 5.0, 7.5, 10.0]);
        // Asking for 3 still gives 3: the ceiling is honoured, not undercut.
        assert_eq!(generate_ticks(0.0, 10.0, 3), vec![0.0, 5.0, 10.0]);
        // Ten slots. The unit ruler needs eleven ticks to span 0..10, so it is
        // one over and drops out; 2 is the next rung that fits.
        assert_eq!(
            generate_ticks(0.0, 10.0, 10),
            vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]
        );
        assert_eq!(
            generate_ticks(-5.0, 5.0, 5),
            vec![-4.0, -2.0, 0.0, 2.0, 4.0]
        );
        assert_eq!(
            generate_ticks(-5.0, 5.0, 7),
            vec![-4.0, -2.0, 0.0, 2.0, 4.0]
        );
        assert_eq!(generate_ticks(0.7, 9.3, 5), vec![2.0, 4.0, 6.0, 8.0]);
        // The unit ruler over 0.7..9.3 is [1 … 9], nine ticks against a ceiling
        // of eight, so the 2-step wins even though it leaves slots unused.
        assert_eq!(generate_ticks(0.7, 9.3, 8), vec![2.0, 4.0, 6.0, 8.0]);
        assert_eq!(
            generate_ticks(0.0, 100.0, 6),
            vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]
        );
        // Reversed input is normalised, then held to the same ceiling: the unit
        // ruler over 5..10 is six ticks against a ceiling of five.
        assert_eq!(generate_ticks(10.0, 5.0, 5), vec![6.0, 8.0, 10.0]);
    }

    #[test]
    fn test_generate_ticks_never_returns_a_lone_tick() {
        // The old ladder rounded the step up unconditionally, so (0.7, 9.3)
        // with a target of 5 collapsed to the single tick [5.0]. An axis (or a
        // colorbar) with one tick is unreadable; scoring rejects candidates
        // that emit fewer than two ticks.
        for target in 3..=10 {
            for (min, max) in [(0.7, 9.3), (0.0, 1.0), (-0.217, 0.982), (3.0, 3.4)] {
                let ticks = generate_ticks(min, max, target);
                assert!(
                    ticks.len() >= 2,
                    "({min}, {max}) target {target} produced {ticks:?}"
                );
            }
        }
    }

    #[test]
    fn test_generate_ticks_awkward_range_lands_on_round_numbers() {
        // A range that does not straddle any obvious boundary must still tick
        // on round numbers rather than on the data endpoints.
        let ticks = generate_ticks(-0.217, 0.982, 6);

        assert_eq!(ticks, vec![-0.2, 0.0, 0.2, 0.4, 0.6, 0.8]);

        // Every tick is an exact multiple of the step.
        let step = ticks[1] - ticks[0];
        for tick in &ticks {
            let multiples = tick / step;
            assert!(
                (multiples - multiples.round()).abs() < 1e-9,
                "{tick} is not a multiple of the step {step}"
            );
        }
    }

    /// Widest formatted label on this axis, in characters.
    fn widest_label(ticks: &[f64]) -> usize {
        format_tick_labels(ticks)
            .iter()
            .map(|label| label.chars().count())
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn test_generate_ticks_target_is_a_hard_ceiling() {
        // Regression, twice over.
        //
        // The density term was once symmetric, so 11 ticks against a target of
        // 8 scored exactly as badly as 5.8 — and the step-0.1 candidate lost on
        // density but won on simplicity, putting eleven labels where
        // `MaxNLocator` emits six.
        //
        // Making the penalty asymmetric was not enough, because it left the
        // ceiling *soft*: the rungs of the ladder are a factor of two apart, so
        // the next rung down doubles the tick count for a relative overshoot
        // small enough to be bought back by a rounder mantissa. 0..1 against a
        // target of 10 came out as the 11-tick decimal ruler — one over the
        // ceiling, nearly free under a relative penalty, and twice matplotlib's
        // density. The ceiling is now a hard cut.
        assert_eq!(
            generate_ticks(0.0, 1.0, 8),
            vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]
        );
        assert_eq!(
            generate_ticks(0.0, 1.0, 10),
            vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]
        );

        // The property behind both: over a broad corpus of ordinary ranges, no
        // axis ever emits more ticks than it was allowed. This is what stops a
        // future scoring tweak from trading legibility for roundness again.
        for target in 3..=10 {
            for (min, max) in [
                (0.0, 1.0),
                (0.0, 10.0),
                (0.0, 100.0),
                (0.0, 2.5e7),
                (-1.0, 1.0),
                (-5.0, 5.0),
                (0.7, 9.3),
                (-0.8, 16.8),
                (-0.217, 0.982),
                (99_000.0, 101_000.0),
                (0.0, 1e9),
                (3.0, 3.4),
            ] {
                let ticks = generate_ticks(min, max, target);
                assert!(
                    ticks.len() <= target,
                    "({min}, {max}) target {target} emitted {} ticks: {ticks:?}",
                    ticks.len()
                );
            }
        }
    }

    #[test]
    fn test_generate_ticks_readability_wins_survive_the_ceiling() {
        // Both of these are wins from the scoring rewrite over the old fixed
        // 1/2/5/10 ladder, and the hard ceiling must not undo either.

        // 01_basic_line's y axis (0..16 plus the 5% autoscale margin). The
        // 2-step ruler [0, 2, … 16] is nine ticks against a ceiling of eight
        // and is not eligible. The old ladder, lacking a 2.5 rung, fell all the
        // way to a 5-step and showed four ticks; the 2.5 rung fills seven of
        // the eight slots instead, so the win here is over the *old* behaviour
        // and it survives.
        assert_eq!(
            generate_ticks(-0.8, 16.8, 8),
            vec![0.0, 2.5, 5.0, 7.5, 10.0, 12.5, 15.0]
        );

        // 20_dense_scatter's y axis. Seven ticks is under the ceiling, so this
        // one is chosen on density alone and is untouched by the change.
        assert_eq!(
            generate_ticks(-1.936_068_671_663_616, 1.935_948_520_314_347_8, 8),
            vec![-1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5]
        );
    }

    #[test]
    fn test_generate_ticks_long_labels_lower_the_ceiling() {
        // Regression: at 0..2.5e7 the generator hit its requested count of 10
        // and emitted 13 ticks, whose eight-character labels merged into runs
        // 135px and 180px wide. Nothing measured the labels, so an axis
        // reading "24000000" was allowed the same density as one reading "8".
        let ticks = generate_ticks(0.0, 2.5e7, 10);

        assert_eq!(
            ticks,
            vec![0.0, 5e6, 1e7, 1.5e7, 2e7, 2.5e7],
            "long labels must not be packed at the requested density"
        );

        // The emitted axis fits the character budget the cap is built from.
        let chars = widest_label(&ticks);
        assert_eq!(chars, 8, "expected labels like \"25000000\"");
        assert!(
            (ticks.len() as f64) * (chars as f64 + LABEL_GAP_CHARS) <= AXIS_LABEL_BUDGET_CHARS,
            "{} labels of {chars} chars do not fit the {AXIS_LABEL_BUDGET_CHARS}-char budget",
            ticks.len()
        );
    }

    #[test]
    fn test_generate_ticks_label_budget_is_monotone_in_label_width() {
        // Same nice-number structure at nine magnitudes: the axis whose labels
        // are longer may never carry *more* ticks than the axis whose labels
        // are shorter. This is the property the character budget exists to
        // enforce, stated without reference to any font.
        let mut previous = usize::MAX;
        let mut previous_chars = 0;
        for exponent in 0..9 {
            let max = 2.5 * 10.0_f64.powi(exponent);
            let ticks = generate_ticks(0.0, max, 10);
            let chars = widest_label(&ticks);

            assert!(
                chars < previous_chars || ticks.len() <= previous,
                "0..{max:e} widened labels to {chars} chars yet grew to {} ticks (was {previous})",
                ticks.len()
            );
            previous = ticks.len();
            previous_chars = chars;
        }
    }

    #[test]
    fn test_generate_ticks_are_evenly_spaced() {
        // `clean_float` used to round every step of one or more to whole
        // numbers, which does not clean up float noise for a 2.5 step, it moves
        // the ticks: 0, 2.5, 5, 7.5, 10 came out as 0, 3, 5, 8, 10. The
        // overshoot penalty makes the 2.5 ladder entry win more often, so an
        // unevenly spaced axis would now be easy to hit.
        for (min, max) in [
            (0.0, 25.0),
            (0.0, 10.0),
            (0.0, 2.5),
            (-8.34, 14.86),
            (268.879, 277.254),
        ] {
            for target in 3..=10 {
                let ticks = generate_ticks(min, max, target);
                if ticks.len() < 3 {
                    continue;
                }
                let step = ticks[1] - ticks[0];
                for pair in ticks.windows(2) {
                    assert!(
                        (pair[1] - pair[0] - step).abs() <= step.abs() * 1e-6,
                        "({min}, {max}) target {target} is unevenly spaced: {ticks:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_generate_ticks_step_below_ulp_terminates() {
        // step = 0.5 at magnitude 1e16, where one ULP is 2.0: those candidates
        // cannot advance the walk and must be rejected. Only step = 2 survives,
        // which happens to reproduce the endpoints exactly.
        let min = 1e16;
        let max = 1e16 + 2.0;
        let ticks = generate_ticks(min, max, 6);

        assert_eq!(ticks, vec![min, max]);
    }

    #[test]
    fn test_generate_ticks_unit_step_below_ulp_terminates() {
        // Same ULP trap, wider range. Baseline move: this used to fall back to
        // the two endpoints, and now emits the representable interior tick too
        // (2 -> 3 ticks). Every sub-ULP candidate is still rejected.
        let min = 1e16;
        let max = 1e16 + 5.0;
        let ticks = generate_ticks(min, max, 6);

        assert_eq!(ticks, vec![min, min + 2.0, max]);
        assert!(ticks.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn test_generate_ticks_returns_quickly_for_ulp_trap() {
        // Regression guard for the non-terminating walk: scoring evaluates
        // every candidate step, so the ULP guard has to hold for all of them —
        // including the label-width cap, which reads the emitted ticks and must
        // not resurrect a candidate the guard rejected.
        let start = std::time::Instant::now();
        for target in 3..=10 {
            let ticks = generate_ticks(1e16, 1e16 + 5.0, target);
            assert!(ticks.len() <= MAX_TICK_STEPS + 1);
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(1),
            "tick generation near 1e16 did not return promptly"
        );
    }

    #[test]
    fn test_generate_ticks_bounded_for_extreme_ranges() {
        let cases = [
            (0.0, f64::MAX),
            (-f64::MAX, f64::MAX),
            (1e300, 1e300 + 1.0),
            (-1e16, -1e16 + 2.0),
            (f64::MIN_POSITIVE, 1.0),
        ];

        for (min, max) in cases {
            let ticks = generate_ticks(min, max, 6);
            assert!(
                ticks.len() <= MAX_TICK_STEPS + 1,
                "unbounded tick count for ({min}, {max}): {}",
                ticks.len()
            );
            assert!(
                ticks.iter().all(|tick| tick.is_finite()),
                "non-finite tick for ({min}, {max}): {ticks:?}"
            );
        }
    }

    #[test]
    fn test_generate_ticks_never_emits_negative_zero() {
        // -0.0 formats as "-0"; the cleanup step must normalise it away.
        for target in 3..=10 {
            for (min, max) in [(-0.217, 0.982), (-0.4, 1.0), (-1.0, 3.0), (-0.05, 0.15)] {
                for tick in generate_ticks(min, max, target) {
                    assert!(
                        !(tick == 0.0 && tick.is_sign_negative()),
                        "negative zero from ({min}, {max}) target {target}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_format_tick_labels_share_one_precision() {
        assert_eq!(
            format_tick_labels(&[0.0, 0.5, 1.0, 1.5, 2.0]),
            vec!["0", "0.5", "1", "1.5", "2"]
        );
        assert_eq!(format_tick_labels(&[]), Vec::<String>::new());
    }

    #[test]
    fn test_no_axis_mixes_plain_and_scientific_notation() {
        // The old per-value switch in `TickLayout` flipped to scientific for
        // individual ticks, so one axis could read "1.0e5" next to "80000".
        // Notation is now decided per axis, so no axis can mix.
        let cases: [(f64, f64, AxisScale); 10] = [
            (0.0, 1.0, AxisScale::Linear),
            (0.0, 1e6, AxisScale::Linear),
            (99000.0, 101000.0, AxisScale::Linear),
            (0.0, 0.001, AxisScale::Linear),
            (1e-6, 5e-6, AxisScale::Linear),
            (-1e7, 1e7, AxisScale::Linear),
            (1.0, 1e5, AxisScale::Log),
            (1e-6, 1.0, AxisScale::Log),
            // Log range whose intermediate ticks are below plain resolution:
            // the decade labels and the intermediates must agree on notation.
            (1e-8, 1e-6, AxisScale::Log),
            (1.0, 100.0, AxisScale::Log),
        ];

        for (min, max, scale) in cases {
            let ticks = generate_ticks_for_scale(min, max, 6, &scale);
            let labels = format_tick_labels_for_scale(&ticks, &scale);

            assert_eq!(labels.len(), ticks.len());
            assert!(
                labels.iter().all(|label| !label.is_empty()),
                "empty label for ({min}, {max}) {scale:?}: {labels:?}"
            );

            // Every label on an axis uses the same notation, or none does.
            // A label of exactly "0" is notation-neutral.
            let candidates = labels.iter().filter(|label| label.as_str() != "0").count();
            let scientific = labels
                .iter()
                .filter(|label| label.contains(['e', 'E']))
                .count();
            assert!(
                scientific == 0 || scientific == candidates,
                "({min}, {max}) {scale:?} mixed notations: {labels:?}"
            );
        }
    }

    #[test]
    fn test_plain_representable_axes_stay_plain() {
        // Only an axis that plain decimals genuinely cannot render switches to
        // scientific; ordinary ranges keep their readable labels.
        for (min, max) in [(0.0, 1.0), (0.0, 1e6), (99000.0, 101000.0), (-1e7, 1e7)] {
            let ticks = generate_ticks(min, max, 6);
            let labels = format_tick_labels(&ticks);
            assert!(
                labels.iter().all(|label| !label.contains(['e', 'E'])),
                "unexpected scientific notation for ({min}, {max}): {labels:?}"
            );
        }
    }

    #[test]
    fn test_log_labels_use_one_notation_per_axis() {
        // Decades inside the plain window read as ordinary decimals.
        assert_eq!(
            format_tick_labels_for_scale(&[1.0, 10.0, 100.0, 1000.0], &AxisScale::Log),
            vec!["1", "10", "100", "1000"]
        );
        assert_eq!(
            format_tick_labels_for_scale(&[0.001, 0.01, 0.1, 1.0], &AxisScale::Log),
            vec!["0.001", "0.01", "0.1", "1"]
        );
        assert_eq!(format_log_tick_label(0.001), "0.001");
        assert_eq!(format_log_tick_label(20.0), "20");

        // One tick outside the window puts the *whole* axis in exponent form,
        // so an axis never shows "10⁻⁵" next to "0.001".
        assert_eq!(
            format_tick_labels_for_scale(&[1e-5, 1e-4, 1e-3], &AxisScale::Log),
            vec!["10⁻⁵", "10⁻⁴", "10⁻³"]
        );
        assert_eq!(
            format_tick_labels_for_scale(&[1e-8, 2e-8, 5e-8, 1e-7], &AxisScale::Log),
            vec!["10⁻⁸", "2×10⁻⁸", "5×10⁻⁸", "10⁻⁷"]
        );
        // Non-positive values have no decade; they must still format.
        assert_eq!(format_log_tick_label(0.0), "0");
        assert_eq!(format_log_tick_label(-5.0), "-5");
    }

    #[test]
    fn test_log_axis_labels_decades_only() {
        // Three decades: decade ticks only, all in one plain notation.
        let ticks = generate_ticks_for_scale(0.005, 1.0, 8, &AxisScale::Log);
        assert_eq!(ticks, vec![0.01, 0.1, 1.0]);
        assert_eq!(
            format_tick_labels_for_scale(&ticks, &AxisScale::Log),
            vec!["0.01", "0.1", "1"]
        );

        // A huge range labels the same decades in exponent form.
        let ticks = generate_ticks_for_scale(1e-9, 1e9, 8, &AxisScale::Log);
        let labels = format_tick_labels_for_scale(&ticks, &AxisScale::Log);
        assert!(
            labels.iter().all(|label| label.starts_with("10")),
            "expected exponent labels: {labels:?}"
        );
        assert!(labels.contains(&"10⁻⁹".to_string()));

        // A range too short for three decades keeps its 2/5 subdivisions, and
        // they read in the same notation as the decade beside them.
        let ticks = generate_ticks_for_scale(1.0, 10.0, 8, &AxisScale::Log);
        assert_eq!(ticks, vec![1.0, 2.0, 5.0, 10.0]);
        assert_eq!(
            format_tick_labels_for_scale(&ticks, &AxisScale::Log),
            vec!["1", "2", "5", "10"]
        );
    }

    #[test]
    fn test_symlog_labels_keep_linear_region_linear() {
        // Log regions far from zero go exponent form on both signs; the linear
        // region's own ticks stay linear.
        let labels = format_tick_labels_for_scale(
            &[-1e6, -1e3, 0.0, 1e3, 1e6],
            &AxisScale::SymLog { linthresh: 1.0 },
        );
        assert_eq!(labels, vec!["-10⁶", "-10³", "0", "10³", "10⁶"]);

        // A symlog axis whose decades all sit in the plain window stays plain.
        let ticks = generate_ticks_for_scale(-100.0, 100.0, 8, &AxisScale::symlog(1.0));
        let labels = format_tick_labels_for_scale(&ticks, &AxisScale::symlog(1.0));
        assert!(
            labels
                .iter()
                .all(|label| !label.contains('×') && !label.contains("10⁻")),
            "expected plain symlog labels: {labels:?}"
        );
        assert!(labels.contains(&"0".to_string()));
    }

    #[test]
    fn test_linear_labels_ignore_log_decade_formatting() {
        let labels = format_tick_labels_for_scale(&[1.0, 10.0, 100.0], &AxisScale::Linear);
        assert_eq!(labels, vec!["1", "10", "100"]);
    }

    #[test]
    fn test_log_ticks_powers_of_10() {
        let ticks = generate_log_ticks(1.0, 10000.0, 10);
        // Should include 1, 10, 100, 1000, 10000
        assert!(ticks.contains(&1.0));
        assert!(ticks.contains(&10.0));
        assert!(ticks.contains(&100.0));
        assert!(ticks.contains(&1000.0));
        assert!(ticks.contains(&10000.0));
    }

    #[test]
    fn test_log_ticks_few_decades() {
        // Three decades is enough to read on its own, so the axis is ticked at
        // the decades and nothing else — the 2/5 intermediates were what made a
        // log axis label itself in two notations.
        assert_eq!(generate_log_ticks(1.0, 100.0, 10), vec![1.0, 10.0, 100.0]);

        // Under three decades they come back, or the axis would have two ticks.
        assert_eq!(generate_log_ticks(1.0, 10.0, 10), vec![1.0, 2.0, 5.0, 10.0]);
    }

    #[test]
    fn test_log_ticks_invalid_range() {
        // Negative values should be handled gracefully
        let ticks = generate_log_ticks(-10.0, 100.0, 5);
        // Should return a fallback
        assert!(!ticks.is_empty());
    }

    #[test]
    fn test_log_ticks_preserve_sub_epsilon_range() {
        let min = f64::EPSILON / 1024.0;
        let max = f64::EPSILON / 16.0;
        let ticks = generate_log_ticks(min, max, 8);

        assert!(
            ticks.len() >= 2,
            "expected distinct sub-epsilon ticks: {ticks:?}"
        );
        assert!(ticks.iter().all(|tick| *tick >= min && *tick <= max));
        assert!(ticks.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn test_log_ticks_fall_back_to_narrow_sub_epsilon_endpoints() {
        let min = f64::EPSILON / 1024.0;
        let max = min * 1.5;
        let ticks = generate_log_ticks(min, max, 8);

        assert_eq!(ticks, vec![min, max]);
    }

    #[test]
    fn test_symlog_ticks_includes_zero() {
        let ticks = generate_symlog_ticks(-100.0, 100.0, 1.0, 10);
        // Should include 0 in the linear region
        assert!(ticks.contains(&0.0));
    }

    #[test]
    fn test_symlog_ticks_both_regions() {
        let ticks = generate_symlog_ticks(-1000.0, 1000.0, 1.0, 10);
        // Should have both positive and negative ticks
        let has_positive = ticks.iter().any(|&t| t > 1.0);
        let has_negative = ticks.iter().any(|&t| t < -1.0);
        assert!(has_positive);
        assert!(has_negative);
    }

    #[test]
    fn test_log_minor_ticks() {
        let major = vec![1.0, 10.0, 100.0];
        let minor = generate_log_minor_ticks(&major);
        // Should have 8 minor ticks between each pair (2,3,4,5,6,7,8,9)
        assert_eq!(minor.len(), 16); // 8 * 2 pairs
        assert!(minor.contains(&2.0));
        assert!(minor.contains(&5.0));
        assert!(minor.contains(&20.0));
        assert!(minor.contains(&50.0));
    }

    #[test]
    fn test_generate_ticks_for_scale() {
        // Linear
        let linear_ticks = generate_ticks_for_scale(0.0, 100.0, 5, &AxisScale::Linear);
        assert!(!linear_ticks.is_empty());

        let reversed_linear_ticks = generate_ticks_for_scale(4.0, 0.0, 5, &AxisScale::Linear);
        assert_eq!(reversed_linear_ticks.first().copied(), Some(0.0));
        assert_eq!(reversed_linear_ticks.last().copied(), Some(4.0));

        // Log
        let log_ticks = generate_ticks_for_scale(1.0, 1000.0, 5, &AxisScale::Log);
        assert!(log_ticks.contains(&10.0));
        assert!(log_ticks.contains(&100.0));

        let reversed_log_ticks = generate_ticks_for_scale(1000.0, 1.0, 5, &AxisScale::Log);
        assert_eq!(reversed_log_ticks.first().copied(), Some(1.0));
        assert_eq!(reversed_log_ticks.last().copied(), Some(1000.0));

        // SymLog
        let symlog_ticks = generate_ticks_for_scale(-100.0, 100.0, 10, &AxisScale::symlog(1.0));
        assert!(symlog_ticks.contains(&0.0) || symlog_ticks.iter().any(|&t| t.abs() < 0.1));
    }
}
