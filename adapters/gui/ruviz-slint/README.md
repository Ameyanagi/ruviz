# ruviz-slint

`ruviz-slint` embeds static or interactive ruviz 2D and 3D plots in native
Slint applications. Rendering is image-backed and happens on one persistent
background worker per slot; a Slint paint or input callback never renders a
plot.

## Component library

The crate is an experimental Slint component-library module. A consumer with a
normal Cargo dependency on `ruviz-slint` can use Slint's package import:

```slint
import { RuvizPlot } from "@Ruviz";

export component App inherits Window {
    RuvizPlot {
        slot-id: 1;
    }
}
```

`slot-id` is the component's only input. The controller-owned
`RuvizRuntime` global supplies its images, effective interaction state, image
fit, device scale, and all callbacks. `RuvizPlotGrid` is also exported for a
responsive multi-slot layout.

`RuvizPlot` stacks two `Image` elements in one fitted geometry: the plot base
and the interaction overlay. Slint's renderer composites them, so a hover,
tooltip, brush, or dynamic annotation only re-uploads the small overlay layer
and the plot base is left untouched. It enables that by calling
`RuvizRuntime.overlay-supported(slot-id)`; until some component does, the
controller keeps publishing one flat, pre-composed image in
`RuvizRuntime.slots[i].source`. A custom component built directly on the slots
model therefore keeps working unchanged, and opts into the cheaper layered
presentation by stacking `overlay` over `source` and announcing itself from its
own `init`.

The consumer build script must use Slint `~1.17` and enable its experimental
module support:

```toml
[dependencies]
ruviz-slint = "0.13.0"
slint = { version = "~1.17", default-features = false, features = [
    "std", "compat-1-2", "backend-winit", "renderer-femtovg"
] }

[build-dependencies]
slint-build = { version = "~1.17", features = ["experimental-module-builds"] }
```

```rust,ignore,reason=requires-consumer-slint-build-context
fn main() {
    slint_build::compile("ui/app.slint").unwrap();
}
```

No source path is configured by the application. Cargo passes the library
metadata emitted by `ruviz-slint` to Slint, which resolves `@Ruviz`.
`experimental-module-builds` is a Slint experimental API and may require a
small migration when upgrading Slint. This crate stays within the compatible
`1.17.x` series; applications should still review Slint patch release notes
because the component-library mechanism is experimental.

## Rust controller

`RuvizController` retains independent slots, so one controller drives every
`RuvizPlot` in a component tree:

```rust,ignore,reason=requires-generated-consumer-App-component
use ruviz::prelude::Plot;
use ruviz_slint::{RuvizController, SlotOptions};
use slint::ComponentHandle;

slint::include_modules!();
let app = App::new()?;
let controller = RuvizController::attach(&app);
controller.set_plot(
    1,
    Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
    SlotOptions::default(),
);
controller.resize(1, 800.0, 500.0, app.window().scale_factor());
# Ok::<(), Box<dyn std::error::Error>>(())
```

`RuvizController::attach` installs the shared model and callback handlers, and
installs the overlay layer sink once a component announces
`RuvizRuntime.overlay-supported`. `RuvizController::new` remains available for
application-owned image sinks; it receives one flat, pre-composed image unless
the application also installs `RuvizController::on_overlay` and presents the
overlay over the base itself.

Each slot supports:

- `Static` or `Interactive` mode;
- fill or fixed physical sizing;
- contain, cover, or fill image fitting;
- pan, wheel zoom, hover, click selection, right-button brush, and reset;
- release/capture/focus-loss drag cancellation;
- reactive 2D redraws and latest-request-only presentation;
- plot replacement with `set_plot_keep_view` to preserve a customized 2D view
  (an untouched view adopts the replacement's natural bounds), or
  `set_plot3d_keep_view` to preserve the 3D camera;
- 3D orbit, pan, zoom, reset, picking, and pick/camera/error callbacks.

An unmoved right click opens the packaged context menu. Right-drag remains a
2D brush or 3D pan after a three-logical-pixel threshold, so the menu does not
replace either gesture. The menu can reset or fit the view, save the installed
frame as PNG, copy it to the native image clipboard, toggle interaction, and
apply Isometric, Front, Back, Left, Right, Top, or Bottom views to a 3D plot.
It remains available for static slots so interaction can be enabled again.
Slint also supplies keyboard context-menu handling, including Menu and
Shift+F10 on Windows.

The backing size is computed from logical dimensions and the explicit
fractional device scale. Pointer coordinates are mapped through the actual
fitted image rectangle, including contain letterboxing and cover cropping.
The packaged component performs an initial resize handshake when its slot
appears. Applications should call `resize` with the new window scale when a
window moves between displays.

## Features and rendering

- Default: CPU image rendering for 2D.
- `3d`: CPU image rendering for 3D.
- `gpu`: forwards ruviz GPU capability.
- `3d-gpu`: 3D GPU rendering followed by readback into a Slint image.

GPU mode is not zero-copy. Slint receives a CPU-accessible
`SharedPixelBuffer`; the GPU path therefore includes explicit readback. Each
layer is copied into that buffer at most once, on a background worker, and only
when the layer actually changed.

The normal dependency deliberately enables neither a Slint renderer nor a
window backend. Applications choose those features, and **an interactive plot
should pick a GPU renderer**: `renderer-femtovg` (or `renderer-skia`) lets
Slint composite the base and overlay layers on the GPU, while
`renderer-software` blends every presented pixel on the CPU and caps the
achievable frame rate.

```toml
slint = { version = "~1.17", default-features = false, features = [
    "std", "compat-1-2", "backend-winit", "renderer-femtovg"
] }
```

This crate's own dev-dependencies enable `renderer-femtovg` **and**
`renderer-software`, because Cargo unifies dev-dependency features across
examples and tests: the examples want the GPU renderer and the headless
`i-slint-backend-testing` tests need the software one. Slint selects at
runtime, and `SLINT_BACKEND=winit-software` forces the software path.

## Examples

Run the interactive 2D dashboard from the repository root:

```sh
cargo run --manifest-path adapters/gui/ruviz-slint/Cargo.toml \
  --example dashboard
```

The mixed dashboard places an interactive 2D slot beside interactive and
static 3D slots, and demonstrates pick and camera callbacks. The plot titles
and startup output list the available gestures and context-menu actions:

```sh
cargo run --manifest-path adapters/gui/ruviz-slint/Cargo.toml \
  --features 3d --example mixed_3d_dashboard
```

## Licensing

This adapter's Rust and `.slint` sources are dual-licensed under MIT or
Apache-2.0, matching ruviz. That does not relicense Slint. Slint currently
offers GPLv3, a royalty-free desktop/mobile/web option that requires
attribution, and paid commercial licensing; its royalty-free option excludes
embedded use. Downstream users must choose and comply with an appropriate
Slint license, and proprietary embedded use may require the appropriate
commercial license. Review Slint's
[license summary](https://github.com/slint-ui/slint/blob/master/LICENSE.md) and
[terms](https://slint.dev/terms-and-conditions) for the application being
shipped; this section is a compatibility note, not legal advice or a legal
guarantee.
