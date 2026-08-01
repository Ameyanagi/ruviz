# Native GUI Adapter Plan

## Goal

Add first-class native adapters for Iced, egui, and Slint, and bring the
existing GPUI 3D adapter up to the same standard.

Every adapter must support:

- static and interactive 2D plots;
- static and interactive 3D plots;
- native Linux, macOS, and Windows applications;
- responsive resize and HiDPI rendering;
- image-backed CPU rendering by default;
- optional `gpu` and `3d-gpu` rendering through truthful GPU readback;
- framework-native state, widgets, events, and repaint scheduling;
- the same concepts and names where framework conventions allow it.

Static mode disables user interaction, but it still responds to resize, plot
replacement, and reactive 2D data updates.

Direct zero-copy texture or surface interop is not part of the first release.
The GUI frameworks do not all expose compatible renderer/device ownership, and
some currently resolve different `wgpu` versions or use non-wgpu renderers.

## Repository Layout

Keep GUI dependency resolution isolated from the root workspace:

```text
adapters/
  gui/
    Cargo.toml
    Cargo.lock
    ruviz-egui/
    ruviz-iced/
    ruviz-slint/
  gpui/
```

`adapters/gui` will be an independent virtual workspace and will be
listed in the root workspace's `exclude` array. The three adapters share one
lockfile. GPUI remains in its existing independent workspace because of its
Zed/GPUI patch.

This keeps the root lockfile and normal core CI free from the large GUI,
windowing, and renderer dependency graphs.

Initial framework versions:

- egui 0.35;
- Iced 0.14;
- Slint 1.17.x.

All three are compatible with the project's Rust 1.92 MSRV. Framework app
shells such as `eframe` belong in dev-dependencies for examples, not in the
adapter's normal dependency graph.

## Shared Public Concepts

The exact construction is framework-native, but all adapters use these names:

- `plot_builder(...)` and `plot3d_builder(...)`;
- `static_view()` and `interactive()`;
- `fill()` and `fixed_pixels(width, height)`;
- `set_plot(...)` and `set_plot_keep_view(...)`;
- `session()`;
- click, hover, selection, pick, camera-change, and error events;
- `3d`, `gpu`, and `3d-gpu` Cargo features.

The core crate should provide shared conversion traits so adapters do not each
invent their own:

```rust
pub trait IntoPlotSession {
    fn into_plot_session(self) -> InteractivePlotSession;
}

pub trait TryIntoPlot3DSession {
    fn try_into_plot3d_session(self) -> Result<InteractivePlot3DSession>;
}
```

`Plot`, `PreparedPlot`, and existing sessions implement the relevant traits.

### egui

egui uses an app-owned retained object with immediate-mode presentation:

```rust
let mut plot = ruviz_egui::plot_builder(plot)
    .interactive()
    .build();

egui::CentralPanel::default().show(ctx, |ui| {
    let response = plot.show(ui);
    if let Some(click) = response.clicked {
        // Update application state.
    }
});
```

`RuvizPlot` and `RuvizPlot3D` retain the session, texture, render scheduler,
interaction state, and last error. Texture allocations are reused with
`TextureHandle::set`; the adapter must not call `load_texture` every frame.

### Iced

Iced keeps authoritative state in the host's Elm state and emits messages:

```rust
enum Message {
    Plot(ruviz_iced::Message),
}

// update
Message::Plot(message) => {
    state.plot.update(message).task.map(Message::Plot)
}

// view
ruviz_iced::plot(&state.plot, Message::Plot)

// subscription
state.plot.subscription().map(Message::Plot)
```

The adapter exposes `PlotState`, `Plot3DState`, and thin custom widgets. A
custom widget is preferred over composing only `MouseArea`, because global
button release, focus loss, and release outside the bounds must cancel drags.

### Slint

The adapter ships a real Slint component library:

```slint
import { RuvizPlot } from "@Ruviz";

RuvizPlot {
    slot: 0;
}
```

The crate uses Slint's component-library build support so consumers do not copy
files or configure hard-coded source paths. A slot-based `RuvizRuntime` global
holds frame models and forwards component callbacks to a retained Rust
controller:

