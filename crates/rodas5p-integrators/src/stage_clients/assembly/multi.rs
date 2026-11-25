//! The further colourings of the multi-colouring check of
//! [`super::AssemblyStageClient`] (loop node V26; frozen construction in
//! `research/production_loop_20261116/assembly_check/PREDECLARED.json`,
//! `mc_construction`; the claim and its proof in `NOTES.txt` of that
//! directory).
//!
//! The client's own colouring `c1` is the Curtis-Powell-Reid colouring of the
//! declared pattern ([`super::colour_columns`]). The check adds colourings
//! `c_1 .. c_R` of the same pattern, each valid (no two columns of a group
//! share a declared row), chosen so that the tuple
//! `(c1(j), c_1(j), .., c_R(j))` differs for every two columns. There are two
//! constructions, and the one with the smaller total number of non-empty
//! groups is used (equal totals: the band construction):
//!
//! - band construction: with `lower` and `upper` the largest distances below
//!   and above the diagonal among the declared entries,
//!   `c_r(j) = j mod m_r`, where `m_1 < m_2 < ..` are the smallest integers of
//!   at least `lower + upper + 1` that are pairwise coprime, taken until their
//!   product is at least `n`. Two columns that share a row are at most
//!   `lower + upper` apart, so every `c_r` is valid; the Chinese remainder
//!   theorem makes the residues alone identify every column;
//! - refinement construction: `c_r(j) = (c1(j), q(j) mod p_r)` with `q(j)` the
//!   rank of column `j` inside its `c1` class and `p_r` the primes
//!   `2, 3, 5, ..` until their product is at least the size of the largest
//!   `c1` class. Every `c_r` refines `c1` and is therefore valid; `c1` and the
//!   residues of `q(j)` identify every column.
//!
//! The tuple map is checked to be injective when the colourings are built; a
//! failure would be a defect of the construction and is returned as
//! [`AssemblyError::Configuration`].

use std::cmp::Ordering;
use std::iter::StepBy;
use std::ops::Range;
use std::slice::Iter;

use super::AssemblyError;
use super::pattern::{SparsityPattern, colour_columns};

/// The construction of the further colourings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckConstruction {
    /// `c_r(j) = j mod moduli[r]`.
    Band {
        lower: usize,
        upper: usize,
        moduli: Vec<usize>,
    },
    /// `c_r(j) = (c1(j), q(j) mod primes[r])`.
    Refinement { primes: Vec<usize> },
}

/// The further colourings of the multi-colouring check of one pattern.
#[derive(Clone, Debug)]
pub struct CheckColourings {
    n: usize,
    construction: CheckConstruction,
    /// The client's colouring `c1` of every column.
    c1: Vec<u32>,
    /// `q(j)`: the rank of column `j` inside its `c1` class.
    rank: Vec<u32>,
    /// The members of `c1` class `c`, increasing:
    /// `members[class_start[c]..class_start[c + 1]]`.
    class_start: Vec<usize>,
    members: Vec<u32>,
    /// Refinement construction: the first group of every `c1` class in
    /// colouring `r` (`offsets[r]` has one entry per class and the total).
    offsets: Vec<Vec<usize>>,
    /// Non-empty groups of every further colouring.
    groups: Vec<usize>,
    /// The first group of every further colouring in the order of the check.
    group_start: Vec<usize>,
    total_band: usize,
    total_refinement: usize,
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn is_prime(p: usize) -> bool {
    p >= 2
        && (2..)
            .take_while(|d| d * d <= p)
            .all(|d| !p.is_multiple_of(d))
}

/// The moduli of the band construction: the smallest pairwise coprime
/// integers of at least `width`, increasing, until their product is at
/// least `n`.
fn band_moduli(width: usize, n: usize) -> Vec<usize> {
    let mut moduli: Vec<usize> = Vec::new();
    let mut product: u128 = 1;
    let mut m = width.max(1);
    while product < n as u128 {
        if moduli.iter().all(|&chosen| gcd(chosen, m) == 1) {
            moduli.push(m);
            product = product.saturating_mul(m as u128);
        }
        m += 1;
    }
    moduli
}

/// The primes `2, 3, 5, ..` until their product is at least `largest`.
fn refinement_primes(largest: usize) -> Vec<usize> {
    let mut primes = Vec::new();
    let mut product: u128 = 1;
    let mut p = 2;
    while product < largest as u128 {
        if is_prime(p) {
            primes.push(p);
            product = product.saturating_mul(p as u128);
        }
        p += 1;
    }
    primes
}

/// The moduli of the band construction for a pattern of order `n` whose
/// declared entries lie in the band `(lower, upper)`, and the total of its
/// non-empty groups.
fn band_plan(n: usize, lower: usize, upper: usize) -> (Vec<usize>, usize) {
    let moduli = band_moduli(lower + upper + 1, n);
    let total = moduli.iter().map(|&m| m.min(n)).sum();
    (moduli, total)
}

/// The primes of the refinement construction for the classes of the
/// client's colouring of sizes `sizes`, and the total of its non-empty
/// groups.
fn refinement_plan(sizes: &[usize]) -> (Vec<usize>, usize) {
    let largest = sizes.iter().copied().max().unwrap_or(1);
    let primes = refinement_primes(largest);
    let total = primes
        .iter()
        .map(|&p| sizes.iter().map(|&size| size.min(p)).sum::<usize>())
        .sum::<usize>();
    (primes, total)
}

/// The columns of one group, increasing.
pub(super) enum GroupColumns<'a> {
    Band(StepBy<Range<usize>>),
    Refinement(StepBy<Iter<'a, u32>>),
}

impl Iterator for GroupColumns<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        match self {
            Self::Band(columns) => columns.next(),
            Self::Refinement(columns) => columns.next().map(|&j| j as usize),
        }
    }
}

