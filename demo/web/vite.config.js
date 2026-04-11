import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { defineConfig } from "vite";

const rootDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        main: resolve(rootDir, "index.html"),
        "bench-interactive": resolve(rootDir, "bench-interactive.html"),
        "bench-plotting": resolve(rootDir, "bench-plotting.html"),
      },
    },
  },
});
