//! `cargo run --release --example molecular_spheres --features 3d-gpu`
//! Also works with `--features 3d` using software rendering.
mod support;

use gpui::{
    App, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb,
    size,
};
use ruviz::prelude::*;
use ruviz_gpui::{Plot3DEvent, RuvizPlot3D, plot3d_builder};
use support::{application, exit_on_window_open_failure};

fn molecule() -> Line3DBuilder {
    let mut atoms = vec![Sphere3D::new(
        0,
        Point3D::new(0.0, 0.0, 0.0),
        0.9,
        Color::ORANGE,
    )];
    for (shell, radius) in [2.5, 6.0, 9.0].into_iter().enumerate() {
        for i in 0..8 {
            let angle = i as f64 * std::f64::consts::TAU / 8.0;
            let oxygen = i % 2 == 0;
            let color = if oxygen {
                Color::from_rgb(211, 65, 68)
            } else {
                Color::from_rgb(45, 130, 170)
            };
            atoms.push(Sphere3D::new(
                atoms.len() as u32,
                Point3D::new(
                    radius * angle.cos(),
                    radius * angle.sin(),
                    if i % 2 == 0 { 0.5 } else { -0.5 },
                ),
                if oxygen { 0.45 } else { 0.7 },
                if shell == 2 {
                    color.with_alpha(0.15)
                } else {
                    color
                },
            ));
        }
    }
    let x: Vec<_> = (0..=128)
        .map(|i| 8.0 * (i as f64 * std::f64::consts::TAU / 128.0).cos())
        .collect();
    let y: Vec<_> = (0..=128)
        .map(|i| 8.0 * (i as f64 * std::f64::consts::TAU / 128.0).sin())
        .collect();
    let mut p = spheres3d(&atoms)
        .axes(false)
        .title("Molecular sphere rendering")
        .xlabel("x (Å)")
        .ylabel("y (Å)")
        .zlabel("z (Å)")
        .line3d(&x, &y, &vec![0.0; x.len()])
        .color(Color::from_rgb(120, 130, 140))
        .line_width(1.0)
        .line_style(LineStyle::Dashed)
        .label("8 Å cutoff");
    for atom in &atoms[1..9] {
        let q = atom.center;
        p = p
            .line3d(&[0.0, q.x], &[0.0, q.y], &[0.0, q.z])
            .color(Color::from_rgb(95, 100, 110))
            .line_width(3.0);
    }
    // Deliberately depth-tested, including arrowheads. A host that needs
    // always-on-top labels should place those in its own GPUI overlay.
    p = p
        .line3d(&[0.0, 2.5, 1.77], &[0.0, 0.0, 1.77], &[0.0, 0.5, -0.5])
        .color(Color::from_rgb(150, 70, 210))
        .line_width(2.5)
        .label("1 → 2 outward");
    p = p
        .line3d(&[1.77, 0.0], &[1.97, 0.2], &[-0.5, 0.0])
        .color(Color::from_rgb(20, 140, 110))
        .line_width(2.5)
        .label("2 → 1 return");
    p.line3d(
        &[1.95, 1.77, 2.25],
        &[1.2, 1.77, 1.4],
        &[-0.35, -0.5, -0.35],
    )
    .color(Color::from_rgb(150, 70, 210))
    .line_width(2.5)
}

struct MolecularDemo {
    plot: gpui::Entity<RuvizPlot3D>,
    shaded: bool,
    selected: String,
    _subscription: gpui::Subscription,
}

impl MolecularDemo {
    fn new(cx: &mut Context<Self>) -> Self {
        let plot = plot3d_builder(molecule())
            .interactive()
            .fill()
            .on_error(|error| eprintln!("sphere rendering: {error}"))
            .build(cx);
        let subscription = cx.subscribe(&plot, |this, _, event: &Plot3DEvent, cx| {
            if let Plot3DEvent::Pick(hit) = event {
                this.selected = if hit.primitive == PickPrimitive3D::Sphere {
                    format!("Selected atom {}", hit.sources()[0])
                } else {
                    "Click a sphere to select an atom".into()
                };
                cx.notify();
            }
        });
        Self {
            plot,
            shaded: true,
            selected: "Orange: absorber · red: oxygen · blue: metal".into(),
            _subscription: subscription,
        }
    }
}

impl Render for MolecularDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shading = div()
            .id("shading-toggle")
            .px_4()
            .py_2()
            .rounded_md()
            .bg(rgb(0xdce9f5))
            .cursor_pointer()
            .child(if self.shaded {
                "Shading: on"
            } else {
                "Shading: off"
            })
            .on_click(cx.listener(|this, _, _, cx| {
                let next = !this.shaded;
                if this
                    .plot
                    .update(cx, |plot, cx| plot.set_sphere_shading(next, cx))
                    .is_ok()
                {
                    this.shaded = next;
                    cx.notify();
                }
            }));
        let reset = div()
            .id("reset-view")
            .px_4()
            .py_2()
            .rounded_md()
            .bg(rgb(0xe4e8ed))
            .cursor_pointer()
            .child("Reset view")
            .on_click(cx.listener(|this, _, _, cx| {
                let _ = this.plot.update(cx, |plot, cx| plot.reset_view(cx));
            }));
        div()
            .size_full().flex().flex_col().p_4().gap_2()
            .bg(rgb(0xf5f7fa)).text_color(rgb(0x243041))
            .text_size(px(14.0))
            .child(div().flex().gap_4().items_center()
                .child(shading).child(reset).child(self.selected.clone()))
            .child(div().flex_1().min_h_0().child(self.plot.clone()))
            .child("Drag to orbit · right drag to pan · scroll to zoom · faded atoms lie outside 8 Å")
            .child("Illustrative coordination model. Purple and green path arrows use normal depth testing.")
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.0), px(780.0)), cx);
        exit_on_window_open_failure(
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(MolecularDemo::new),
            ),
            "molecular spheres",
        );
        cx.activate(true);
    });
}