impl CheckColourings {
    /// The further colourings of `pattern`, with `c1` its Curtis-Powell-Reid
    /// colouring.
    pub fn of(pattern: &SparsityPattern) -> Result<Self, AssemblyError> {
        let (colour, _) = colour_columns(pattern);
        let colour: Vec<u32> = colour.iter().map(|&c| c as u32).collect();
        let (lower, upper) = pattern.bandwidth();
        Self::from_parts(pattern.n(), &colour, lower, upper)
    }

    /// The further colourings of a pattern of order `n` whose client
    /// colouring is `c1` and whose declared entries lie in the band
    /// `(lower, upper)`.
    pub(super) fn from_parts(
        n: usize,
        c1: &[u32],
        lower: usize,
        upper: usize,
    ) -> Result<Self, AssemblyError> {
        if n == 0 || c1.len() != n {
            return Err(AssemblyError::Configuration(format!(
                "check colourings of order {n} need {n} colours, got {}",
                c1.len()
            )));
        }
        let classes = c1.iter().map(|&c| c as usize + 1).max().unwrap_or(0);
        let mut sizes = vec![0_usize; classes];
        for &c in c1 {
            sizes[c as usize] += 1;
        }
        let mut class_start = Vec::with_capacity(classes + 1);
        class_start.push(0);
        for c in 0..classes {
            class_start.push(class_start[c] + sizes[c]);
        }
        let mut next = class_start.clone();
        let mut members = vec![0_u32; n];
        let mut rank = vec![0_u32; n];
        for (j, &c) in c1.iter().enumerate() {
            let c = c as usize;
            rank[j] = (next[c] - class_start[c]) as u32;
            members[next[c]] = j as u32;
            next[c] += 1;
        }
        let (moduli, total_band) = band_plan(n, lower, upper);
        let (primes, total_refinement) = refinement_plan(&sizes);
        let (construction, offsets, groups): (_, _, Vec<usize>) = if total_band <= total_refinement
        {
            let groups = moduli.iter().map(|&m| m.min(n)).collect();
            (
                CheckConstruction::Band {
                    lower,
                    upper,
                    moduli,
                },
                Vec::new(),
                groups,
            )
        } else {
            let offsets: Vec<Vec<usize>> = primes
                .iter()
                .map(|&p| {
                    let mut offsets = Vec::with_capacity(classes + 1);
                    offsets.push(0);
                    for c in 0..classes {
                        offsets.push(offsets[c] + sizes[c].min(p));
                    }
                    offsets
                })
                .collect();
            let groups = offsets.iter().map(|offsets| offsets[classes]).collect();
            (CheckConstruction::Refinement { primes }, offsets, groups)
        };
        let mut group_start = Vec::with_capacity(groups.len() + 1);
        group_start.push(0);
        for (r, &count) in groups.iter().enumerate() {
            group_start.push(group_start[r] + count);
        }
        let colourings = Self {
            n,
            construction,
            c1: c1.to_vec(),
            rank,
            class_start,
            members,
            offsets,
            groups,
            group_start,
            total_band,
            total_refinement,
        };
        if let Some((a, b)) = colourings.repeated_tuple() {
            return Err(AssemblyError::Configuration(format!(
                "the check colourings do not tell columns {a} and {b} apart (a defect of the \
                 construction, not of the pattern)"
            )));
        }
        Ok(colourings)
    }

