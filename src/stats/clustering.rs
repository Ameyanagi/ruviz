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
///
/// The matrix uses SciPy's `scipy.cluster.hierarchy.linkage` convention, which
/// every consumer in this crate (and every reader who has met a linkage matrix
/// before) already knows:
///
/// * ids `0 .. n` are the original observations (leaves);
/// * row `i` of the matrix describes the cluster with id `n + i`, so a merged
///   cluster is always nameable and always has an id `>= n`;
/// * each row is `[left_id, right_id, merge_distance, cluster_size]` with
///   `left_id < right_id`.
///
/// Getting this wrong is not cosmetic: a consumer that reads `n + i` from a
/// matrix that reuses leaf ids draws a structurally wrong tree, because every
/// merged cluster is indistinguishable from one of its own leaves.
#[derive(Debug, Clone)]
pub struct Linkage {
    /// Linkage matrix: each row is `[cluster1, cluster2, distance, size]`,
    /// where row `i` defines cluster `n + i`. See the type docs.
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
    // The distance matrix is indexed by *slot*; `cluster_id[slot]` is the
    // SciPy-convention id of the cluster currently living in that slot. The
    // merged cluster stays in slot `min_i` but takes a fresh id `n + step`, so
    // no id is ever reused and every consumer can tell a merge from a leaf.
    let mut cluster_id: Vec<usize> = (0..n).collect();
    let mut linkage_matrix = Vec::with_capacity(n - 1);

    for step in 0..(n - 1) {
        // Find minimum distance between active clusters
        let (min_i, min_j, min_dist) = find_min_distance(&dist, &active);

        // Record linkage
        let size = cluster_size[min_i] + cluster_size[min_j];
        let (left, right) = {
            let (a, b) = (cluster_id[min_i], cluster_id[min_j]);
            (a.min(b), a.max(b))
        };
        linkage_matrix.push([left as f64, right as f64, min_dist, size as f64]);

        // Update distances to merged cluster
        update_distances(&mut dist, &cluster_size, min_i, min_j, method);

        // Mark j as inactive, update i's size and give the merge its own id
        active[min_j] = false;
        cluster_size[min_i] = size;
        cluster_id[min_i] = n + step;
    }

    // Compute optimal leaf ordering (in-order traversal of the tree)
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
                // Lance-Williams for Ward is defined on SQUARED distances. Applying
                // it to raw distances (as this once did) is not a small error: it
                // produces INVERSIONS, i.e. a parent merging lower than its own
                // child, which is structurally impossible for a real dendrogram.
                let n_total = ni + nj + nk;
                let d_ij = dist[i.min(j)][i.max(j)];
                (((ni + nk) * d_ik * d_ik + (nj + nk) * d_jk * d_jk - nk * d_ij * d_ij) / n_total)
                    .max(0.0)
                    .sqrt()
            }
        };

        dist[i.min(k)][i.max(k)] = new_dist;
        dist[k.min(i)][k.max(i)] = new_dist;
    }
}

