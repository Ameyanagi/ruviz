import DefaultTheme from "vitepress/theme";
import type { Theme } from "vitepress";

import PlotGallery from "./components/PlotGallery.vue";
import "./style.css";

const theme: Theme = {
  ...DefaultTheme,
  enhanceApp({ app }) {
    DefaultTheme.enhanceApp?.({ app });
    app.component("PlotGallery", PlotGallery);
  },
};

export default theme;
