//! Opt-in retained report storage. No physical evidence is inferred from
//! compact layout; every report is produced by the original certified query.
//! All stored values are binary64 copies or bit-checked aliases.
use super::*;
use rodas5p_integrators::retro_dae_init::InitMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum QueryArenaLayout {
    Packed,
    Shared,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum NativeRetention {
    Owned,
    BorrowedRows,
    BorrowedComponent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RetainedOutputRoute {
    pub queries: QueryArenaLayout,
    pub native: NativeRetention,
}

const AUTHORITIES: [&str; 3] = [
    "whole-branch contraction + local self-map",
    "directed frozen affine root enclosure",
    "bounded exact square-free Sturm count + certified root bracket",
];
#[derive(Clone)]
struct ReportHeader {
    authority: u8,
    route: InitRoute,
    mode: InitMode,
    t: f64,
    correction_upper: f64,
    original_residual_upper: f64,
    residual_upper: f64,
    state_error_upper: f64,
    exact_center_consistency: bool,
    branch_defect_upper: Option<f64>,
    local_defect_upper: f64,
    inverse_norm_upper: f64,
    condition_upper: f64,
    rank: usize,
    derivative_error_upper: f64,
    tangent_residual_upper: f64,
    work: InitWork,
    qualified: bool,
}
impl ReportHeader {
    fn capture(r: &InitReport) -> CoreResult<Self> {
        if r.affine_root.is_some() {
            return Err(bad("arena exact affine metadata unsupported"));
        }
        let authority = AUTHORITIES
            .iter()
            .position(|&a| a == r.branch_authority)
            .ok_or_else(|| bad("arena branch authority unsupported"))?
            as u8;
        Ok(Self {
            authority,
            route: r.route,
            mode: r.mode,
            t: r.t,
            correction_upper: r.correction_upper,
            original_residual_upper: r.original_residual_upper,
            residual_upper: r.residual_upper,
            state_error_upper: r.state_error_upper,
            exact_center_consistency: r.exact_center_consistency,
            branch_defect_upper: r.branch_defect_upper,
            local_defect_upper: r.local_defect_upper,
            inverse_norm_upper: r.inverse_norm_upper,
            condition_upper: r.condition_upper,
            rank: r.rank,
            derivative_error_upper: r.derivative_error_upper,
            tangent_residual_upper: r.tangent_residual_upper,
            work: r.work.clone(),
            qualified: r.qualified,
        })
    }
    fn write(&self, r: &mut InitReport) {
        r.affine_root = None;
        r.branch_authority.clear();
        r.branch_authority
            .push_str(AUTHORITIES[self.authority as usize]);
        r.route = self.route;
        r.mode = self.mode;
        r.t = self.t;
        r.correction_upper = self.correction_upper;
        r.original_residual_upper = self.original_residual_upper;
        r.residual_upper = self.residual_upper;
        r.state_error_upper = self.state_error_upper;
        r.exact_center_consistency = self.exact_center_consistency;
        r.branch_defect_upper = self.branch_defect_upper;
        r.local_defect_upper = self.local_defect_upper;
        r.inverse_norm_upper = self.inverse_norm_upper;
        r.condition_upper = self.condition_upper;
        r.rank = self.rank;
        r.derivative_error_upper = self.derivative_error_upper;
        r.tangent_residual_upper = self.tangent_residual_upper;
        r.work = self.work.clone();
        r.qualified = self.qualified;
    }
    fn empty_report(&self) -> InitReport {
        InitReport {
            branch_authority: AUTHORITIES[self.authority as usize].to_string(),
            affine_root: None,
            branch: Vec::new(),
            original_z: Vec::new(),
            reduced_jacobian: Vec::new(),
            root_box: Vec::new(),
            x: Vec::new(),
            xdot: Vec::new(),
            z: Vec::new(),
            zdot: Vec::new(),
            zt: Vec::new(),
            zx_columns: Vec::new(),
            route: self.route,
            mode: self.mode,
            t: self.t,
            correction_upper: self.correction_upper,
            original_residual_upper: self.original_residual_upper,
            residual_upper: self.residual_upper,
            state_error_upper: self.state_error_upper,
            exact_center_consistency: self.exact_center_consistency,
            branch_defect_upper: self.branch_defect_upper,
            local_defect_upper: self.local_defect_upper,
            inverse_norm_upper: self.inverse_norm_upper,
            condition_upper: self.condition_upper,
            rank: self.rank,
            derivative_error_upper: self.derivative_error_upper,
            tangent_residual_upper: self.tangent_residual_upper,
            work: self.work.clone(),
            qualified: self.qualified,
        }
    }
}
#[derive(Clone)]
struct QueryHeader {
    t: f64,
    raw_constraint_upper: Option<f64>,
    projected_constraint_upper: f64,
    physical_error_upper: Option<f64>,
    projection: ReportHeader,
}
/// Fixed-stride contiguous report arena. Reserve once. Appending fails
/// without mutation if the capacity/shape/authority contract is violated.
/// Shared layout only aliases represented-bit-identical fields.
pub struct QueryArena {
    layout: QueryArenaLayout,
    nx: usize,
    nz: usize,
    nr: usize,
    stride: usize,
    limit: usize,
    headers: Vec<QueryHeader>,
    data: Vec<f64>,
    branch: Vec<[f64; 2]>,
}
impl QueryArena {
    pub fn new(
        layout: QueryArenaLayout,
        nx: usize,
        nz: usize,
        nr: usize,
        branch: &[[f64; 2]],
        capacity: usize,
    ) -> CoreResult<Self> {
        let n = nx
            .checked_add(nz)
            .ok_or_else(|| bad("arena dimension overflow"))?;
        if nx == 0
            || nz == 0
            || n > 32
            || !(nr == nx || nr == n)
            || branch.len() != nz
            || branch.iter().flatten().any(|v| !v.is_finite())
        {
            return Err(bad("arena shape"));
        }
        let packed = nr + 3 * nx + 9 * nz + nx * nz + nx * nx;
        let stride = if layout == QueryArenaLayout::Packed {
            packed
        } else {
            packed - nx - 3 * nz - if nr == nx { nr } else { 0 }
        };
        let len = capacity
            .checked_mul(stride)
            .ok_or_else(|| bad("arena capacity overflow"))?;
        let mut headers = Vec::new();
        headers.try_reserve_exact(capacity).map_err(bad)?;
        let mut data = Vec::new();
        data.try_reserve_exact(len).map_err(bad)?;
        Ok(Self {
            layout,
            nx,
            nz,
            nr,
            stride,
            limit: capacity,
            headers,
            data,
            branch: if layout == QueryArenaLayout::Shared {
                branch.to_vec()
            } else {
                vec![]
            },
        })
    }
    pub fn len(&self) -> usize {
        self.headers.len()
    }
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }
    pub fn capacity_bytes(&self) -> usize {
        self.headers.capacity() * std::mem::size_of::<QueryHeader>()
            + self.data.capacity() * 8
            + self.branch.capacity() * 16
    }
    pub fn words_per_query(&self) -> usize {
        self.stride
    }
    pub fn push(&mut self, q: &Query) -> CoreResult<()> {
        let nx = self.nx;
        let nz = self.nz;
        let r = &q.projection;
        if self.len() >= self.limit
            || q.raw.len() != self.nr
            || q.state.len() != nx + nz
            || r.x.len() != nx
            || r.original_z.len() != nz
            || r.z.len() != nz
            || r.branch.len() != nz
            || r.root_box.len() != nz
            || r.xdot.len() != nx
            || r.zdot.len() != nz
            || r.zt.len() != nz
            || r.zx_columns.len() != nx
            || r.zx_columns.iter().any(|v| v.len() != nz)
            || r.reduced_jacobian.len() != nx
            || r.reduced_jacobian.iter().any(|v| v.len() != nx)
        {
            return Err(bad("arena query shape/capacity"));
        }
        if self.layout == QueryArenaLayout::Shared
            && (!same_f64_bits(&q.state[..nx], &r.x)
                || !same_f64_bits(&q.state[nx..], &r.z)
                || (self.nr == nx && !same_f64_bits(&q.raw, &r.x))
                || self
                    .branch
                    .iter()
                    .zip(&r.branch)
                    .any(|(a, b)| !same_f64_bits(a, b)))
        {
            return Err(bad("arena shared identity"));
        }
        let header = QueryHeader {
            t: q.t,
            raw_constraint_upper: q.raw_constraint_upper,
            projected_constraint_upper: q.projected_constraint_upper,
            physical_error_upper: q.physical_error_upper,
            projection: ReportHeader::capture(r)?,
        };
        let start = self.data.len();
        if self.layout == QueryArenaLayout::Packed || self.nr != nx {
            self.data.extend_from_slice(&q.raw);
        }
        self.data.extend_from_slice(&q.state);
        if self.layout == QueryArenaLayout::Packed {
            self.data.extend_from_slice(&r.x);
            self.data.extend_from_slice(&r.z);
        }
        self.data.extend_from_slice(&r.original_z);
        if self.layout == QueryArenaLayout::Packed {
            for b in &r.branch {
                self.data.extend_from_slice(b);
            }
        }
        for b in &r.root_box {
            self.data.extend_from_slice(b);
        }
        self.data.extend_from_slice(&r.xdot);
        self.data.extend_from_slice(&r.zdot);
        self.data.extend_from_slice(&r.zt);
        for row in r.zx_columns.iter().chain(&r.reduced_jacobian) {
            self.data.extend_from_slice(row);
        }
        debug_assert_eq!(self.data.len() - start, self.stride);
        self.headers.push(header);
        Ok(())
    }
    pub fn get(&self, index: usize) -> Option<RetainedQueryView<'_>> {
        let h = self.headers.get(index)?;
        let data = &self.data[index * self.stride..(index + 1) * self.stride];
        Some(RetainedQueryView {
            arena: self,
            header: h,
            data,
        })
    }
}
/// Read-only view of a report in an arena. No lifetime extension or
/// references into a movable Vec while appending are permitted by Rust.
pub struct RetainedQueryView<'a> {
    arena: &'a QueryArena,
    header: &'a QueryHeader,
    data: &'a [f64],
}
struct Parts<'a> {
    raw: &'a [f64],
    state: &'a [f64],
    x: &'a [f64],
    z: &'a [f64],
    seed: &'a [f64],
    branch: Option<&'a [f64]>,
    root: &'a [f64],
    xdot: &'a [f64],
    zdot: &'a [f64],
    zt: &'a [f64],
    zx: &'a [f64],
    jac: &'a [f64],
}
fn take<'a>(row: &mut &'a [f64], n: usize) -> &'a [f64] {
    let (a, b) = row.split_at(n);
    *row = b;
    a
}
fn replace<T: Copy>(v: &mut Vec<T>, src: &[T]) {
    v.clear();
    v.extend_from_slice(src);
}
fn pairs(v: &mut Vec<[f64; 2]>, src: &[f64]) {
    v.clear();
    v.extend(src.chunks_exact(2).map(|a| [a[0], a[1]]));
}
fn rows(v: &mut Vec<Vec<f64>>, src: &[f64], width: usize) {
    v.resize_with(src.len() / width, Vec::new);
    for (v, s) in v.iter_mut().zip(src.chunks_exact(width)) {
        replace(v, s);
    }
}
impl<'a> RetainedQueryView<'a> {
    fn parts(&self) -> Parts<'a> {
        let mut d = self.data;
        let nx = self.arena.nx;
        let nz = self.arena.nz;
        let nr = self.arena.nr;
        let packed = self.arena.layout == QueryArenaLayout::Packed;
        let raw = if packed || nr != nx {
            Some(take(&mut d, nr))
        } else {
            None
        };
        let state = take(&mut d, nx + nz);
        let x = if packed {
            take(&mut d, nx)
        } else {
            &state[..nx]
        };
        let z = if packed {
            take(&mut d, nz)
        } else {
            &state[nx..]
        };
        let seed = take(&mut d, nz);
        let branch = if packed {
            Some(take(&mut d, 2 * nz))
        } else {
            None
        };
        let root = take(&mut d, 2 * nz);
        let xdot = take(&mut d, nx);
        let zdot = take(&mut d, nz);
        let zt = take(&mut d, nz);
        let zx = take(&mut d, nx * nz);
        let jac = take(&mut d, nx * nx);
        debug_assert!(d.is_empty());
        Parts {
            raw: raw.unwrap_or(x),
            state,
            x,
            z,
            seed,
            branch,
            root,
            xdot,
            zdot,
            zt,
            zx,
            jac,
        }
    }
    pub fn t(&self) -> f64 {
        self.header.t
    }
    pub fn raw(&self) -> &'a [f64] {
        self.parts().raw
    }
    pub fn state(&self) -> &'a [f64] {
        self.parts().state
    }
    pub fn physical_error_upper(&self) -> Option<f64> {
        self.header.physical_error_upper
    }
    pub fn write_into(&self, out: &mut Query) {
        let p = self.parts();
        let h = self.header;
        out.t = h.t;
        replace(&mut out.raw, p.raw);
        replace(&mut out.state, p.state);
        out.raw_constraint_upper = h.raw_constraint_upper;
        out.projected_constraint_upper = h.projected_constraint_upper;
        out.physical_error_upper = h.physical_error_upper;
        let r = &mut out.projection;
        h.projection.write(r);
        replace(&mut r.x, p.x);
        replace(&mut r.z, p.z);
        replace(&mut r.original_z, p.seed);
        if let Some(b) = p.branch {
            pairs(&mut r.branch, b);
        } else {
            replace(&mut r.branch, &self.arena.branch);
        }
        pairs(&mut r.root_box, p.root);
        replace(&mut r.xdot, p.xdot);
        replace(&mut r.zdot, p.zdot);
        replace(&mut r.zt, p.zt);
        rows(&mut r.zx_columns, p.zx, self.arena.nz);
        rows(&mut r.reduced_jacobian, p.jac, self.arena.nx);
    }
    pub fn to_owned_query(&self) -> Query {
        let mut q = Query {
            t: 0.,
            raw: vec![],
            state: vec![],
            raw_constraint_upper: None,
            projected_constraint_upper: 0.,
            physical_error_upper: None,
            projection: self.header.projection.empty_report(),
        };
        self.write_into(&mut q);
        q
    }
}
