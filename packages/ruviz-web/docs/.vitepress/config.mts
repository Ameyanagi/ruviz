import { defineConfig } from "vitepress";
import { fileURLToPath } from "node:url";

const packageRoot = fileURLToPath(new URL("../../", import.meta.url));

export default defineConfig({
  title: "ruviz",
  description: "Browser-first plotting SDK docs for ruviz.",
  themeConfig: {
    nav: [
      { text: "Guide", link: "/guide/getting-started" },
      { text: "Gallery", link: "/guide/gallery" },
      { text: "API", link: "/api/" },
    ],
    sidebar: [
      {
        text: "Guide",
        items: [
          { text: "Getting Started", link: "/guide/getting-started" },
          { text: "Plot Types", link: "/guide/plot-types" },
          { text: "Interactivity", link: "/guide/interactivity" },
          { text: "Gallery", link: "/guide/gallery" },
        ],
      },
      {
        text: "API",
        items: [{ text: "Overview", link: "/api/" }],
      },
    ],
  },
  vite: {
    server: {
      fs: {
        allow: [packageRoot],
      },
    },
  },
});
