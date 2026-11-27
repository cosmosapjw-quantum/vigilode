//! Reusable slab computation and lossless fixed-stride slab retention.
//! Compact metadata never escapes as a legacy owning certificate.
use super::*;
use de::{
    ReconstructionSeedRoute as Seed, ScalarReconstructionWorkspace, VectorReconstructionWorkspace,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SlabReplayRoute {
    ReuseOwned,
    Flat,
}

/// Scratch for an explicitly bounded five-node replay. The large vector
/// workspace is separately caller-owned; scalar users do not instantiate it.
/// Each source proof is recomputed; stage flags prevent stale outputs.
pub struct SlabReplayWorkspace<'a> {
    nx: usize,
    nz: usize,
    stage: u8,
    power: Vec<Vec<Iv>>,
    scalar: ScalarReconstructionWorkspace,
    vector: Option<&'a mut VectorReconstructionWorkspace>,
    z: [Iv; 160],
    work: [InitWork; 5],
    proof: de::SlabCertificateWorkspace,
    report: SlabCertificate,
}
impl<'a> SlabReplayWorkspace<'a> {
    pub fn new(
        nx: usize,
        nz: usize,
        vector: Option<&'a mut VectorReconstructionWorkspace>,
    ) -> CoreResult<Self> {
        let n = nx
            .checked_add(nz)
            .ok_or_else(|| bad("slab workspace dimension"))?;
        if nx == 0 || nz == 0 || n > 32 || nz > 1 && vector.is_none() {
            return Err(bad("slab workspace shape"));
        }
        let zero = Iv { lo: 0., hi: 0. };
        Ok(Self {
            nx,
            nz,
            stage: 0,
            power: vec![vec![zero; 5]; n],
            scalar: ScalarReconstructionWorkspace::new(),
            vector,
            z: [zero; 160],
            work: std::array::from_fn(|_| InitWork::default()),
            proof: de::SlabCertificateWorkspace::new(),
            report: SlabCertificate::workspace_output(nx, nz)?,
        })
    }
    pub fn dense_rows(&mut self, v: NativePacketView<'_>) -> CoreResult<()> {
        self.stage = 0;
        de::dense_power_rows_into(v.y0, v.y1, &v.stages, v.basis, &mut self.power[..self.nx])?;
        self.stage = 1;
        Ok(())
    }
    fn dense_components(&mut self, s: &Solution, k: usize) -> CoreResult<()> {
        self.stage = 0;
        let Some(Extension::Fast(f)) = &s.extension else {
            return Err(bad("component replay requires fast dense store"));
        };
        de::dense_power_components_into(f, k, &mut self.power[..self.nx])?;
        self.stage = 1;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        &mut self,
        src: &DaeSource,
        times: [f64; 2],
        seed: &[f64],
        branch: &[[f64; 2]],
        policy: InitPolicy,
        route: InitRoute,
        audit: &mut ProjectionAttemptWork,
    ) -> CoreResult<()> {
        let ready = self.stage == 1;
        self.stage = 0;
        if !ready || src.nx() != self.nx || src.nz() != self.nz {
            return Err(bad("slab reconstruction scratch state/shape"));
        }
        let x = &self.power[..self.nx];
        let z = &mut self.z[..self.nz * 5];
        let w = &mut self.work;
        match route {
            InitRoute::Reuse if self.nz == 1 => de::reconstruct_scalar_reuse_into(
                src,
                times,
                x,
                seed,
                branch,
                policy,
                Seed::Fixed,
                z,
                w,
                audit,
                &mut self.scalar,
            )?,
            InitRoute::BoundedPreparedScalar if self.nz == 1 => {
                de::reconstruct_scalar_bounded_prepared_into(
                    src,
                    times,
                    x,
                    seed,
                    branch,
                    policy,
                    Seed::Fixed,
                    z,
                    w,
                    audit,
                    &mut self.scalar,
                )?
            }
            InitRoute::BoundedAffineScalar if self.nz == 1 => {
                de::reconstruct_scalar_bounded_affine_into(
                    src,
                    times,
                    x,
                    seed,
                    branch,
                    policy,
                    Seed::Fixed,
                    z,
                    w,
                    audit,
                    &mut self.scalar,
                )?
            }
            InitRoute::BoundedSturmScalar | InitRoute::BoundedSturmNewtonScalar if self.nz == 1 => {
                de::reconstruct_scalar_bounded_sturm_into(
                    src,
                    times,
                    x,
                    seed,
                    branch,
                    policy,
                    Seed::Fixed,
                    route == InitRoute::BoundedSturmNewtonScalar,
                    z,
                    w,
                    audit,
                    &mut self.scalar,
                )?
            }
            InitRoute::BoundedVectorPivot | InitRoute::BoundedVectorEquilibrated => {
                de::reconstruct_pivoted_vector_into(
                    src,
                    times,
                    x,
                    seed,
                    branch,
                    policy,
                    route == InitRoute::BoundedVectorEquilibrated,
                    Seed::Fixed,
                    z,
                    w,
                    audit,
                    self.vector
                        .as_deref_mut()
                        .ok_or_else(|| bad("vector reconstruction workspace missing"))?,
                )?
            }
            InitRoute::BoundedVector | InitRoute::BoundedVectorRefined => {
                de::reconstruct_bounded_vector_into(
                    src,
                    times,
                    x,
                    seed,
                    branch,
                    policy,
                    route == InitRoute::BoundedVectorRefined,
                    Seed::Fixed,
                    z,
                    w,
                    audit,
                    self.vector
                        .as_deref_mut()
                        .ok_or_else(|| bad("vector reconstruction workspace missing"))?,
                )?
            }
            _ => return Err(bad("slab reconstruction route unsupported")),
        }
        for (o, v) in self.power[self.nx..].iter_mut().zip(z.chunks_exact(5)) {
            o.copy_from_slice(v);
        }
        self.stage = 2;
        Ok(())
    }
    pub fn power(&self) -> Option<&[Vec<Iv>]> {
        (self.stage >= 2).then_some(&self.power)
    }
    pub fn reconstruction_work(&self) -> Option<&[InitWork]> {
        (self.stage >= 2).then_some(&self.work)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn certify(
        &mut self,
        src: &DaeSource,
        times: [f64; 2],
        branch: &[[f64; 2]],
        policy: EvolutionPolicy,
        incoming: Option<f64>,
        route: de::SlabWorkspaceRoute,
    ) -> CoreResult<()> {
        if self.stage < 2 || src.nx() != self.nx || src.nz() != self.nz {
            self.stage = 0;
            return Err(bad("slab certificate scratch state/shape"));
        }
        self.stage = 2;
        de::certify_slab_workspace_into(
            src,
            times,
            &self.power,
            branch,
            policy.tube_radius,
            incoming,
            policy.green,
            policy.init.condition_cap,
            route,
            &mut self.proof,
            &mut self.report,
        )?;
        self.stage = 3;
        Ok(())
    }
    pub fn certificate(&self) -> Option<&SlabCertificate> {
        (self.stage == 3).then_some(&self.report)
    }
    pub fn proof_work(&self) -> de::SlabWorkspaceWork {
        self.proof.work()
    }
}

type Pair = [f64; 2];
type PowerRow = [Pair; 5];
/// Fixed-stride numeric slab payloads; metadata lives in private slab
/// records. Missing power and certificate states are retained explicitly.
pub(in crate::dae_evolution) struct SlabArena {
    nx: usize,
    nz: usize,
    cap: usize,
    present: Vec<bool>,
    power: Vec<PowerRow>,
    ranges: Vec<Pair>,
    numbers: Vec<f64>,
}
#[derive(Clone, Copy)]
struct CertArrays<'a> {
    x_range: &'a [Pair],
    z_range: &'a [Pair],
    branch_root_box: &'a [Pair],
    self_map_box: &'a [Pair],
    self_map_center: &'a [f64],
    preconditioner: &'a [f64],
}
impl<'a> CertArrays<'a> {
    fn owned(c: &'a SlabCertificate) -> Self {
        Self {
            x_range: &c.x_range,
            z_range: &c.z_range,
            branch_root_box: &c.branch_root_box,
            self_map_box: &c.self_map_box,
            self_map_center: &c.self_map_center,
            preconditioner: &c.preconditioner,
        }
    }
    fn restore(self, c: &mut SlabCertificate) {
        c.x_range = self.x_range.to_vec();
        c.z_range = self.z_range.to_vec();
        c.branch_root_box = self.branch_root_box.to_vec();
        c.self_map_box = self.self_map_box.to_vec();
        c.self_map_center = self.self_map_center.to_vec();
        c.preconditioner = self.preconditioner.to_vec();
    }
}
impl SlabArena {
    fn new(nx: usize, nz: usize, cap: usize) -> CoreResult<Self> {
        let mut present = Vec::new();
        let mut power = Vec::new();
        let mut ranges = Vec::new();
        let mut numbers = Vec::new();
        let reserve = |n: usize| {
            cap.checked_mul(n)
                .ok_or_else(|| bad("slab arena capacity overflow"))
        };
        present.try_reserve_exact(cap).map_err(bad)?;
        power.try_reserve_exact(reserve(nx + nz)?).map_err(bad)?;
        ranges
            .try_reserve_exact(reserve(nx + 3 * nz)?)
            .map_err(bad)?;
        numbers
            .try_reserve_exact(reserve(nz + nz * nz)?)
            .map_err(bad)?;
        Ok(Self {
            nx,
            nz,
            cap,
            present,
            power,
            ranges,
            numbers,
        })
    }
    fn append(&mut self, p: Option<&[Vec<Iv>]>, c: Option<&SlabCertificate>) -> CoreResult<()> {
        let n = self.nx + self.nz;
        let nz = self.nz;
        let nx = self.nx;
        if self.present.len() >= self.cap
            || p.is_some_and(|p| p.len() != n || p.iter().any(|r| r.len() != 5))
            || c.is_some_and(|c| {
                p.is_none()
                    || c.x_range.len() != nx
                    || c.z_range.len() != nz
                    || c.branch_root_box.len() != nz
                    || c.self_map_box.len() != nz
                    || c.self_map_center.len() != nz
                    || c.preconditioner.len() != nz * nz
            })
        {
            return Err(bad("slab arena shape/capacity"));
        }
        if let Some(p) = p {
            self.power.extend(
                p.iter()
                    .map(|row| std::array::from_fn(|j| row[j].as_pair())),
            );
        } else {
            self.power.extend(std::iter::repeat_n([[0.; 2]; 5], n));
        }
        if let Some(c) = c {
            for v in [&c.x_range, &c.z_range, &c.branch_root_box, &c.self_map_box] {
                self.ranges.extend_from_slice(v);
            }
            self.numbers.extend_from_slice(&c.self_map_center);
            self.numbers.extend_from_slice(&c.preconditioner);
        } else {
            self.ranges
                .extend(std::iter::repeat_n([0.; 2], nx + 3 * nz));
            self.numbers.extend(std::iter::repeat_n(0., nz + nz * nz));
        }
        self.present.push(p.is_some());
        Ok(())
    }
    pub(in crate::dae_evolution) fn power(&self, k: usize) -> &[PowerRow] {
        let n = self.nx + self.nz;
        if self.present[k] {
            &self.power[k * n..(k + 1) * n]
        } else {
            &[]
        }
    }
    pub(in crate::dae_evolution) fn x_range(&self, k: usize) -> &[Pair] {
        self.arrays(k).x_range
    }
    fn arrays(&self, k: usize) -> CertArrays<'_> {
        let nx = self.nx;
        let nz = self.nz;
        let m = nx + 3 * nz;
        let r = &self.ranges[k * m..(k + 1) * m];
        let s = nz + nz * nz;
        let a = &self.numbers[k * s..(k + 1) * s];
        CertArrays {
            x_range: &r[..nx],
            z_range: &r[nx..nx + nz],
            branch_root_box: &r[nx + nz..nx + 2 * nz],
            self_map_box: &r[nx + 2 * nz..],
            self_map_center: &a[..nz],
            preconditioner: &a[nz..],
        }
    }
    fn capacity_bytes(&self) -> usize {
        self.present.capacity() * std::mem::size_of::<bool>()
            + self.power.capacity() * std::mem::size_of::<PowerRow>()
            + self.ranges.capacity() * std::mem::size_of::<Pair>()
            + self.numbers.capacity() * 8
    }
    fn restore(self, slabs: &mut [SlabRecord]) {
        for (k, s) in slabs.iter_mut().enumerate() {
            s.power = self.power(k).iter().map(|r| r.to_vec()).collect();
            if let Some(c) = &mut s.certificate {
                self.arrays(k).restore(c);
            }
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum RetainedPowerRows<'a> {
    Owned(&'a [Vec<Pair>]),
    Flat(&'a [PowerRow]),
}
/// Complete read-only certificate. Same serialized fields as the legacy report.
#[derive(Serialize)]
pub struct CompactCertificateView<'a> {
    pub times: &'a [f64; 2],
    pub tube_radius: &'a f64,
    pub x_range: &'a [[f64; 2]],
    pub z_range: &'a [[f64; 2]],
    pub branch_root_box: &'a [[f64; 2]],
    pub self_map_box: &'a [[f64; 2]],
    pub self_map_center: &'a [f64],
    pub preconditioner: &'a [f64],
    pub branch_defect: &'a f64,
    pub condition_upper: &'a f64,
    pub original_g_upper: &'a f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residual_variant: &'a Option<de::SlabResidualBounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picard_endpoint: &'a Option<de::PicardEndpointBounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear_ls_endpoint: &'a Option<de::LinearLsEndpointBounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complex_endpoint: &'a Option<de::ComplexEndpointBounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aperiodic_endpoint: &'a Option<de::AperiodicEndpointBounds>,
    pub algebraic_defect: &'a f64,
    pub differential_defect: &'a f64,
    pub normalized_reduced_defect: &'a f64,
    pub reduced_mu_upper: &'a f64,
    pub zx_norm_upper: &'a f64,
    pub incoming: &'a f64,
    pub outgoing: &'a Option<f64>,
    pub whole_x_error: &'a Option<f64>,
    pub whole_z_error: &'a Option<f64>,
    pub green_used: &'a bool,
    pub global_qualified: &'a bool,
    pub global_reason: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conservation: &'a Option<de::ConservationReceipt>,
}
impl<'a> CompactCertificateView<'a> {
    fn new(c: &'a SlabCertificate, a: CertArrays<'a>) -> Self {
        Self {
            times: &c.times,
            tube_radius: &c.tube_radius,
            x_range: a.x_range,
            z_range: a.z_range,
            branch_root_box: a.branch_root_box,
            self_map_box: a.self_map_box,
            self_map_center: a.self_map_center,
            preconditioner: a.preconditioner,
            branch_defect: &c.branch_defect,
            condition_upper: &c.condition_upper,
            original_g_upper: &c.original_g_upper,
            residual_variant: &c.residual_variant,
            picard_endpoint: &c.picard_endpoint,
            linear_ls_endpoint: &c.linear_ls_endpoint,
            complex_endpoint: &c.complex_endpoint,
            aperiodic_endpoint: &c.aperiodic_endpoint,
            algebraic_defect: &c.algebraic_defect,
            differential_defect: &c.differential_defect,
            normalized_reduced_defect: &c.normalized_reduced_defect,
            reduced_mu_upper: &c.reduced_mu_upper,
            zx_norm_upper: &c.zx_norm_upper,
            incoming: &c.incoming,
            outgoing: &c.outgoing,
            whole_x_error: &c.whole_x_error,
            whole_z_error: &c.whole_z_error,
            green_used: &c.green_used,
            global_qualified: &c.global_qualified,
            global_reason: &c.global_reason,
            conservation: &c.conservation,
        }
    }
}
fn header(c: &SlabCertificate) -> SlabCertificate {
    SlabCertificate {
        times: c.times.clone(),
        tube_radius: c.tube_radius.clone(),
        x_range: Vec::new(),
        z_range: Vec::new(),
        branch_root_box: Vec::new(),
        self_map_box: Vec::new(),
        self_map_center: Vec::new(),
        preconditioner: Vec::new(),
        branch_defect: c.branch_defect.clone(),
        condition_upper: c.condition_upper.clone(),
        original_g_upper: c.original_g_upper.clone(),
        residual_variant: c.residual_variant.clone(),
        picard_endpoint: c.picard_endpoint.clone(),
        linear_ls_endpoint: c.linear_ls_endpoint.clone(),
        complex_endpoint: c.complex_endpoint.clone(),
        aperiodic_endpoint: c.aperiodic_endpoint.clone(),
        algebraic_defect: c.algebraic_defect.clone(),
        differential_defect: c.differential_defect.clone(),
        normalized_reduced_defect: c.normalized_reduced_defect.clone(),
        reduced_mu_upper: c.reduced_mu_upper.clone(),
        zx_norm_upper: c.zx_norm_upper.clone(),
        incoming: c.incoming.clone(),
        outgoing: c.outgoing.clone(),
        whole_x_error: c.whole_x_error.clone(),
        whole_z_error: c.whole_z_error.clone(),
        green_used: c.green_used.clone(),
        global_qualified: c.global_qualified.clone(),
        global_reason: c.global_reason.clone(),
        conservation: c.conservation.clone(),
    }
}

#[derive(Serialize)]
pub struct CompactSlabView<'a> {
    pub times: [f64; 2],
    pub native: NativePacketView<'a>,
    pub power: RetainedPowerRows<'a>,
    pub certificate: Option<CompactCertificateView<'a>>,
    pub reason: Option<&'a str>,
}
impl RetainedBuild {
    #[inline(never)]
    pub(in crate::dae_evolution) fn replay_slabs(
        &mut self,
        out: &mut EvolutionRun,
        e: &Engine,
        incoming: Option<f64>,
    ) -> CoreResult<Option<f64>> {
        if self.slab_plan == Some(SlabReplayRoute::Flat) {
            self.slab_arena = Some(SlabArena::new(
                e.src.nx(),
                e.src.nz(),
                out.driver.t.len() - 1,
            )?);
        }
        if e.src.nz() == 1 && e.scalar_callback() {
            let mut w = SlabReplayWorkspace::new(e.src.nx(), 1, None)?;
            self.replay_with(out, e, incoming, &mut w)
        } else {
            self.replay_vector(out, e, incoming)
        }
    }
    #[inline(never)]
    fn replay_vector(
        &mut self,
        out: &mut EvolutionRun,
        e: &Engine,
        incoming: Option<f64>,
    ) -> CoreResult<Option<f64>> {
        let mut vector = VectorReconstructionWorkspace::new();
        let mut w = SlabReplayWorkspace::new(e.src.nx(), e.src.nz(), Some(&mut vector))?;
        self.replay_with(out, e, incoming, &mut w)
    }
    fn replay_with(
        &mut self,
        out: &mut EvolutionRun,
        e: &Engine,
        mut incoming: Option<f64>,
        w: &mut SlabReplayWorkspace<'_>,
    ) -> CoreResult<Option<f64>> {
        let (route, storage) = e
            .route
            .workspace()
            .ok_or_else(|| bad("slab workspace route"))?;
        if storage != CertificateStorage::Local
            || self.route.native == NativeRetention::Owned
            || e.conservation.is_some()
            || e.warp.is_some()
        {
            return Err(bad("slab replay unsupported route"));
        }
        for k in 0..out.driver.t.len() - 1 {
            let times = [out.driver.t[k], out.driver.t[k + 1]];
            let v = native_packet_view(&out.driver, k)?;
            let basis = v.basis;
            match self.route.native {
                NativeRetention::BorrowedRows => w.dense_rows(v)?,
                NativeRetention::BorrowedComponent => w.dense_components(&out.driver, k)?,
                _ => unreachable!(),
            }
            let native = NativePacket {
                basis,
                coefficients: vec![],
                y0: vec![],
                y1: vec![],
                stages: vec![],
            };
            let mut receipt = ProjectionAttemptWork::default();
            let r = w.reconstruct(
                &e.src,
                times,
                &e.seed,
                &e.branch,
                e.policy.init,
                e.route.init(),
                &mut receipt,
            );
            update_audit(&e.audit, |a| {
                a.replay_reconstruction.record(r.is_ok());
                a.replay_nodes.merge(&receipt);
            });
            if let Err(err) = r {
                if let Some(a) = &mut self.slab_arena {
                    a.append(None, None)?;
                }
                out.slabs.push(SlabRecord {
                    times,
                    native,
                    power: vec![],
                    certificate: None,
                    reason: Some(err.to_string()),
                });
                incoming = None;
                continue;
            }
            for v in w.reconstruction_work().unwrap() {
                e.work.lock().map_err(|_| bad("work lock"))?.add(v, false);
            }
            let result = w.certify(&e.src, times, &e.branch, e.policy, incoming, route);
            update_audit(&e.audit, |a| {
                a.replay_certificate.record(result.is_ok());
                a.replay_global_certificates +=
                    usize::from(w.certificate().is_some_and(|c| c.global_qualified));
            });
            incoming = w
                .certificate()
                .filter(|c| c.global_qualified)
                .and_then(|c| c.outgoing);
            let reason = result.err().map(|e| e.to_string());
            let p = w.power().expect("successful reconstruction");
            let c = w.certificate();
            let (power, certificate) = if let Some(a) = &mut self.slab_arena {
                a.append(Some(p), c)?;
                (vec![], c.map(header))
            } else {
                (
                    p.iter()
                        .map(|row| row.iter().map(|v| v.as_pair()).collect())
                        .collect(),
                    c.cloned(),
                )
            };
            out.slabs.push(SlabRecord {
                times,
                native,
                power,
                certificate,
                reason,
            });
        }
        Ok(incoming)
    }
}
/// Explicit replay API. Both owning and flat slab variants retain exact
/// query/slab views. It never exposes the internal legacy placeholders.
pub struct SlabReplayRun {
    inner: RetainedEvolutionRun,
}
impl SlabReplayRun {
    pub fn driver(&self) -> &Solution {
        self.inner.driver()
    }
    pub fn initial(&self) -> &InitReport {
        self.inner.initial()
    }
    pub fn work(&self) -> &ProjectionWork {
        self.inner.work()
    }
    pub fn global_qualified(&self) -> bool {
        self.inner.global_qualified()
    }
    pub fn max_physical_error(&self) -> Option<f64> {
        self.inner.max_physical_error()
    }
    pub fn reason(&self) -> Option<&str> {
        self.inner.reason()
    }
    pub fn accepted_len(&self) -> usize {
        self.inner.accepted_len()
    }
    pub fn query_len(&self) -> usize {
        self.inner.query_len()
    }
    pub fn slabs_len(&self) -> usize {
        self.inner.slabs_len()
    }
    pub fn accepted(&self, k: usize) -> Option<RetainedQueryView<'_>> {
        self.inner.accepted(k)
    }
    pub fn retained_query(&self, k: usize) -> Option<RetainedQueryView<'_>> {
        self.inner.retained_query(k)
    }
    pub fn slab(&self, k: usize) -> CoreResult<CompactSlabView<'_>> {
        let s = self
            .inner
            .run
            .slabs
            .get(k)
            .ok_or_else(|| bad("compact slab index"))?;
        let a = self.inner.build.slab_arena.as_ref();
        let power = if let Some(a) = a {
            RetainedPowerRows::Flat(a.power(k))
        } else {
            RetainedPowerRows::Owned(&s.power)
        };
        let certificate = s.certificate.as_ref().map(|c| {
            CompactCertificateView::new(c, a.map_or_else(|| CertArrays::owned(c), |a| a.arrays(k)))
        });
        Ok(CompactSlabView {
            times: s.times,
            native: native_packet_view(&self.inner.run.driver, k)?,
            power,
            certificate,
            reason: s.reason.as_deref(),
        })
    }
    pub fn slab_numeric_capacity_bytes(&self) -> usize {
        self.inner.build.slab_arena.as_ref().map_or_else(
            || {
                self.inner
                    .run
                    .slabs
                    .iter()
                    .map(owned_slab_numeric_bytes)
                    .sum()
            },
            SlabArena::capacity_bytes,
        )
    }
    pub fn materialize(mut self) -> CoreResult<EvolutionRun> {
        if let Some(a) = self.inner.build.slab_arena.take() {
            a.restore(&mut self.inner.run.slabs);
        }
        self.inner.materialize()
    }
}
/// Numeric payload and owning polynomial row headers only; exclude slab
/// inline metadata, reasons and original native/driver data.
pub fn owned_slab_numeric_bytes(s: &SlabRecord) -> usize {
    s.power.capacity() * std::mem::size_of::<Vec<Pair>>()
        + s.power.iter().map(|r| r.capacity() * 16).sum::<usize>()
        + s.certificate.as_ref().map_or(0, |c| {
            16 * (c.x_range.capacity()
                + c.z_range.capacity()
                + c.branch_root_box.capacity()
                + c.self_map_box.capacity())
                + 8 * (c.self_map_center.capacity() + c.preconditioner.capacity())
        })
}
#[allow(clippy::too_many_arguments)]
pub fn evolve_epoch_slab_replay_audited(
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
    replay: SlabReplayRoute,
) -> (CoreResult<SlabReplayRun>, NativeResourceAudit) {
    evolve_epoch_slab_replay_planned(
        src,
        x,
        z,
        branch,
        epoch,
        opts,
        policy,
        route,
        grid,
        retention,
        replay,
        EnginePlan::Legacy,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn evolve_epoch_slab_replay_planned(
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
    replay: SlabReplayRoute,
    plan: EnginePlan,
) -> (CoreResult<SlabReplayRun>, NativeResourceAudit) {
    let a = Arc::new(Mutex::new(NativeResourceAudit::default()));
    let mut build = RetainedBuild::new(retention);
    build.slab_plan = Some(replay);
    build.engine_plan = plan;
    let result = if route.query_workspace().is_none()
        || !route
            .workspace()
            .is_some_and(|(_, s)| s == CertificateStorage::Local)
        || retention.native == NativeRetention::Owned
    {
        Err(bad(
            "slab replay requires WorkspaceQuery5 local and borrowed native",
        ))
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
    let audit = a.lock().expect("native audit lock").clone();
    (
        result.map(|run| SlabReplayRun {
            inner: RetainedEvolutionRun { run, build },
        }),
        audit,
    )
}
