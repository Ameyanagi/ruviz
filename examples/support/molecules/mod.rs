//! Reconstruct the two 8 Å clusters used by X-Ray Tsubaki (#182).
//! Lattice parameters: COD 9017842 (Mochalov et al., 1998) and COD 1548818
//! (RuO2, primary structure: Boman et al., 1970, doi:10.3891/acta.chem.scand.24-0116).
//! Atoms are generated from the symmetry-expanded fractional basis, centered
//! on Ru, including every center within 8 Å. No atom subsampling is used.

use ruviz::prelude::*;

pub fn cluster(oxide: bool, cutoff: f64) -> Vec<Sphere3D> {
    let u = 0.3058;
    let hcp = [
        (44, [1.0 / 3.0, 2.0 / 3.0, 0.25]),
        (44, [2.0 / 3.0, 1.0 / 3.0, 0.75]),
    ];
    let rutile = [
        (44, [0.0, 0.0, 0.0]),
        (44, [0.5, 0.5, 0.5]),
        (8, [u, u, 0.0]),
        (8, [-u, -u, 0.0]),
        (8, [0.5 + u, 0.5 - u, 0.5]),
        (8, [0.5 - u, 0.5 + u, 0.5]),
    ];
    let (a, c, basis, cos, sin) = if oxide {
        (4.4919, 3.1066, &rutile[..], 0.0, 1.0)
    } else {
        (2.7058, 4.2819, &hcp[..], -0.5, 3.0_f64.sqrt() * 0.5)
    };
    let mut atoms = Vec::new();
    for i in -5..=5 {
        for j in -5..=5 {
            for k in -5..=5 {
                for &(element, f) in basis {
                    let x = f[0] + f64::from(i) - basis[0].1[0];
                    let y = f[1] + f64::from(j) - basis[0].1[1];
                    let z = f[2] + f64::from(k) - basis[0].1[2];
                    let center = Point3D::new(a * x + a * y * cos, a * y * sin, c * z);
                    let distance = center.x * center.x + center.y * center.y + center.z * center.z;
                    if distance <= cutoff * cutoff + 1e-8 {
                        let color = if element == 44 {
                            Color::from_rgb(45, 130, 170)
                        } else {
                            Color::from_rgb(211, 65, 68)
                        };
                        atoms.push(Sphere3D::new(
                            atoms.len() as u32,
                            center,
                            if element == 44 { 0.65 } else { 0.4 },
                            color,
                        ));
                    }
                }
            }
        }
    }
    atoms
}
