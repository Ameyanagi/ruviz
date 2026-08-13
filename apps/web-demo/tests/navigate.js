import { expect } from "@playwright/test";

/**
 * How long a parsed document may take to report its title. The title lives in
 * the static HTML and needs no wasm, so this is deliberately far shorter than
 * the readiness timeouts: it bounds "did the document arrive at all", not
 * "is the demo booted".
 */
const TITLE_TIMEOUT_MS = 15_000;

/**
 * How long the retried navigation may take. Longer than the first attempt so a
 * genuinely slow-but-healthy cold start still passes on the second try.
 */
const RETRY_TITLE_TIMEOUT_MS = 45_000;

/**
 * Navigate to a demo page, tolerating a single failed navigation.
 *
 * WebKit occasionally settles `goto` against an empty document: navigation
 * resolves, but nothing parses, so the title stays "" and every later
 * assertion fails against a blank page. It is transient — the identical commit
 * passed on rerun — and with Playwright's `retries: 0` it failed the whole
 * "WASM + Web" job (issue #149).
 *
 * The recovery is deliberately narrow. Only the navigation is retried, and
 * only once; a second empty document fails the test. Everything a caller
 * asserts after this returns stays strict, so real breakage still fails the
 * run rather than being papered over by a blanket retry policy.
 */
export async function gotoDemo(page, path, title) {
  try {
    await page.goto(path);
    await expect(page).toHaveTitle(title, { timeout: TITLE_TIMEOUT_MS });
    return;
  } catch (error) {
    console.warn(`navigation to ${path} produced no usable document, reloading once: ${error}`);
  }

  await page.goto(path);
  await expect(page).toHaveTitle(title, { timeout: RETRY_TITLE_TIMEOUT_MS });
}
