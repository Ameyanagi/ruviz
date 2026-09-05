//! Export the issue #182 clusters with analytic spheres and a faint context.
#[path = "support/molecules/mod.rs"]
mod molecules;
use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "generated/examples/spheres".into());
    std::fs::create_dir_all(&output)?;
    for (name, oxide) in [("ru-hcp", false), ("ruo2", true)] {
        let mut atoms = molecules::cluster(oxide, 10.0);
        for atom in &mut atoms {
            let p = atom.center;
            if p.x * p.x + p.y * p.y + p.z * p.z > 64.0 {
                atom.color = atom.color.with_alpha(0.15);
            }
            if p.x * p.x + p.y * p.y + p.z * p.z < 1e-8 {
                atom.color = Color::ORANGE;
            }
        }
        let p = spheres3d(&atoms)
            .axes(false)
            .title(format!("{name}: 8 Å cluster with faded context"))
            .xlabel("x (Å)")
            .ylabel("y (Å)")
            .zlabel("z (Å)")
            .size_px(900, 700)
            .dpi(96);
        p.clone().save(format!("{output}/{name}-shaded.png"))?;
        p.clone()
            .shading(false)
            .save(format!("{output}/{name}-unlit.png"))?;
        p.clone().save(format!("{output}/{name}.svg"))?;
        #[cfg(feature = "gpu")]
        p.save_gpu(format!("{output}/{name}-gpu.png"))?;
    }
    Ok(())
}
