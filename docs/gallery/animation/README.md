# Animation Gallery

Animated visualizations using the `record!` macro and Signal-based reactive animations.

## Examples

### Traveling Sine Wave

![Traveling Sine Wave](../../images/animation_sine_wave.gif)

[View source code](https://github.com/Ameyanagi/ruviz/blob/main/examples/generate_animation_gallery.rs#L47-L73)

---

### Expanding Spiral Pattern

![Expanding Spiral Pattern](../../images/animation_growing_scatter.gif)

[View source code](https://github.com/Ameyanagi/ruviz/blob/main/examples/generate_animation_gallery.rs#L76-L103)

---

### Animated Bar Chart

![Animated Bar Chart](../../images/animation_bars.gif)

[View source code](https://github.com/Ameyanagi/ruviz/blob/main/examples/generate_animation_gallery.rs#L106-L138)

---

### Archimedean Spiral Growth

![Archimedean Spiral Growth](../../images/animation_spiral.gif)

[View source code](https://github.com/Ameyanagi/ruviz/blob/main/examples/generate_animation_gallery.rs#L141-L172)

---

### Signal Composition

![Signal Composition](../../images/animation_composition.gif)

Uses `Signal` combinators for reactive amplitude and frequency animation.

[View source code](https://github.com/Ameyanagi/ruviz/blob/main/examples/generate_animation_gallery.rs#L175-L208)

---

### Wave Interference Patterns

![Wave Interference Patterns](../../images/animation_interference.gif)

Shows traveling wave, standing wave, and damped wave overlaid.

[View source code](https://github.com/Ameyanagi/ruviz/blob/main/examples/generate_animation_gallery.rs#L211-L253)

---

### Easing Functions Demo

![Easing Functions Demo](../../images/animation_easing.gif)

Compares linear, ease-out-cubic, elastic, and bounce easing functions.

[View source code](https://github.com/Ameyanagi/ruviz/blob/main/examples/generate_animation_gallery.rs#L256-L297)

---

## Running the Examples

Generate all animation GIFs:

```bash
cargo run --features animation --example generate_animation_gallery
```

[← Back to Gallery](../README.md)