```rust
let controller = ruviz_slint::RuvizController::attach(
    &app,
    [
        PlotSlot::interactive_2d(plot),
        PlotSlot::static_3d(surface),
    ],
)?;
```

The controller must be retained for the component tree's lifetime. Multiple
widgets can use separate slots, and mirrored widgets may intentionally share a
slot.

### GPUI

The existing 2D builder remains the reference API. Add a matching 3D builder:

```rust
let plot = ruviz_gpui::plot3d_builder(surface)
    .interactive()
    .fill()
    .on_pick(|hit| { /* ... */ })
    .build(cx);
```

Keep the existing `plot3d(session, cx)` helper as a compatibility shortcut.
Deprecate unrestricted `session_mut()` only after replacement and explicit
camera mutation APIs can invalidate and notify correctly.

## Core Prerequisites

The current 2D retained session already has dirty domains, generation-aware
frames, a render gate, and reactive subscriptions. The current 3D GPUI path
renders synchronously during prepaint and is not safe to copy into new
adapters.

Implement these core contracts first:

1. **Generated frame stamps**
   - Return an opaque render stamp with every generated 2D or 3D image.
   - Expose `is_render_stamp_current(stamp)`.
   - Represent superseded work with a typed result/error instead of parsing an
     error string.

2. **Unified change notifications**
   - Add a dependency-free `subscribe_changes` mechanism with a monotonic
     revision.
   - Notify for data, view, camera, resize, replacement, and invalidation
     changes.
   - Invoke callbacks outside session locks.

3. **Background-safe 3D rendering**
   - Create a sendable native render job from a scene/camera/target snapshot.
   - Keep retained GPU resources on a worker/cache instead of recreating the
     renderer for every frame.
   - Distinguish scene, camera, and render-target generations.

4. **3D interaction lifecycle**
   - Add a source-compatible drag-cancellation method.
   - Clear or invalidate picks after scene, camera, or target changes.
   - Add replacement and keep-camera operations.
   - Return render statistics and diagnostics.

5. **Explicit image alpha mode**
   - Mark rendered images as straight or premultiplied RGBA, or expose a
     normalization method.
   - Adapters must not guess when constructing framework image buffers.

6. **Small adapter utilities**
   - Share pure coordinate conversion, fitted-content bounds, physical backing
     size, render request identity, and latest-request scheduling.
   - Keep GUI events, clipboard, dialogs, runtime tasks, and framework callbacks
     inside each adapter.

Observable/source-backed 3D data is a separate core feature. The first adapter
release supports interactive 3D and host-driven plot replacement, but must not
claim observable 3D updates until 3D series gain source-backed data, scene/BVH
invalidation, and coalesced change notifications.

## Rendering and Scheduling Contract

All adapters follow the same rules:

- Rendering never blocks the UI paint/view callback.
- Keep at most one render in flight and one coalesced latest request.
- A completion is installed only when its request and session stamp are still
  current.
- Keep displaying the last good frame while new work is pending.
- Texture/image creation that is restricted to the UI thread happens only
  after the background RGBA render completes.
- Resize uses physical backing pixels derived from logical bounds and the
  current scale factor.
- Pointer coordinates are mapped through the fitted image content bounds, not
  blindly through the outer widget bounds.
- Static widgets ignore user input but still invalidate for resize, replacement,
  and reactive 2D data.
- Render errors are observable and do not erase the last good frame.

The first implementation is image-backed. `gpu` means ruviz GPU rendering
followed by readback and framework upload. It does not imply zero-copy or direct
surface presentation.

## Implementation Phases

### Phase 1: Core adapter contract

- Add conversion traits, generated-frame stamps, typed supersession, and
  `subscribe_changes`.
- Add explicit alpha handling.
- Make 3D render jobs background-safe.
- Add pure adapter utility tests.
- Add 3D cancellation, pick validity, replacement, and keep-camera APIs.

Exit gate: deterministic concurrency tests prove that slow frame A cannot
replace newer frame C after resize, input, or plot replacement.

### Phase 2: GPUI 3D parity

- Replace synchronous prepaint rendering with latest-request background work.
- Add `plot3d_builder`, static/interactive modes, sizing, image fit, backend
  preference, pick/error callbacks, and plot replacement.
