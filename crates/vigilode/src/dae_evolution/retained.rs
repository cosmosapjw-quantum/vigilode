//! Opt-in retained report storage. No physical evidence is inferred from
//! compact layout; every report is produced by the original certified query.
//! All stored values are binary64 copies or bit-checked aliases.
use super::*;
mod slabs;
use rodas5p_integrators::retro_dae_init::InitMode;
pub(super) use slabs::SlabArena;
pub use slabs::*;

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

pub(super) struct RetainedBuild {
    route: RetainedOutputRoute,
    arena: Option<QueryArena>,
    accepted: usize,
    queries: usize,
    pub(super) slab_plan: Option<SlabReplayRoute>,
    pub(super) engine_plan: EnginePlan,
    slab_arena: Option<SlabArena>,
}
impl RetainedBuild {
    fn new(route: RetainedOutputRoute) -> Self {
        Self {
            route,
            arena: None,
            accepted: 0,
            queries: 0,
            slab_plan: None,
            engine_plan: EnginePlan::Legacy,
            slab_arena: None,
        }
    }
    pub(super) fn native(
        &self,
        s: &Solution,
        k: usize,
    ) -> CoreResult<(NativePacket, Vec<Vec<Iv>>)> {
        if self.route.native == NativeRetention::Owned {
            let p = packet(s, k)?;
            let y = p.power()?;
            return Ok((p, y));
        }
        let v = native_packet_view(s, k)?;
        let power = match self.route.native {
            NativeRetention::BorrowedRows => v.power()?,
            NativeRetention::BorrowedComponent => {
                let Some(Extension::Fast(f)) = &s.extension else {
                    return Err(bad("component native retention requires fast dense driver"));
                };
                de::dense_power_components(f, k)?
            }
            _ => unreachable!(),
        };
        // Private placeholder only; not exposed as a legacy SlabRecord.
        // The immutable driver remains the sole native packet authority.
        Ok((
            NativePacket {
                basis: v.basis,
                coefficients: vec![],
                y0: vec![],
                y1: vec![],
                stages: vec![],
            },
            power,
        ))
    }
    #[inline(never)]
    pub(super) fn collect(
        &mut self,
        out: &mut EvolutionRun,
        e: &Engine,
        grid: &[f64],
        route: QueryWorkspaceRoute,
    ) -> CoreResult<()> {
        let nx = e.src.nx();
        let nz = e.src.nz();
        let nr = if e.route == EvolutionRoute::Mass {
            nx + nz
        } else {
            nx
        };
        let cap = out
            .driver
            .t
            .len()
            .checked_add(grid.len())
            .ok_or_else(|| bad("retained report count overflow"))?;
        let arena = QueryArena::new(self.route.queries, nx, nz, nr, &e.branch, cap)?;
        self.arena = Some(arena);
        if e.scalar_callback() || (e.route == EvolutionRoute::Mass && nz == 1) {
            self.collect_with(out, e, grid, route, &mut ScalarQueryWorkspace::new())
        } else {
            self.collect_vector(out, e, grid, route)
        }
    }
    #[inline(never)]
    fn collect_vector(
        &mut self,
        out: &mut EvolutionRun,
        e: &Engine,
        grid: &[f64],
        route: QueryWorkspaceRoute,
    ) -> CoreResult<()> {
        self.collect_with(out, e, grid, route, &mut QueryWorkspace::new())
    }
    fn collect_with(
        &mut self,
        out: &mut EvolutionRun,
        e: &Engine,
        grid: &[f64],
        route: QueryWorkspaceRoute,
        w: &mut impl QueryScratch,
    ) -> CoreResult<()> {
        let mut stage: Option<Query> = None;
        let arena = self.arena.as_mut().expect("arena reserved");
        for (&t, y) in out.driver.t.iter().zip(&out.driver.y) {
            match fixed_query_with_arena(
                e,
                &out.driver,
                t,
                Some(y),
                &out.slabs,
                route,
                w,
                self.slab_arena.as_ref(),
            ) {
                Ok(v) => {
                    if let Some(q) = &mut stage {
                        v.write(q);
                    } else {
                        stage = Some(v.owned());
                    }
                    arena.push(stage.as_ref().unwrap())?;
                    self.accepted += 1;
                }
                Err(err) => {
                    out.reason = Some(err.to_string());
                    break;
                }
            }
        }
        for &t in grid {
            match fixed_query_with_arena(
                e,
                &out.driver,
                t,
                None,
                &out.slabs,
                route,
                w,
                self.slab_arena.as_ref(),
            ) {
                Ok(v) => {
                    if let Some(q) = &mut stage {
                        v.write(q);
                    } else {
                        stage = Some(v.owned());
                    }
                    arena.push(stage.as_ref().unwrap())?;
                    self.queries += 1;
                }
                Err(err) => {
                    out.reason = Some(err.to_string());
                    break;
                }
            }
        }
        Ok(())
    }
    pub(super) fn counts(&self) -> (usize, usize) {
        (self.accepted, self.queries)
    }
    pub(super) fn max_error(&self, mx: f64) -> Option<f64> {
        self.arena
            .as_ref()?
            .headers
            .iter()
            .try_fold(mx, |m, h| h.physical_error_upper.map(|v| m.max(v)))
    }
}
/// Borrowed slab view, with a complete native packet and original
/// power/certificate; no owned native rows are constructed.
#[derive(Serialize)]
pub struct RetainedSlabView<'a> {
    pub times: [f64; 2],
    pub native: NativePacketView<'a>,
    pub power: &'a [Vec<[f64; 2]>],
    pub certificate: Option<&'a SlabCertificate>,
    pub reason: Option<&'a str>,
}
/// Separate retained-output API. The legacy EvolutionRun is only returned
/// after complete materialization; private placeholders are never exposed.
pub struct RetainedEvolutionRun {
    run: EvolutionRun,
    build: RetainedBuild,
}
impl RetainedEvolutionRun {
    pub fn driver(&self) -> &Solution {
        self.run.driver()
    }
    pub fn initial(&self) -> &InitReport {
        &self.run.initial
    }
    pub fn work(&self) -> &ProjectionWork {
        &self.run.work
    }
    pub fn global_qualified(&self) -> bool {
        self.run.global_qualified
    }
    pub fn branch_qualified(&self) -> bool {
        self.run.branch_qualified
    }
    pub fn pointwise_qualified(&self) -> bool {
        self.run.pointwise_qualified
    }
    pub fn max_physical_error(&self) -> Option<f64> {
        self.run.max_physical_error
    }
    pub fn reason(&self) -> Option<&str> {
        self.run.reason.as_deref()
    }
    pub fn accepted_len(&self) -> usize {
        self.build.accepted
    }
    pub fn query_len(&self) -> usize {
        self.build.queries
    }
    pub fn slabs_len(&self) -> usize {
        self.run.slabs.len()
    }
    pub fn accepted(&self, k: usize) -> Option<RetainedQueryView<'_>> {
        if k >= self.accepted_len() {
            return None;
        }
        self.build.arena.as_ref()?.get(k)
    }
    pub fn retained_query(&self, k: usize) -> Option<RetainedQueryView<'_>> {
        if k >= self.query_len() {
            return None;
        }
        self.build.arena.as_ref()?.get(self.accepted_len() + k)
    }
    pub fn slab(&self, k: usize) -> CoreResult<RetainedSlabView<'_>> {
        let a = self
            .run
            .slabs
            .get(k)
            .ok_or_else(|| bad("retained slab index"))?;
        Ok(RetainedSlabView {
            times: a.times,
            native: native_packet_view(&self.run.driver, k)?,
            power: &a.power,
            certificate: a.certificate.as_ref(),
            reason: a.reason.as_deref(),
        })
    }
    pub fn query_arena_bytes(&self) -> usize {
        self.build
            .arena
            .as_ref()
            .map_or(0, QueryArena::capacity_bytes)
    }
    /// Heap capacity of duplicated native packet arrays only, not the
    /// driver itself or slab certificate storage.
    pub fn native_copy_bytes(&self) -> usize {
        self.run
            .slabs
            .iter()
            .map(|s| native_copy_bytes(&s.native))
            .sum()
    }
    pub fn materialize(mut self) -> CoreResult<EvolutionRun> {
        if let Some(arena) = self.build.arena.take() {
            self.run.accepted = (0..self.build.accepted)
                .map(|i| arena.get(i).unwrap().to_owned_query())
                .collect();
            self.run.queries = (self.build.accepted..arena.len())
                .map(|i| arena.get(i).unwrap().to_owned_query())
                .collect();
        }
        if self.build.route.native != NativeRetention::Owned {
            for k in 0..self.run.slabs.len() {
                self.run.slabs[k].native =
                    native_packet_view(&self.run.driver, k)?.to_owned_packet();
            }
        }
        Ok(self.run)
    }
}
pub fn native_copy_bytes(s: &NativePacket) -> usize {
    8 * (s.coefficients.capacity()
        + s.y0.capacity()
        + s.y1.capacity()
        + s.stages.iter().map(Vec::capacity).sum::<usize>())
        + std::mem::size_of::<Vec<f64>>() * s.stages.capacity()
}
/// Compare owned output capacity to an arena; excludes allocator metadata,
/// driver, original certificate, and inline container itself.
pub fn owned_query_bytes(q: &Query) -> usize {
    let r = &q.projection;
    8 * (q.raw.capacity()
        + q.state.capacity()
        + r.x.capacity()
        + r.original_z.capacity()
        + r.z.capacity()
        + r.xdot.capacity()
        + r.zdot.capacity()
        + r.zt.capacity()
        + r.zx_columns
            .iter()
            .chain(&r.reduced_jacobian)
            .map(Vec::capacity)
            .sum::<usize>())
        + 16 * (r.branch.capacity() + r.root_box.capacity())
        + r.branch_authority.capacity()
        + std::mem::size_of::<Vec<f64>>()
            * (r.zx_columns.capacity() + r.reduced_jacobian.capacity())
}
#[allow(clippy::too_many_arguments)]
pub fn evolve_epoch_retained_audited(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
    retention: RetainedOutputRoute,
) -> (CoreResult<RetainedEvolutionRun>, NativeResourceAudit) {
    let a = Arc::new(Mutex::new(NativeResourceAudit::default()));
    let mut build = RetainedBuild::new(retention);
    // Separate API is admitted only for the explicit bounded query route.
    // Do not reinterpret an arbitrary existing owning projection.
    let r = if route.query_workspace().is_none() {
        Err(bad("retained epoch requires WorkspaceQuery5"))
    } else {
        evolve_epoch_authority_retained(
            src,
            x,
            z,
            branch,
            epoch,
            opts,
            policy,
            route,
            grid,
            None,
            None,
            Some(a.clone()),
            Some(&mut build),
        )
    };
    let receipt = a.lock().expect("native audit lock").clone();
    (r.map(|run| RetainedEvolutionRun { run, build }), receipt)
}
