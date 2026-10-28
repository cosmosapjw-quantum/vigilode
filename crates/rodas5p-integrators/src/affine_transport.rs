//! Parallel, outward prefix transport of nonnegative error bounds.
//!
//! A leaf `(g, d)` represents the caller's inequality `e_next <= g*e + d`.
//! This module verifies its arithmetic, not the caller's ODE/model premise.
//! Up/down sweeps use O(n) work and O(log n) parallel span. Rounded composition
//! need not be associative: every tree node encloses its exact composition.

use rayon::join;
use rodas5p_core::directed::{add_up, mul_up};
use rodas5p_core::{CoreError, CoreResult};

/// An immutable monotone affine majorant on nonnegative real numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AffineErrorMap {
    gain: f64,
    defect: f64,
}

impl AffineErrorMap {
    pub fn new(gain: f64, defect: f64) -> CoreResult<Self> {
        if !gain.is_finite() || gain < 0.0 || !defect.is_finite() || defect < 0.0 {
            return Err(CoreError::InvalidInput(
                "affine transport requires finite nonnegative bounds".into(),
            ));
        }
        Ok(Self { gain, defect })
    }

    pub fn gain(self) -> f64 {
        self.gain
    }
    pub fn defect(self) -> f64 {
        self.defect
    }

    pub fn apply(self, radius: f64) -> CoreResult<f64> {
        if !radius.is_finite() || radius < 0.0 {
            return Err(CoreError::InvalidInput(
                "invalid initial error radius".into(),
            ));
        }
        add_up(mul_up(self.gain, radius)?, self.defect)
    }

    /// Return an outward enclosure of `self(earlier(e))` for every `e >= 0`.
    pub fn after(self, earlier: Self) -> CoreResult<Self> {
        Ok(Self {
            gain: mul_up(self.gain, earlier.gain)?,
            defect: add_up(mul_up(self.gain, earlier.defect)?, self.defect)?,
        })
    }

    const IDENTITY: Self = Self {
        gain: 1.0,
        defect: 0.0,
    };
}

/// Successful result: the initial radius followed by one bound per leaf.
/// Counts describe this algorithm, not allocator, runtime or ODE work.
#[derive(Debug)]
pub struct AffinePrefixBounds {
    radii: Vec<f64>,
    leaves: usize,
    compositions: usize,
    sweep_levels: u32,
}

impl AffinePrefixBounds {
    pub fn radii(&self) -> &[f64] {
        &self.radii
    }
    pub fn leaves(&self) -> usize {
        self.leaves
    }
    pub fn compositions(&self) -> usize {
        self.compositions
    }
    pub fn sweep_levels(&self) -> u32 {
        self.sweep_levels
    }
}

fn storage<T: Clone>(len: usize, value: T) -> CoreResult<Vec<T>> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(len)
        .map_err(|_| CoreError::InvalidInput("affine transport allocation refused".into()))?;
    result.resize(len, value);
    Ok(result)
}

/// Reusable storage: setup is O(n) serial work. After setup, `evaluate` has
/// O(n) work and O(log n) fork-join span, including all leaf writes. No claim
/// about measured scheduler/allocator costs is implicit in these bounds.
#[derive(Debug)]
pub struct AffinePrefixWorkspace {
    tree: Vec<AffineErrorMap>,
    radii: Vec<f64>,
    n: usize,
}

impl AffinePrefixWorkspace {
    pub fn new(n: usize) -> CoreResult<Self> {
        let size = n
            .max(1)
            .checked_mul(2)
            .and_then(|x| x.checked_sub(1))
            .ok_or_else(|| CoreError::InvalidInput("affine transport dimension overflow".into()))?;
        Ok(Self {
            tree: storage(size, AffineErrorMap::IDENTITY)?,
            radii: storage(n + 1, 0.0)?,
            n,
        })
    }

    /// Return a view only after all prefixes succeed. A failed call exposes no
    /// partial result. The input majorants, not this arithmetic helper, must
    /// establish the ODE error recurrence.
    pub fn evaluate<'a>(
        &'a mut self,
        maps: &[AffineErrorMap],
        initial_radius: f64,
    ) -> CoreResult<&'a [f64]> {
        if maps.len() != self.n {
            return Err(CoreError::Dimension("affine transport workspace".into()));
        }
        if !initial_radius.is_finite() || initial_radius < 0.0 {
            return Err(CoreError::InvalidInput(
                "invalid initial error radius".into(),
            ));
        }
        self.radii[0] = initial_radius;
        if self.n > 0 {
            build(maps, &mut self.tree)?;
            distribute(
                &self.tree,
                AffineErrorMap::IDENTITY,
                initial_radius,
                &mut self.radii[1..],
            )?;
        }
        Ok(&self.radii)
    }
}

// Preorder complete binary tree: a subtree with n leaves has 2n-1 slots.
// Each recursive child owns a disjoint contiguous slice, avoiding locks.
fn build(maps: &[AffineErrorMap], tree: &mut [AffineErrorMap]) -> CoreResult<()> {
    if maps.len() == 1 {
        tree[0] = maps[0];
        return Ok(());
    }
    let mid = maps.len() / 2;
    let (root, rest) = tree.split_at_mut(1);
    let (left, right) = rest.split_at_mut(2 * mid - 1);
    let (a, b) = join(|| build(&maps[..mid], left), || build(&maps[mid..], right));
    a?;
    b?;
    root[0] = right[0].after(left[0])?;
    Ok(())
}

fn distribute(
    tree: &[AffineErrorMap],
    prefix: AffineErrorMap,
    e0: f64,
    out: &mut [f64],
) -> CoreResult<()> {
    if out.len() == 1 {
        out[0] = tree[0].after(prefix)?.apply(e0)?;
        return Ok(());
    }
    let mid = out.len() / 2;
    let (left, right) = tree[1..].split_at(2 * mid - 1);
    let right_prefix = left[0].after(prefix)?;
    let (lo, hi) = out.split_at_mut(mid);
    let (a, b) = join(
        || distribute(left, prefix, e0, lo),
        || distribute(right, right_prefix, e0, hi),
    );
    a?;
    b?;
    Ok(())
}

/// Allocating convenience wrapper. Setup initialization is serial O(n); use
/// `AffinePrefixWorkspace::evaluate` to amortize setup over repeated scans.
/// Successful values are independent of Rayon thread count.
pub fn parallel_error_prefix(
    maps: &[AffineErrorMap],
    initial_radius: f64,
) -> CoreResult<AffinePrefixBounds> {
    let mut workspace = AffinePrefixWorkspace::new(maps.len())?;
    workspace.evaluate(maps, initial_radius)?;
    let n = maps.len();
    let levels = if n <= 1 {
        0
    } else {
        usize::BITS - (n - 1).leading_zeros()
    };
    Ok(AffinePrefixBounds {
        radii: workspace.radii,
        leaves: n,
        compositions: if n == 0 { 0 } else { 3 * n - 2 },
        sweep_levels: 2 * levels,
    })
}