    /// The order of the pattern.
    pub fn n(&self) -> usize {
        self.n
    }

    /// The construction in use.
    pub fn construction(&self) -> &CheckConstruction {
        &self.construction
    }

    /// The number of further colourings (the client's own not counted).
    pub fn colourings(&self) -> usize {
        self.groups.len()
    }

    /// Non-empty groups of further colouring `r`.
    pub fn groups(&self, r: usize) -> usize {
        self.groups[r]
    }

    /// Non-empty groups of all further colourings: the right-hand sides of
    /// one check.
    pub fn total_groups(&self) -> usize {
        self.group_start[self.groups.len()]
    }

    /// [`CheckColourings::total_groups`] of the colourings that
    /// [`CheckColourings::of`] builds for `pattern`, without building them:
    /// the smaller of the totals of the two constructions (the band
    /// construction on ties), computed from the client's colouring of the
    /// pattern ([`colour_columns`]), its band ([`SparsityPattern::bandwidth`])
    /// and its order. A function of the declared pattern alone: no state and
    /// no right-hand side enter. It decides the default first-use check of the
    /// coloured route ([`super::AssemblyOptions::default_first_use_check`]).
    pub fn total_groups_of(pattern: &SparsityPattern) -> usize {
        let (colour, classes) = colour_columns(pattern);
        let mut sizes = vec![0_usize; classes];
        for &c in &colour {
            sizes[c] += 1;
        }
        let (lower, upper) = pattern.bandwidth();
        let (_, total_band) = band_plan(pattern.n(), lower, upper);
        let (_, total_refinement) = refinement_plan(&sizes);
        total_band.min(total_refinement)
    }

    /// The total of the band construction, whether it is used or not.
    pub fn total_groups_band(&self) -> usize {
        self.total_band
    }

    /// The total of the refinement construction, whether it is used or not.
    pub fn total_groups_refinement(&self) -> usize {
        self.total_refinement
    }

    /// The client's colour of column `j`.
    pub fn client_colour(&self, j: usize) -> usize {
        self.c1[j] as usize
    }

    /// The group of column `j` in further colouring `r`.
    pub fn colour(&self, r: usize, j: usize) -> usize {
        match &self.construction {
            CheckConstruction::Band { moduli, .. } => j % moduli[r],
            CheckConstruction::Refinement { primes } => {
                self.offsets[r][self.c1[j] as usize] + self.rank[j] as usize % primes[r]
            }
        }
    }

    /// The columns of group `g` of further colouring `r`, increasing.
    pub fn columns(&self, r: usize, g: usize) -> Vec<usize> {
        self.group_columns(r, g).collect()
    }

    /// Item `item` of the check order (colourings in turn, groups
    /// increasing) as `(colouring, group)`.
    pub(super) fn locate(&self, item: usize) -> (usize, usize) {
        let r = self.group_start.partition_point(|&start| start <= item) - 1;
        (r, item - self.group_start[r])
    }

    /// The columns of group `g` of further colouring `r`, increasing.
    pub(super) fn group_columns(&self, r: usize, g: usize) -> GroupColumns<'_> {
        match &self.construction {
            CheckConstruction::Band { moduli, .. } => {
                GroupColumns::Band((g..self.n).step_by(moduli[r]))
            }
            CheckConstruction::Refinement { primes } => {
                let offsets = &self.offsets[r];
                let class = offsets.partition_point(|&start| start <= g) - 1;
                let residue = g - offsets[class];
                let class_members =
                    &self.members[self.class_start[class]..self.class_start[class + 1]];
                GroupColumns::Refinement(class_members[residue..].iter().step_by(primes[r]))
            }
        }
    }

    /// Two columns with the same tuple of colours, if there are any.
    fn repeated_tuple(&self) -> Option<(usize, usize)> {
        let compare = |a: usize, b: usize| -> Ordering {
            self.c1[a].cmp(&self.c1[b]).then_with(|| {
                (0..self.colourings())
                    .map(|r| self.colour(r, a).cmp(&self.colour(r, b)))
                    .find(|ordering| ordering.is_ne())
                    .unwrap_or(Ordering::Equal)
            })
        };
        let mut order: Vec<usize> = (0..self.n).collect();
        order.sort_unstable_by(|&a, &b| compare(a, b));
        order
            .windows(2)
            .find(|pair| compare(pair[0], pair[1]).is_eq())
            .map(|pair| (pair[0].min(pair[1]), pair[0].max(pair[1])))
    }
}
