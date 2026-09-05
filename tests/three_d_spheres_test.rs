#![cfg(feature = "3d")]

use ruviz::core::Image;
use ruviz::prelude::*;

fn atom(id: u32, center: [f64; 3], radius: f64, color: Color) -> Sphere3D {
    Sphere3D::new(
        id,
        Point3D::new(center[0], center[1], center[2]),
        radius,
        color,
    )
}

fn plot(atoms: &[Sphere3D]) -> Spheres3DBuilder {
    spheres3d(atoms)
        .camera(
            Camera3D::default()
                .camera_view(CameraView3D::Front)
                .axis_aspect(AxisAspect3D::Data),
        )
        .xlim(-2.0, 2.0)
        .ylim(-2.0, 2.0)
        .zlim(-2.0, 2.0)
        .size_px(400, 320)
        .dpi(96)
}

fn pixel(image: &Image, position: (f32, f32)) -> [u8; 3] {
    let offset = ((position.1 as u32 * image.width + position.0 as u32) * 4) as usize;
    image.pixels[offset..offset + 3].try_into().unwrap()
}

#[test]
fn validates_identity_radius_position_and_lighting_at_terminals() {
    let valid = atom(7, [0.0; 3], 0.4, Color::RED);
    let bounds = spheres3d(&[valid]).data_bounds().unwrap();
    assert_eq!(bounds.min, Point3D::new(-0.4, -0.4, -0.4));
    assert_eq!(bounds.max, Point3D::new(0.4, 0.4, 0.4));
    for radius in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            spheres3d(&[Sphere3D { radius, ..valid }])
                .validate()
                .is_err()
        );
    }
    assert!(spheres3d(&[]).validate().is_err());
    assert!(
        spheres3d(&[valid, valid])
            .validate()
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
    assert!(
        spheres3d(&[Sphere3D {
            center: Point3D::new(f64::NAN, 0.0, 0.0),
            ..valid
        }])
        .validate()
        .is_err()
    );
    assert!(spheres3d(&[valid]).specular(1.1, 32.0).validate().is_err());
    assert!(spheres3d(&[valid]).specular(0.1, 0.0).validate().is_err());
    // Alpha belongs to the new primitive; existing marker validation is unchanged.
    spheres3d(&[Sphere3D {
        color: Color::RED.with_alpha(0.2),
        ..valid
    }])
    .validate()
    .unwrap();
    assert!(
        scatter3d(&[0.0], &[0.0], &[0.0])
            .color(Color::RED.with_alpha(0.2))
            .validate()
            .is_err()
    );
}

#[test]
fn data_aspect_keeps_radii_round_and_picking_uses_the_exact_silhouette() {
    let atoms = [atom(5, [0.0; 3], 0.5, Color::RED)];
    let p = plot(&atoms).xlim(-3.0, 3.0).zlim(-1.0, 1.0);
    let center = p.clone().project(atoms[0].center).unwrap().unwrap();
    let x = p
        .clone()
        .project(Point3D::new(0.5, 0.0, 0.0))
        .unwrap()
        .unwrap();
    let z = p
        .clone()
        .project(Point3D::new(0.0, 0.0, 0.5))
        .unwrap()
        .unwrap();
    assert!(((x.0 - center.0).abs() - (z.1 - center.1).abs()).abs() < 0.01);
    let outside = p
        .clone()
        .project(Point3D::new(0.51, 0.0, 0.0))
        .unwrap()
        .unwrap();
    assert!(p.clone().pick(outside.0, outside.1).unwrap().is_none());
    let inside = p
        .clone()
        .project(Point3D::new(0.49, 0.0, 0.0))
        .unwrap()
        .unwrap();
    assert_eq!(p.pick(inside.0, inside.1).unwrap().unwrap().sources(), &[5]);
}

#[test]
fn nearer_surface_wins_even_when_its_center_is_farther_away() {
    let atoms = [
        atom(91, [0.0, 0.3, 0.0], 1.0, Color::RED),
        atom(23, [0.45, 0.0, 0.0], 0.3, Color::BLUE),
    ];
    let p = plot(&atoms).shading(false);
    let position = p.clone().project(atoms[1].center).unwrap().unwrap();
    let hit = p.clone().pick(position.0, position.1).unwrap().unwrap();
    assert_eq!(hit.primitive, PickPrimitive3D::Sphere);
    assert_eq!(hit.sources(), &[91]);
    assert!(hit.point.y < -0.5);
    assert_eq!(
        pixel(&p.render().unwrap(), position),
        [Color::RED.r, Color::RED.g, Color::RED.b]
    );
    let reversed = plot(&[atoms[1], atoms[0]]).shading(false);
    assert_eq!(
        reversed
            .pick(position.0, position.1)
            .unwrap()
            .unwrap()
            .sources(),
        &[91]
    );
}

