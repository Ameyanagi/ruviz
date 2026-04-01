<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

import { webExamples } from "../../../../examples/index.js";
import type { PlotExample } from "../../../../examples/types.js";

const props = defineProps<{
  categories?: string[];
}>();

const entries = computed(() => {
  if (!props.categories || props.categories.length === 0) {
    return webExamples;
  }
  const allowed = new Set(props.categories);
  return webExamples.filter((example) => allowed.has(example.category));
});

const canvases = new Map<string, HTMLCanvasElement>();
const cleanups = new Map<string, () => void>();
const failures = ref<Record<string, string>>({});

function setCanvas(slug: string, canvas: HTMLCanvasElement | null): void {
  if (!canvas) {
    canvases.delete(slug);
    return;
  }
  canvases.set(slug, canvas);
}

function bindCanvas(slug: string) {
  return (canvas: Element | null) => {
    setCanvas(slug, canvas instanceof HTMLCanvasElement ? canvas : null);
  };
}

async function mountExample(example: PlotExample): Promise<void> {
  const canvas = canvases.get(example.slug);
  if (!canvas) {
    return;
  }

  try {
    if (example.mount) {
      const cleanup = await example.mount(canvas);
      if (typeof cleanup === "function") {
        cleanups.set(example.slug, cleanup);
      }
      return;
    }

    if (!example.createPlot) {
      return;
    }

    const session = await example.createPlot().mount(canvas, { autoResize: true });
    cleanups.set(example.slug, () => session.destroy());
  } catch (error) {
    failures.value = {
      ...failures.value,
      [example.slug]: error instanceof Error ? error.message : String(error),
    };
  }
}

onMounted(async () => {
  for (const example of entries.value) {
    await mountExample(example);
  }
});

onBeforeUnmount(() => {
  for (const cleanup of cleanups.values()) {
    cleanup();
  }
  cleanups.clear();
});
</script>

<template>
  <div class="plot-gallery">
    <article v-for="example in entries" :key="example.slug" class="plot-card">
      <h3>{{ example.title }}</h3>
      <p>{{ example.summary }}</p>
      <canvas :ref="bindCanvas(example.slug)"></canvas>
      <p v-if="failures[example.slug]"><strong>Render error:</strong> {{ failures[example.slug] }}</p>
      <details>
        <summary>Source</summary>
        <pre><code>{{ example.source }}</code></pre>
      </details>
    </article>
  </div>
</template>
