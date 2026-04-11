#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageDir = resolve(scriptDir, "..");
const requiredTarballFiles = [
  "dist/index.js",
  "generated/raw/ruviz_web_raw.d.ts",
  "generated/raw/ruviz_web_raw.js",
  "generated/raw/ruviz_web_raw_bg.wasm",
];
const requiredInstalledFiles = [
  "dist/index.js",
  "generated/raw/ruviz_web_raw.js",
  "generated/raw/ruviz_web_raw_bg.wasm",
];

function run(command, args, cwd) {
  try {
    return execFileSync(command, args, {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    if (!(error instanceof Error)) {
      throw error;
    }

    const stdout = typeof error.stdout === "string" ? error.stdout.trim() : "";
    const stderr = typeof error.stderr === "string" ? error.stderr.trim() : "";
    const status = typeof error.status === "number" ? error.status : "unknown";
    const details = [
      `Command failed: ${command} ${args.join(" ")}`,
      `exit status: ${status}`,
    ];

    if (stdout.length > 0) {
      details.push(`stdout:\n${stdout}`);
    }

    if (stderr.length > 0) {
      details.push(`stderr:\n${stderr}`);
    }

    throw new Error(details.join("\n\n"));
  }
}

function assertFilesExist(baseDir, relativePaths, scope) {
  const missing = relativePaths.filter((relativePath) => !existsSync(join(baseDir, relativePath)));
  if (missing.length > 0) {
    throw new Error(`${scope} is missing required files:\n${missing.map((path) => `- ${path}`).join("\n")}`);
  }
}

const tempRoot = mkdtempSync(join(tmpdir(), "ruviz-web-pack-"));
let keepTemp = false;

try {
  const packDir = join(tempRoot, "pack");
  mkdirSync(packDir, { recursive: true });

  const packOutput = run("npm", ["pack", "--json", "--pack-destination", packDir], packageDir);
  const packEntries = JSON.parse(packOutput);
  if (!Array.isArray(packEntries) || packEntries.length !== 1) {
    throw new Error(`Expected one npm pack result, received: ${packOutput}`);
  }

  const packEntry = packEntries[0];
  const tarballFiles = new Set(packEntry.files.map((entry) => entry.path));
  const missingTarballFiles = requiredTarballFiles.filter((relativePath) => !tarballFiles.has(relativePath));
  if (missingTarballFiles.length > 0) {
    throw new Error(
      `npm pack tarball is missing required files:\n${missingTarballFiles.map((path) => `- ${path}`).join("\n")}`,
    );
  }

  const tarballPath = join(packDir, packEntry.filename);
  const installDir = join(tempRoot, "install");
  mkdirSync(installDir, { recursive: true });
  writeFileSync(join(installDir, "package.json"), JSON.stringify({ name: "ruviz-pack-verify", private: true }, null, 2));

  run("bun", ["add", tarballPath], installDir);

  const installedPackageDir = join(installDir, "node_modules", packEntry.name);
  assertFilesExist(installedPackageDir, requiredInstalledFiles, "Installed package");

  console.log(
    `Verified ${packEntry.name}@${packEntry.version} tarball contains wasm runtime assets and survives local bun install.`,
  );
} catch (error) {
  keepTemp = true;
  const message = error instanceof Error ? error.message : String(error);
  console.error(message);
  console.error(`Preserved debug artifacts at ${tempRoot}`);
  process.exitCode = 1;
} finally {
  if (!keepTemp) {
    rmSync(tempRoot, { recursive: true, force: true });
  }
}
