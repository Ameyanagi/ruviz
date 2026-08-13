import { expect } from "@playwright/test";

/**
 * How long a parsed document may take to report its title. The title lives in
 * the static HTML and needs no wasm, so this is deliberately far shorter than
 * the readiness budget: it bounds "did the document arrive at all", not "is
 * the demo booted".
 */
const TITLE_TIMEOUT_MS = 15_000;

/**
 * Readiness budget for the first attempt, deliberately shorter than the retry.
 * The two attempts plus the assertions a caller runs afterwards have to fit
 * inside the 120s per-test timeout, or a recoverable stall would resurface as
 * a timeout — a failure that says far less about what went wrong.
 */
const FIRST_READY_TIMEOUT_MS = 25_000;

/** Navigation and readiness budgets for the second attempt. */
const RETRY_TITLE_TIMEOUT_MS = 45_000;
const RETRY_READY_TIMEOUT_MS = 45_000;

/**
 * Open a demo page and wait for it to boot, tolerating one failed attempt.
 *
 * Two transient failures have been seen on webkit, both of which failed the
 * whole "WASM + Web" job because the Playwright config sets `retries: 0`
 * (issue #149):
 *
 * 1. `goto` settles against an empty document — navigation resolves, nothing
 *    parses, and the title stays "".
 * 2. The document parses and the title is right, but the wasm demo never
 *    finishes booting: `#capability-status` is present and empty for the full
 *    readiness timeout. This is what actually recurred on main at c8780ea,
 *    in the last test of the file, with the other 53 passing.
 *
 * The second is why readiness lives in here rather than in the caller: a
 * retry that only covers navigation does nothing for a boot that stalls after
 * a perfectly good navigation.
 *
 * The recovery stays narrow. One retry, covering only getting the page to a
 * booted state; everything a test asserts after this returns is strict, so a
 * demo that is genuinely broken still fails the run. A blanket `retries: 1`
 * would instead retry every assertion in the suite, which is the thing the
 * issue argued against.
 *
 * @param assertReady receives `(page, timeoutMs)` and must use that timeout
 *   for its own assertions, so both attempts stay inside the test budget.
 */
export async function openDemo(page, path, title, assertReady = null) {
  try {
    await page.goto(path);
    await expect(page).toHaveTitle(title, { timeout: TITLE_TIMEOUT_MS });
    if (assertReady) {
      await assertReady(page, FIRST_READY_TIMEOUT_MS);
    }
    return;
  } catch (error) {
    // A fresh navigation rather than reload(): it recovers the same stalls and
    // still works when the first goto itself threw.
    console.warn(`${path} did not reach a booted state, retrying once: ${error}`);
  }

  await page.goto(path);
  await expect(page).toHaveTitle(title, { timeout: RETRY_TITLE_TIMEOUT_MS });
  if (assertReady) {
    await assertReady(page, RETRY_READY_TIMEOUT_MS);
  }
}
