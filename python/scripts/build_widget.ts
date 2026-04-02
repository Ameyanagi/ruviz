import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..", "..");
const bunExecutable = process.execPath;

async function runBun(args: string[], env: Record<string, string> = {}) {
  const child = Bun.spawn([bunExecutable, ...args], {
    cwd: repoRoot,
    env: {
      ...process.env,
      ...env,
    },
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });

  const exitCode = await child.exited;
  if (exitCode !== 0) {
    process.exit(exitCode);
  }
}

await runBun(["run", "build:web-sdk"], {
  RUVIZ_WASM_PACK_NO_OPT: "1",
});
await runBun(["python/scripts/build_widget_bundle.ts"]);
