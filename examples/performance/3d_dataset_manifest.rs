//! Print the deterministic dataset hashes used by the 3d benchmark contract.

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Copy)]
struct StableHash(u64);

impl StableHash {
    fn new() -> Self {
        Self(FNV_OFFSET)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(FNV_PRIME);
        }
    }

    fn usize(&mut self, value: usize) {
        self.bytes(&(value as u64).to_le_bytes());
    }

    fn f64(&mut self, value: f64) {
        self.bytes(&value.to_bits().to_le_bytes());
    }

    fn finish(self) -> String {
        format!("fnv1a64:{:016x}", self.0)
    }
}

fn scatter_hash(size: usize) -> String {
    let denominator = size.max(1) as f64;
    let mut hash = StableHash::new();
    hash.usize(size);
    for index in 0..size {
        hash.f64(index as f64 / denominator);
    }
    for index in 0..size {
        let value = index as f64 / denominator;
        hash.f64((value * 31.0).sin() + (index % 17) as f64 * 0.002);
    }
    for index in 0..size {
        let value = index as f64 / denominator;
        hash.f64((value * 23.0).cos() + (index % 11) as f64 * 0.003);
    }
    hash.finish()
}

fn surface_hash(side: usize) -> String {
    let denominator = side.saturating_sub(1).max(1) as f64;
    let mut hash = StableHash::new();
    hash.usize(side);
    for _ in 0..2 {
        for index in 0..side {
            hash.f64(-3.0 + 6.0 * index as f64 / denominator);
        }
    }
    for row in 0..side {
        let y = -3.0 + 6.0 * row as f64 / denominator;
        for column in 0..side {
            let x = -3.0 + 6.0 * column as f64 / denominator;
            let radius = x.hypot(y);
            hash.f64(if radius == 0.0 {
                1.0
            } else {
                radius.sin() / radius
            });
        }
    }
    hash.finish()
}

fn main() {
    println!("{{");
    println!("  \"schema_version\": 1,");
    println!("  \"hash\": \"fnv1a64 over little-endian f64 bits\",");
    println!("  \"datasets\": [");
    let mut rows = Vec::new();
    for size in [100_000, 1_000_000, 10_000_000] {
        rows.push(format!(
            "    {{\"id\":\"scatter-wave-{size}\",\"kind\":\"scatter\",\"elements\":{size},\"hash\":\"{}\"}}",
            scatter_hash(size)
        ));
    }
    for side in [100_usize, 512, 1024] {
        rows.push(format!(
            "    {{\"id\":\"surface-sinc-{side}\",\"kind\":\"surface\",\"rows\":{side},\"columns\":{side},\"triangles\":{},\"hash\":\"{}\"}}",
            side.saturating_sub(1).pow(2).saturating_mul(2),
            surface_hash(side)
        ));
    }
    println!("{}", rows.join(",\n"));
    println!("  ]");
    println!("}}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_stable_and_size_sensitive() {
        assert_eq!(scatter_hash(0), "fnv1a64:a8c7f832281a39c5");
        assert_ne!(scatter_hash(10), scatter_hash(11));
        assert_ne!(surface_hash(10), surface_hash(11));
    }

    #[test]
    fn committed_manifest_matches_reference_cases() {
        let manifest = include_str!("../../docs/benchmarks/ruviz-3d-datasets.json");
        for size in [100_000, 1_000_000, 10_000_000] {
            assert!(manifest.contains(&scatter_hash(size)));
        }
        for side in [100, 512, 1024] {
            assert!(manifest.contains(&surface_hash(side)));
        }
        assert!(manifest.contains("\"feature\": \"3d\""));
    }
}