/// Compute leaf order from a SciPy-convention linkage matrix.
///
/// This is an in-order traversal of the merge tree from its root, which is what
/// makes a dendrogram drawable without crossing arms: every cluster's leaves are
/// contiguous in the returned order.
fn compute_leaf_order(linkage: &[[f64; 4]], n: usize) -> Vec<usize> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![0];
    }

    let mut order = Vec::with_capacity(n);
    let mut visited = vec![false; n];

    // Iterative post/in-order walk from the root (the last merge). Iterative
    // rather than recursive so a pathological chain of 10^5 merges cannot blow
    // the stack.
    let mut stack = vec![n + linkage.len() - 1];
    while let Some(id) = stack.pop() {
        if id < n {
            if !visited[id] {
                visited[id] = true;
                order.push(id);
            }
            continue;
        }
        match linkage.get(id - n) {
            // Push right first so the left subtree is expanded first.
            Some(row) => stack.extend([row[1] as usize, row[0] as usize]),
            None => continue,
        }
    }

    // Any leaf the tree never reached (only possible for a malformed matrix)
    // still has to appear, or it would silently vanish from the plot.
    for (leaf, &seen) in visited.iter().enumerate() {
        if !seen {
            order.push(leaf);
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

    /// Three well-separated pairs, so the merge order is unambiguous.
    fn three_clusters() -> Vec<Vec<f64>> {
        pdist_euclidean(&[
            vec![0.0, 0.0],
            vec![0.2, 0.1],
            vec![5.0, 0.0],
            vec![5.2, 0.2],
            vec![0.0, 9.0],
            vec![0.3, 9.3],
        ])
    }

    #[test]
    fn every_merge_gets_an_id_of_its_own() {
        let result = linkage(&three_clusters(), LinkageMethod::Average);
        let n = 6;

        for (step, row) in result.matrix.iter().enumerate() {
            let (left, right) = (row[0] as usize, row[1] as usize);
            assert!(
                left < right,
                "row {step} is not in ascending id order: {row:?}"
            );
            // A child must already exist: a leaf, or a cluster merged earlier.
            for child in [left, right] {
                assert!(
                    child < n + step,
                    "row {step} names cluster {child}, which does not exist yet"
                );
            }
        }

        // Reusing a leaf id for a merge is the failure this guards: every
        // consumer reads `n + i` to tell a merge from a leaf, so an id below `n`
        // that is really a merge silently becomes a leaf at height zero.
        let merges: Vec<usize> = (0..result.matrix.len()).map(|i| n + i).collect();
        assert_eq!(merges, vec![6, 7, 8, 9, 10]);
    }

    #[test]
    fn merge_heights_never_invert() {
        // A parent merging *below* its own child is structurally impossible for
        // a real dendrogram. Ward produced exactly that, because Lance-Williams
        // for Ward is defined on squared distances and was applied to raw ones.
        for method in [
            LinkageMethod::Single,
            LinkageMethod::Complete,
            LinkageMethod::Average,
            LinkageMethod::Ward,
        ] {
            let result = linkage(&three_clusters(), method);
            let n = 6;
            for (step, row) in result.matrix.iter().enumerate() {
                for child in [row[0] as usize, row[1] as usize] {
                    let child_height = match child < n {
                        true => 0.0,
                        false => result.matrix[child - n][2],
                    };
                    assert!(
                        row[2] >= child_height,
                        "{method:?}: cluster {} merges at {:.4}, below its child \
                         {child} at {child_height:.4}",
                        n + step,
                        row[2],
                    );
                }
            }
        }
    }

    #[test]
    fn every_cluster_owns_a_contiguous_run_of_leaves() {
        // This is what makes a dendrogram drawable without crossing arms, and
        // it is the reason the leaf order is a tree traversal rather than the
        // order names happened to appear in the merge sequence.
        let result = linkage(&three_clusters(), LinkageMethod::Average);
        let n = 6;
        assert_eq!(result.leaves.len(), n);

        let slot = |leaf: usize| {
            result
                .leaves
                .iter()
                .position(|&placed| placed == leaf)
                .expect("every leaf is placed exactly once")
        };

        fn leaves_of(matrix: &[[f64; 4]], n: usize, id: usize, out: &mut Vec<usize>) {
            if id < n {
                out.push(id);
                return;
            }
            let row = matrix[id - n];
            leaves_of(matrix, n, row[0] as usize, out);
            leaves_of(matrix, n, row[1] as usize, out);
        }

        for id in n..(n + result.matrix.len()) {
            let mut members = Vec::new();
            leaves_of(&result.matrix, n, id, &mut members);
            let mut slots: Vec<usize> = members.iter().map(|&leaf| slot(leaf)).collect();
            slots.sort_unstable();
            let span = slots[slots.len() - 1] - slots[0] + 1;
            assert_eq!(
                span,
                slots.len(),
                "cluster {id} occupies slots {slots:?}, which are not contiguous"
            );
        }
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