#[test]
fn bonds_occlude_at_the_sphere_surface_and_picking_agrees() {
    let atoms = [atom(5, [0.0; 3], 1.0, Color::RED)];
    for (y, expected) in [
        (-0.5, PickPrimitive3D::Sphere),
        (-1.2, PickPrimitive3D::LineSegment),
    ] {
        let p = plot(&atoms)
            .shading(false)
            .line3d(&[-1.4, 1.4], &[y, y], &[0.0, 0.0])
            .color(Color::BLUE)
            .line_width(5.0);
        let position = p
            .clone()
            .project(Point3D::new(0.0, 0.0, 0.0))
            .unwrap()
            .unwrap();
        assert_eq!(
            p.clone()
                .pick(position.0, position.1)
                .unwrap()
                .unwrap()
                .primitive,
            expected
        );
        let color = pixel(&p.render().unwrap(), position);
        if expected == PickPrimitive3D::Sphere {
            assert!(color[0] > color[2]);
        } else {
            assert!(color[2] > color[0]);
        }
    }
}

#[test]
fn sphere_and_surface_depth_compose() {
    let atoms = [atom(5, [0.0; 3], 1.0, Color::RED)];
    // Camera from +x, nearly in the xy plane. A z=0 surface passes through the ball.
    let p = plot(&atoms)
        .shading(false)
        .camera(
            Camera3D::default()
                .elevation_deg(80.0)
                .axis_aspect(AxisAspect3D::Data),
        )
        .surface(&[-1.5, 1.5], &[-1.5, 1.5], &[[0.0, 0.0], [0.0, 0.0]])
        .color(Color::BLUE)
        .colorbar(false);
    let center = p
        .clone()
        .project(Point3D::new(0.0, 0.0, 0.0))
        .unwrap()
        .unwrap();
    assert_eq!(
        p.clone()
            .pick(center.0, center.1)
            .unwrap()
            .unwrap()
            .primitive,
        PickPrimitive3D::Sphere
    );
    let color = pixel(&p.render().unwrap(), center);
    assert!(color[0] > color[2]);
}

#[test]
fn transparency_respects_opaque_depth_and_nearly_invisible_atoms_do_not_pick() {
    for alpha in [0.0, 0.04, 0.5] {
        let atoms = [
            atom(1, [0.0, 0.5, 0.0], 0.5, Color::RED),
            atom(2, [0.0, -0.7, 0.0], 0.5, Color::BLUE.with_alpha(alpha)),
        ];
        let p = plot(&atoms).shading(false);
        let center = p.clone().project(atoms[0].center).unwrap().unwrap();
        let hit = p.clone().pick(center.0, center.1).unwrap().unwrap();
        assert_eq!(hit.sources(), if alpha <= 0.04 { &[1] } else { &[2] });
        let image = p.clone().render().unwrap();
        if alpha == 0.5 {
            let color = pixel(&image, center);
            assert!(
                color[0] > 70 && color[2] > 70,
                "both layers must contribute: {color:?}"
            );
        }
        let reversed = plot(&[atoms[1], atoms[0]]).shading(false).render().unwrap();
        assert_eq!(image.pixels, reversed.pixels);
    }
}

#[test]
fn shading_toggle_preserves_camera_selection_geometry_and_supersedes_old_frames() {
    let atoms = [atom(100, [0.0; 3], 1.0, Color::RED)];
    let p = plot(&atoms);
    let center = p.clone().project(atoms[0].center).unwrap().unwrap();
    let mut session = p.interactive_session().unwrap();
    session.pick(center.0, center.1).unwrap();
    let selected = session.current_pick().unwrap().hit;
    let camera = session.camera();
    let old_job = session.background_render_job().unwrap();
    let before = session.render().unwrap();
    assert!(session.set_sphere_shading(false).unwrap());
    assert!(!session.set_sphere_shading(false).unwrap());
    assert_eq!(session.camera(), camera);
    let current = session.current_pick().unwrap();
    assert_eq!(selected.sources(), current.hit.sources());
    assert_eq!(selected.point, current.hit.point);
    assert!(session.is_stamped_pick_current(&current));
    assert!(!session.is_render_current(old_job.stamp()));
    let after = session.render().unwrap();
    assert_ne!(before.pixels, after.pixels);
    assert!(session.set_sphere_shading(true).unwrap());
    assert_eq!(before.pixels, session.render().unwrap().pixels);
}

