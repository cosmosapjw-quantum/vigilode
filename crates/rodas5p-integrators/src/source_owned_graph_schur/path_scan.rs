//! Ordered inclusive prefixes of affine maps on the spatial path of one W stage.
use rayon::prelude::*;

/// Path solver used for the eliminated forest of the source-owned graph-Schur client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphPathScan {
    /// Original serial Hillis-Steele evaluator, with a snapshot per stride.
    HillisSteele,
    /// Spatial forward substitution with one scalar recurrence per edge.
    Sequential,
    /// Work-efficient inclusive up/down tree, with independent blocks assigned to Rayon.
    Blelloch,
}

type Affine = (f64, f64);

// Apply the left segment first. Composition is associative over reals but not commutative.
fn then(left: Affine, right: Affine) -> Affine {
    (right.0 * left.0, right.1 + right.0 * left.1)
}

pub(super) fn hillis_steele(pairs: &mut [Affine]) -> u64 {
    let mut work = 0;
    let mut stride = 1;
    while stride < pairs.len() {
        let previous = pairs.to_vec();
        for i in stride..pairs.len() {
            pairs[i] = then(previous[i - stride], previous[i]);
            work += 1;
        }
        stride *= 2;
    }
    work
}

fn combine_level(pairs: &mut [Affine], stride: usize) -> u64 {
    let width = 2 * stride;
    let count = pairs.len() / width;
    let combine = |block: &mut [Affine]| {
        block[width - 1] = then(block[stride - 1], block[width - 1]);
    };
    // Small tree levels have less work than scheduling one extra Rayon job.
    if count >= 16 {
        pairs
            .par_chunks_exact_mut(width)
            .with_min_len(8)
            .for_each(combine);
    } else {
        pairs.chunks_exact_mut(width).for_each(combine);
    }
    count as u64
}

/// Inclusive Blelloch-family (Brent–Kung) network. Each up-level reduces adjacent
/// blocks. The down-level fills missing prefixes between completed longer prefixes.
/// Incomplete terminal blocks perform no operation, as if padded by identities and
/// then pruned. For m leaves the number of compositions is <2m and depth <=2 ceil(log2 m).
pub(super) fn blelloch(pairs: &mut [Affine]) -> u64 {
    let mut work = 0;
    let mut stride = 1;
    while stride <= pairs.len() / 2 {
        work += combine_level(pairs, stride);
        stride *= 2;
    }
    while stride > 1 {
        stride /= 2;
        work += combine_level(&mut pairs[stride..], stride);
    }
    work
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_noncommuting_prefixes_include_ragged_tails() {
        for n in [0, 1, 2, 3, 7, 8, 9, 31, 32, 33, 47, 63, 95, 192, 257] {
            let raw: Vec<Affine> = (0..n)
                .map(|i| ([-1.0, 0.5, -0.25, 1.0][i % 4], ((7 * i) % 13) as f64 - 6.0))
                .collect();
            let mut expected = (1.0, 0.0);
            let mut actual = raw.clone();
            let work = blelloch(&mut actual);
            assert!(work <= 2 * n as u64);
            for (edge, prefix) in raw.into_iter().zip(actual) {
                expected = then(expected, edge);
                assert!((prefix.0 - expected.0).abs() < 1e-14);
                assert!((prefix.1 - expected.1).abs() < 1e-13);
            }
        }
    }
}
