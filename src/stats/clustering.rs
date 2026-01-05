//! Hierarchical clustering
//!
//! Provides linkage computation for dendrograms and clustermaps.

/// Linkage method for hierarchical clustering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkageMethod {
    /// Single linkage (minimum distance)
    Single,
    /// Complete linkage (maximum distance)
    Complete,
    /// Average linkage (UPGMA)
    Average,
    /// Ward's minimum variance method
    Ward,
}

/// Result of hierarchical clustering
#[derive(Debug, Clone)]
pub struct Linkage {
    /// Linkage matrix: each row is [cluster1, cluster2, distance, size]
    pub matrix: Vec<[f64; 4]>,
    /// Optimal leaf ordering
    pub leaves: Vec<usize>,
}

/// Compute hierarchical clustering linkage
///
/// # Arguments
/// * `distance_matrix` - Pairwise distance matrix (symmetric, zero diagonal)
/// * `method` - Linkage method to use
///
/// # Returns
/// Linkage result with matrix and leaf ordering
pub fn linkage(distance_matrix: &[Vec<f64>], method: LinkageMethod) -> Linkage {
    let n = distance_matrix.len();

    if n == 0 {
        return Linkage {
            matrix: vec![],
            leaves: vec![],
        };
    }

    if n == 1 {
        return Linkage {
            matrix: vec![],
            leaves: vec![0],
        };
    }

    // Working copy of distances
    let mut dist = distance_matrix.to_vec();

    // Track cluster sizes and membership
    let mut cluster_size = vec![1usize; n];
    let mut active = vec![true; n];
    let mut linkage_matrix = Vec::with_capacity(n - 1);

    // Next cluster index for merged clusters
    let mut next_cluster = n;

    for _ in 0..(n - 1) {
        // Find minimum distance between active clusters
        let (min_i, min_j, min_dist) = find_min_distance(&dist, &active);

        // Record linkage
        let size = cluster_size[min_i] + cluster_size[min_j];
        linkage_matrix.push([min_i as f64, min_j as f64, min_dist, size as f64]);

        // Update distances to merged cluster
        update_distances(&mut dist, &cluster_size, min_i, min_j, method);

        // Mark j as inactive, update i's size
        active[min_j] = false;
        cluster_size[min_i] = size;

        next_cluster += 1;
    }

    // Compute optimal leaf ordering (simple version: in-order traversal)
    let leaves = compute_leaf_order(&linkage_matrix, n);

    Linkage {
        matrix: linkage_matrix,
        leaves,
    }
}

/// Find minimum distance between active clusters
fn find_min_distance(dist: &[Vec<f64>], active: &[bool]) -> (usize, usize, f64) {
    let n = dist.len();
    let mut min_dist = f64::INFINITY;
    let mut min_i = 0;
    let mut min_j = 1;

    for i in 0..n {
        if !active[i] {
            continue;
        }
        for j in (i + 1)..n {
            if !active[j] {
                continue;
            }
            if dist[i][j] < min_dist {
                min_dist = dist[i][j];
                min_i = i;
                min_j = j;
            }
        }
    }

    (min_i, min_j, min_dist)
}

/// Update distances after merging clusters i and j
fn update_distances(
    dist: &mut [Vec<f64>],
    sizes: &[usize],
    i: usize,
    j: usize,
    method: LinkageMethod,
) {
    let n = dist.len();
    let ni = sizes[i] as f64;
    let nj = sizes[j] as f64;

    for k in 0..n {
        if k == i || k == j {
            continue;
        }

        let d_ik = dist[i.min(k)][i.max(k)];
        let d_jk = dist[j.min(k)][j.max(k)];
        let nk = sizes[k] as f64;

        let new_dist = match method {
            LinkageMethod::Single => d_ik.min(d_jk),
            LinkageMethod::Complete => d_ik.max(d_jk),
            LinkageMethod::Average => (ni * d_ik + nj * d_jk) / (ni + nj),
            LinkageMethod::Ward => {
                let n_total = ni + nj + nk;
                let d_ij = dist[i.min(j)][i.max(j)];
                (((ni + nk) * d_ik + (nj + nk) * d_jk - nk * d_ij) / n_total).sqrt()
            }
        };

        dist[i.min(k)][i.max(k)] = new_dist;
        dist[k.min(i)][k.max(i)] = new_dist;
    }
}

/// Compute leaf order from linkage matrix
fn compute_leaf_order(linkage: &[[f64; 4]], n: usize) -> Vec<usize> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![0];
    }

    // Since the implementation reuses indices, we need to track which original
    // leaves are absorbed into which clusters. For simplicity, return
    // a basic ordering based on the linkage sequence.
    let mut absorbed = vec![false; n];
    let mut order = Vec::with_capacity(n);

    // Process linkage in order - add leaves as they first appear in merges
    for row in linkage {
        let left = row[0] as usize;
        let right = row[1] as usize;

        // Only original indices (< n) are leaves
        if left < n && !absorbed[left] {
            order.push(left);
            absorbed[left] = true;
        }
        if right < n && !absorbed[right] {
            order.push(right);
            absorbed[right] = true;
        }
    }

    // Add any remaining leaves that weren't merged
    for (i, &was_absorbed) in absorbed.iter().enumerate() {
        if !was_absorbed {
            order.push(i);
        }
    }

    order
}

/// Compute pairwise Euclidean distance matrix
pub fn pdist_euclidean(data: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = data.len();
    let mut dist = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in (i + 1)..n {
            let d = euclidean_distance(&data[i], &data[j]);
            dist[i][j] = d;
            dist[j][i] = d;
        }
    }

    dist
}

/// Euclidean distance between two vectors
fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(&ai, &bi)| (ai - bi).powi(2))
        .sum::<f64>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linkage_single() {
        // Simple 3-point example
        let dist = vec![
            vec![0.0, 1.0, 4.0],
            vec![1.0, 0.0, 2.0],
            vec![4.0, 2.0, 0.0],
        ];

        let result = linkage(&dist, LinkageMethod::Single);

        assert_eq!(result.matrix.len(), 2);
        assert_eq!(result.leaves.len(), 3);

        // First merge should be 0,1 (distance 1.0)
        assert!((result.matrix[0][2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_linkage_complete() {
        let dist = vec![
            vec![0.0, 1.0, 4.0],
            vec![1.0, 0.0, 2.0],
            vec![4.0, 2.0, 0.0],
        ];

        let result = linkage(&dist, LinkageMethod::Complete);
        assert_eq!(result.matrix.len(), 2);
    }

    #[test]
    fn test_pdist_euclidean() {
        let data = vec![vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]];

        let dist = pdist_euclidean(&data);

        assert!((dist[0][1] - 1.0).abs() < 1e-10);
        assert!((dist[0][2] - 1.0).abs() < 1e-10);
        assert!((dist[1][2] - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_empty_linkage() {
        let result = linkage(&[], LinkageMethod::Single);
        assert!(result.matrix.is_empty());
        assert!(result.leaves.is_empty());
    }

    #[test]
    fn test_single_point() {
        let dist = vec![vec![0.0]];
        let result = linkage(&dist, LinkageMethod::Single);
        assert!(result.matrix.is_empty());
        assert_eq!(result.leaves, vec![0]);
    }
}