- Gate scrolling to content bounds.
- Handle pointer-up outside, focus loss, and drag cancellation.
- Use fitted content bounds for painting and coordinate mapping.
- Clear stale picks and expose camera-change events.
- Add static and interactive 3D examples and tests.

Exit gate: GPUI passes the same 3D behavioral contract as the new adapters.

### Phase 3: Adapter implementations

After Phases 1 and 2 freeze the behavior, these tracks can run in parallel:

- `ruviz-egui`: retained state, reusable texture, raw event filtering,
  repaint wakeups, and `show` responses;
- `ruviz-iced`: Elm-owned state, custom widget, tasks/subscriptions, and
  retained image allocation;
- `ruviz-slint`: packaged `@Ruviz` component library, slot runtime, controller,
  background buffers, and event-loop delivery.

Each track delivers static 2D, interactive 2D, static 3D, interactive 3D, a
mixed dashboard example, unit tests, and documentation.

### Phase 4: Cross-adapter behavior and review

- Run one shared acceptance matrix against all four adapters.
- Check multiple widgets in one window.
- Check plot/session replacement during an in-flight render.
- Check transparent and opaque image paths.
- Review scroll propagation inside framework scroll containers.
- Review Slint licensing and document the result.

### Phase 5: CI, packaging, documentation, and release

- Add a dedicated adapter matrix for Linux, macOS, and Windows.
- Check default, `3d`, and all-feature builds plus examples and doc tests.
- Add a Rust 1.92 MSRV job.
- Keep deterministic controller tests headless; use only minimal native-window
  smoke tests.
- Scope caches by workspace lockfile:
  - root: `Cargo.lock`;
  - GPUI: `adapters/gpui/Cargo.lock`;
  - GUI adapters: `adapters/gui/Cargo.lock`.
- Generalize packaged-crate verification for all adapters and compile fresh
  external consumers from extracted archives.
- Verify Slint's `@Ruviz` import from the packaged archive.
- Add adapter rustdoc to the documentation build.
- Publish `ruviz` first, then publish GPUI, egui, Iced, and Slint adapters after
  the exact core version is available.

## Acceptance Matrix

| Behavior | GPUI | egui | Iced | Slint |
| --- | --- | --- | --- | --- |
| Static 2D | Required | Required | Required | Required |
| Interactive 2D | Required | Required | Required | Required |
| Static 3D | Required | Required | Required | Required |
| Interactive 3D | Required | Required | Required | Required |
| Resize and fractional HiDPI | Required | Required | Required | Required |
| Pan, zoom, hover, select | Required | Required | Required | Required |
| Orbit, pan, zoom, pick, reset | Required | Required | Required | Required |
| Release outside/cancel drag | Required | Required | Required | Required |
| Reactive 2D wakeup | Required | Required | Required | Required |
| Latest-frame-only install | Required | Required | Required | Required |
| Last-good-frame on error | Required | Required | Required | Required |
| Multiple widgets | Required | Required | Required | Required |
| CPU image path | Required | Required | Required | Required |
| GPU readback feature | Compile + test | Compile + test | Compile + test | Compile + test |

Test scale factors at `1.0`, `1.25`, `1.5`, and `2.0`, including fractional
logical widget bounds. Test center and corner coordinate round trips through
letterboxed/fitted content bounds.

## Main Risks

- 3D background rendering and retained GPU ownership are the largest core
  changes.
- Alpha-mode ambiguity must be resolved before adapter image uploads are
  trusted.
- Framework dependency updates can raise MSRV or change event APIs.
- Iced's message model, egui's immediate mode, and Slint's declarative model
  need consistent behavior without pretending their scheduling APIs are
  identical.
- Slint component-library support is currently experimental and needs a
  packaged-consumer contract test.
- Slint licensing terms must be explicitly reviewed for this distribution.
- Headless GUI tests can be flaky; most behavior should be tested below the
  actual window boundary.
- Direct GPU interop would require a separate design for external device,
  queue, texture, target-format, and renderer lifetime ownership.

## Definition of Done

The feature is complete when all four adapters pass the acceptance matrix on
Linux, macOS, and Windows; examples compile and run; packaged external
consumers compile; documentation shows consistent static and interactive
2D/3D usage; CI and release jobs cover every crate; and no adapter blocks its UI
paint callback while rendering.