#[test]
fn molecular_view_hides_axes_but_preserves_title_legend_and_picking() {
    let atoms = [atom(100, [0.0; 3], 1.0, Color::RED)];
    let p = plot(&atoms)
        .axes(false)
        .xlabel("hidden axis label")
        .title("Visible molecule title")
        .label("Absorber");
    let center = p.clone().project(atoms[0].center).unwrap().unwrap();
    assert_eq!(
        p.clone()
            .pick(center.0, center.1)
            .unwrap()
            .unwrap()
            .sources(),
        &[100]
    );
    let svg = p.render_to_svg().unwrap();
    assert!(!svg.contains("hidden axis label"));
    assert!(svg.contains("Visible molecule title"));
    assert!(svg.contains("Absorber"));
}

#[cfg(feature = "gpu")]
#[test]
fn gpu_spheres_match_software_through_orbit_and_reuse_buffers_on_toggle() {
    let atoms = [
        atom(100, [-0.3, 0.2, 0.0], 0.9, Color::RED),
        atom(200, [0.5, -0.2, 0.2], 0.5, Color::BLUE),
        atom(300, [-0.5, -1.2, 0.5], 0.4, Color::GREEN.with_alpha(0.35)),
    ];
    for perspective in [false, true] {
        for azimuth in [-90.0, 15.0, 130.0] {
            let mut camera = Camera3D::default()
                .azimuth_deg(azimuth)
                .axis_aspect(AxisAspect3D::Data);
            if perspective {
                camera = camera.perspective_deg(45.0);
            }
            let p = plot(&atoms)
                .camera(camera)
                .line3d(&[-1.5, 1.5], &[0.0, 0.0], &[0.0, 0.0])
                .color(Color::BLACK)
                .line_width(2.0);
            let cpu = p.clone().render().unwrap();
            let gpu = p
                .render_gpu()
                .expect("GPU adapter required for sphere validation");
            let mean = cpu
                .pixels
                .iter()
                .zip(&gpu.pixels)
                .map(|(a, b)| u64::from(a.abs_diff(*b)))
                .sum::<u64>() as f64
                / cpu.pixels.len() as f64;
            assert!(
                mean < 1.5,
                "CPU/GPU mean difference {mean}, perspective={perspective}, azimuth={azimuth}"
            );
        }
    }
    let mut session = plot(&atoms).interactive_session().unwrap();
    let (_, first) = session.render_gpu_readback().unwrap();
    assert!(first.vertex_upload_bytes > 0);
    session.set_sphere_shading(false).unwrap();
    let (_, toggled) = session.render_gpu_readback().unwrap();
    assert_eq!(toggled.vertex_upload_bytes, 0);
    assert_eq!(toggled.index_upload_bytes, 0);
}

#[test]
fn fixed_ratio_preserves_geometry_through_orbit_zoom_shading_and_reset() {
    let atoms = [atom(0, [0.0; 3], 0.4, Color::RED)];
    let aspect = AxisAspect3D::fixed(2.0, 3.0, 1.0);
    let p = plot(&atoms)
        .axes(false)
        .axis_aspect(aspect)
        .stable_scale(true);
    let c = p
        .clone()
        .project(Point3D::new(0.0, 0.0, 0.0))
        .unwrap()
        .unwrap();
    let x = p
        .clone()
        .project(Point3D::new(0.5, 0.0, 0.0))
        .unwrap()
        .unwrap();
    let z = p
        .clone()
        .project(Point3D::new(0.0, 0.0, 0.5))
        .unwrap()
        .unwrap();
    assert!(((x.0 - c.0).abs() / (z.1 - c.1).abs() - 2.0).abs() < 0.001);
    let mut session = p.interactive_session().unwrap();
    let home = session.camera();
    session
        .set_camera(home.azimuth_deg(35.0).elevation_deg(40.0).zoom(1.5))
        .unwrap();
    let camera = session.camera();
    session.set_sphere_shading(false).unwrap();
    assert_eq!(session.camera(), camera);
    assert_eq!(camera.axis_aspect_value(), aspect);
    assert!(camera.has_stable_scale());
    session.reset_view().unwrap();
    assert_eq!(session.camera(), home);
    assert!(
        spheres3d(&atoms)
            .interactive_session()
            .unwrap()
            .camera()
            .has_stable_scale()
    );
}
