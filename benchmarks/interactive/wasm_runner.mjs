import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const baseUrl = "http://127.0.0.1:4173";

function parseArgs(argv) {
  const parsed = {
    mode: "full",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--manifest") {
      parsed.manifest = argv[++index];
      continue;
    }
    if (arg === "--mode") {
      parsed.mode = argv[++index];
      continue;
    }
    if (arg === "--output") {
      parsed.output = argv[++index];
      continue;
    }
    throw new Error(`unexpected argument: ${arg}`);
  }

  if (!parsed.manifest) {
    throw new Error("missing --manifest");
  }
  if (!parsed.output) {
    throw new Error("missing --output");
  }
  return parsed;
}

async function runCommand(command, args, options = {}) {
  await new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(command, args, {
      cwd: root,
      stdio: "inherit",
      ...options,
    });
    child.on("error", rejectPromise);
    child.on("exit", (code) => {
      if (code === 0) {
        resolvePromise();
      } else {
        rejectPromise(new Error(`${command} ${args.join(" ")} exited with code ${code}`));
      }
    });
  });
}

async function waitForServer(url, timeoutMs = 120000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
    } catch {
      // Server not ready yet.
    }
    await Bun.sleep(500);
  }
  throw new Error(`timed out waiting for ${url}`);
}

async function terminateChild(child) {
  if (child.exitCode !== null) {
    return;
  }

  child.kill("SIGTERM");
  await Promise.race([
    new Promise((resolvePromise) => child.once("exit", resolvePromise)),
    new Promise((resolvePromise) =>
      setTimeout(() => {
        if (child.exitCode === null) {
          child.kill("SIGKILL");
        }
        resolvePromise();
      }, 5000),
    ),
  ]);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const manifestPath = resolve(root, args.manifest);
  const outputPath = resolve(root, args.output);
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));

  await runCommand("bun", ["run", "--cwd", "demo/web", "build"]);

  const preview = spawn("bunx", ["vite", "preview", "--host", "127.0.0.1", "--port", "4173"], {
    cwd: resolve(root, "demo/web"),
    stdio: "inherit",
  });

  try {
    await waitForServer(`${baseUrl}/bench-interactive.html`);

    const browser = await chromium.launch({ headless: true });
    try {
      const page = await browser.newPage();
      await page.goto(`${baseUrl}/bench-interactive.html`, { waitUntil: "networkidle" });
      const payload = await page.evaluate(
        async ({ manifestInput, mode }) =>
          window.__ruvizInteractiveBenchmark.run(manifestInput, mode),
        {
          manifestInput: manifest,
          mode: args.mode,
        },
      );

      payload.environment.browserVersion = browser.version();
      payload.environment.baseUrl = baseUrl;

      await mkdir(dirname(outputPath), { recursive: true });
      await writeFile(outputPath, JSON.stringify(payload, null, 2), "utf8");
    } finally {
      await browser.close();
    }
  } finally {
    await terminateChild(preview);
  }
}

await main();
