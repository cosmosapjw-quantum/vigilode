//! Frozen D3 research campaign. The production crate is not linked or edited.
//!
//! The original D2 admission module owns its read-only MXCSR check. Dense,
//! domain, directed arithmetic and the stage hook retain their safe boundary.
//! Every query is prepared before the accepted-state transaction commits.
//! File writes and diagnostic callback counters are external side effects;
//! an I/O error terminates this process and is not claimed to roll them back.

include!("support.rs");
mod directed;
mod domain;
mod integer_gate;
mod admission;
mod trajectory;
mod stage_safe;
mod dense;

use admission::{Accepted, Lease, Metric, Session};
use dense::{Certified, Curve, DenseError, Diagnostics, Model, Query};
use integer_gate::Decision;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;

const TF: f64 = 0.25;
const H0: f64 = 0.004;
const ATTEMPT_CAP: u64 = 20_000;
const REQUEST_COUNT: usize = 37;

#[derive(Clone, Copy)]
struct Spec { run: usize, alpha: f64, degree: usize, budget: f64 }

#[derive(Default)]
struct Counts {
    stage_rhs: u64,
    solves: u64,
    checked: u64,
    curve_builds: u64,
    curve_rhs_vectors: u64,
    analytic_jf_vectors: u64,
    certificate_requests: u64,
    query_attempts: u64,
    requested: usize,
    interior: usize,
    segments: usize,
    dense_rejected: u64,
    norm_rejected: u64,
    stage_rejected: u64,
    post_certificate_rejected: u64,
    forbidden_rhs: u64,
    max_uniform_full33: f64,
    max_query_radius: f64,
    min_h: Option<f64>,
    max_h: f64,
}

impl Counts {
    fn work(&mut self, work: stage_safe::Work) {
        self.stage_rhs += work.rhs;
        self.solves += work.solves;
        self.checked += work.checked;
    }
    fn step_size(&mut self, h: f64) {
        self.min_h = Some(self.min_h.map_or(h, |old| old.min(h)));
        self.max_h = self.max_h.max(h);
    }
}

struct Summary {
    spec: Spec,
    counts: Counts,
    status: &'static str,
    reason: Option<String>,
    accepted: u64,
    attempts: u64,
    rejected: u64,
    domain_rejected: u64,
    final_time: f64,
    final_incoming: [f64; 30],
}

#[derive(Clone, Copy)]
struct Checkpoint {
    accepted: Accepted,
    incoming: [f64; 30],
    cursor: usize,
    attempts: u64,
    rejected: u64,
}

fn checkpoint(session: &Session, incoming: &[f64; 30], cursor: usize) -> Checkpoint {
    Checkpoint {
        accepted: session.snapshot(), incoming: *incoming, cursor,
        attempts: session.attempts(), rejected: session.rejected(),
    }
}

fn same_values<const N: usize>(a: &[f64; N], b: &[f64; N]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

fn same_accepted(a: Accepted, b: Accepted) -> bool {
    a.t.to_bits() == b.t.to_bits() && same_values(&a.y, &b.y)
        && a.epoch == b.epoch
        && a.last_norm.map(f64::to_bits) == b.last_norm.map(f64::to_bits)
}

fn preserve(
    cp: &Checkpoint, session: &Session, incoming: &[f64; 30], cursor: usize,
    lease: Option<Lease>,
) {
    assert!(same_accepted(cp.accepted, session.snapshot()), "refusal changed accepted state");
    assert!(same_values(&cp.incoming, incoming), "refusal changed incoming radius");
    assert_eq!(cp.cursor, cursor, "refusal changed requested-output cursor");
    assert_eq!(session.attempts(), cp.attempts + u64::from(lease.is_some()),
        "attempt accounting changed across refusal");
}

fn quoted(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c <= '\u{1f}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Seventeen significant decimal digits, including for subnormal magnitudes.
fn number(x: f64) -> String {
    assert!(x.is_finite(), "non-finite evidence value");
    format!("{x:.16e}")
}

fn array(values: &[f64]) -> String {
    let mut out = String::from("[");
    for (i, &x) in values.iter().enumerate() {
        if i != 0 { out.push(','); }
        out.push_str(&number(x));
    }
    out.push(']');
    out
}

fn rows<const N: usize>(values: &[[f64; N]], width: usize) -> String {
    let mut out = String::from("[");
    for (i, row) in values.iter().enumerate() {
        if i != 0 { out.push(','); }
        out.push_str(&array(&row[..width]));
    }
    out.push(']');
    out
}

fn failure_record(
    writer: &mut impl Write, spec: Spec, cp: &Checkpoint, session: &Session,
    incoming: &[f64; 30], cursor: usize, lease: Option<Lease>,
    phase: &str, reason: &str, refinable: bool, work: stage_safe::Work,
) -> io::Result<()> {
    preserve(cp, session, incoming, cursor, lease);
    let after = session.snapshot();
    let next = lease.map_or_else(|| "null".into(), |l| number(l.next));
    let h = lease.map_or_else(|| "null".into(), |l| number(l.h));
    writeln!(writer,
        "{{\"run\":{},\"attempt\":{},\"phase\":{},\"reason\":{},\"refinable\":{},\"t0\":{},\"next\":{},\"h\":{},\"accepted_state_preserved\":true,\"incoming_preserved\":true,\"cursor_preserved\":true,\"attempt_budget_increased\":{},\"attempts_before\":{},\"attempts_after\":{},\"rejected_before\":{},\"rejected_after\":{},\"epoch_before\":{},\"epoch_after\":{},\"state_before\":{},\"state_after\":{},\"incoming_before\":{},\"incoming_after\":{},\"cursor_before\":{},\"cursor_after\":{},\"partial_rhs\":{},\"partial_solves\":{},\"partial_checked\":{},\"external_effects_rolled_back\":false}}",
        spec.run, session.attempts(), quoted(phase), quoted(reason), refinable,
        number(cp.accepted.t), next, h, lease.is_some(), cp.attempts,
        session.attempts(), cp.rejected, session.rejected(), cp.accepted.epoch,
        after.epoch, array(&cp.accepted.y), array(&after.y), array(&cp.incoming),
        array(incoming), cp.cursor, cursor, work.rhs, work.solves, work.checked)
}

fn abort_record(
    writer: &mut impl Write, spec: Spec, cp: &Checkpoint, session: &mut Session,
    incoming: &[f64; 30], cursor: usize, lease: Lease,
    refusal: &Refusal, work: stage_safe::Work,
) -> io::Result<()> {
    session.abort_stage(lease, refusal.refinable)
        .expect("the active lease must resolve exactly once");
    failure_record(writer, spec, cp, session, incoming, cursor, Some(lease),
        refusal.phase, &refusal.reason, refusal.refinable, work)
}

struct Refusal { phase: &'static str, reason: String, refinable: bool }

impl Refusal {
    fn fatal(phase: &'static str, reason: impl std::fmt::Debug) -> Self {
        Self { phase, reason: format!("{reason:?}"), refinable: false }
    }
    fn dense(phase: &'static str, error: DenseError) -> Self {
        let refinable = matches!(error,
            DenseError::DomainNotCertified { .. } | DenseError::Budget { .. }
            | DenseError::QueryDomain(domain::Violation::NegativeCompact { .. })
            | DenseError::QueryDomain(domain::Violation::GroupOverflow { .. })
            | DenseError::QueryDomain(domain::Violation::RepresentedNegative { .. }));
        Self { phase, reason: format!("{error:?}"), refinable }
    }
}

struct SavedQuery { t: f64, kind: &'static str, value: Query }

struct Prepared {
    certificate: Certified,
    diagnostics: Diagnostics,
    queries: Vec<SavedQuery>,
    cursor: usize,
    max_uniform: f64,
    max_query: f64,
}

fn guarded_query(
    certificate: &Certified, t: f64, kind: &'static str, counts: &mut Counts,
) -> Result<SavedQuery, Refusal> {
    admission::environment().map_err(|e| Refusal::fatal("query_environment", e))?;
    counts.query_attempts += 1;
    let value = certificate.query(t).map_err(|e| Refusal::dense("query", e))?;
    Ok(SavedQuery { t, kind, value })
}

fn prepare_dense(
    spec: Spec, model: &Model, cp: &Checkpoint, lease: Lease,
    trial: &stage_safe::Trial, requested: &[f64; REQUEST_COUNT], counts: &mut Counts,
) -> Result<Prepared, Refusal> {
    admission::environment().map_err(|e| Refusal::fatal("curve_environment", e))?;
    if model.alpha().to_bits() != spec.alpha.to_bits() {
        return Err(Refusal::fatal("model_binding", "stage/certificate alpha mismatch"));
    }
    counts.curve_builds += 1;
    counts.curve_rhs_vectors += if spec.degree == 3 { 2 } else { 4 };
    counts.analytic_jf_vectors += if spec.degree == 3 { 0 } else { 2 };
    let curve = Curve::new(model, spec.degree, cp.accepted.t, lease.next,
        &cp.accepted.y, &trial.candidate).map_err(|e| Refusal::dense("curve", e))?;
    let controls = curve.controls();
    let (t0, t1) = curve.times();
    if curve.degree() != spec.degree || t0.to_bits() != cp.accepted.t.to_bits()
        || t1.to_bits() != lease.next.to_bits()
        || !same_values(&controls[0], &cp.accepted.y)
        || !same_values(&controls[spec.degree], &trial.candidate)
    {
        return Err(Refusal::fatal("endpoint_binding", "curve endpoint bits differ"));
    }
    admission::environment().map_err(|e| Refusal::fatal("certificate_environment", e))?;
    counts.certificate_requests += 1;
    let certificate = curve.certify(&cp.incoming, spec.budget)
        .map_err(|e| Refusal::dense("certificate", e))?;
    let diagnostics = certificate.diagnostics();
    if diagnostics.alpha.to_bits() != spec.alpha.to_bits()
        || !same_values(&diagnostics.incoming, &cp.incoming)
        || certificate.budget().to_bits() != spec.budget.to_bits()
    {
        return Err(Refusal::fatal("certificate_binding", "alpha/E0/budget mismatch"));
    }
    let uniform = certificate.uniform_radius();
    let mut max_uniform = uniform.iter().copied().fold(0.0_f64, f64::max);
    let pair = directed::add_up(uniform[24], uniform[25])
        .map_err(|e| Refusal::fatal("uniform_full33", e))?;
    let triple = directed::add_up(
        directed::add_up(uniform[26], uniform[27])
            .map_err(|e| Refusal::fatal("uniform_full33", e))?, uniform[28])
        .map_err(|e| Refusal::fatal("uniform_full33", e))?;
    max_uniform = max_uniform.max(pair).max(triple);
    if !max_uniform.is_finite() || max_uniform > spec.budget {
        return Err(Refusal::fatal("uniform_full33", "certificate/summary disagreement"));
    }

    let mut cursor = cp.cursor;
    let mut queries = Vec::new();
    while cursor < requested.len() && requested[cursor] <= lease.next {
        if requested[cursor] <= cp.accepted.t {
            return Err(Refusal::fatal("output_cursor", "requested time not in (t0,t1]"));
        }
        queries.push(guarded_query(&certificate, requested[cursor], "requested", counts)?);
        cursor += 1;
    }
    let midpoint = cp.accepted.t + (lease.next - cp.accepted.t) * 0.5;
    if !midpoint.is_finite() || midpoint <= cp.accepted.t || midpoint >= lease.next {
        return Err(Refusal::fatal("interior_clock", admission::Fault::Clock));
    }
    queries.push(guarded_query(&certificate, midpoint, "interior", counts)?);
    admission::environment().map_err(|e| Refusal::fatal("post_query_environment", e))?;
    let max_query = queries.iter().flat_map(|q| q.value.radius)
        .fold(0.0_f64, f64::max);
    Ok(Prepared { certificate, diagnostics, queries, cursor, max_uniform, max_query })
}

fn save_segment(
    writer: &mut impl Write, spec: Spec, index: usize, prepared: &Prepared,
) -> io::Result<()> {
    let d = &prepared.diagnostics;
    write!(writer,
        "{{\"run\":{},\"idx\":{},\"alpha\":{},\"degree\":{},\"t0\":{},\"t1\":{},\"controls\":{},\"incoming\":{},\"rho\":{},\"endpoint\":{},\"uniform\":{},\"defect_lo\":{},\"defect_hi\":{},\"delta\":{},\"gain_lo\":{},\"gain_hi\":{},\"response_upper\":{},\"queries\":[",
        spec.run, index, number(d.alpha), d.degree, number(d.t0), number(d.t1),
        rows(&d.controls[..=d.degree], 30), array(&d.incoming), array(&d.rho),
        array(&d.endpoint), array(&d.uniform), rows(&d.defect_lo, 2*d.degree+1),
        rows(&d.defect_hi, 2*d.degree+1), array(&d.delta), array(&d.gain_lo),
        array(&d.gain_hi), array(&d.response_upper))?;
    for (i, query) in prepared.queries.iter().enumerate() {
        if i != 0 { write!(writer, ",")?; }
        write!(writer, "{{\"t\":{},\"kind\":{},\"center\":{},\"radius\":{},\"rounding\":{}}}",
            number(query.t), quoted(query.kind), array(&query.value.center),
            array(&query.value.radius), array(&query.value.rounding))?;
    }
    writeln!(writer, "]}}")
}

fn run_case(
    spec: Spec, requested: &[f64; REQUEST_COUNT],
    segments: &mut impl Write, failures: &mut impl Write,
) -> io::Result<Summary> {
    let mut counts = Counts::default();
    let mut incoming = [0.0; 30];
    let mut cursor = 0;
    let model = Model::new(spec.alpha).expect("frozen model alpha");
    let mut session = match Session::new_nonnegative(0.0, TF, trajectory::initial(), H0, ATTEMPT_CAP) {
        Ok(session) => session,
        Err(error) => return Ok(Summary { spec, counts, status: "FAILED",
            reason: Some(format!("session_initialization: {error:?}")), accepted: 0,
            attempts: 0, rejected: 0, domain_rejected: 0, final_time: 0.0,
            final_incoming: incoming }),
    };
    let metric = Metric { atol: [spec.budget * 1.0e-3; 33], rtol: [spec.budget; 33] };
    let mut terminal = None;

    while session.snapshot().t < TF {
        let cp = checkpoint(&session, &incoming, cursor);
        let max_rate = cp.accepted.y.iter().enumerate()
            .map(|(j, z)| trajectory::rate(j) + 2.0 * model.alpha() * z)
            .fold(0.0_f64, f64::max);
        // Nonlinear proposal heuristic only. Stage and dense checks grant admission.
        let cap = 1.8 / max_rate;
        if let Err(error) = session.cap_next_h(cap) {
            let reason = format!("proposal_cap: {error:?}");
            failure_record(failures, spec, &cp, &session, &incoming, cursor, None,
                "proposal_cap", &reason, false, stage_safe::Work::default())?;
            terminal = Some(reason); break;
        }
        let lease = match session.begin() {
            Ok(lease) => lease,
            Err(error) => {
                let reason = format!("begin: {error:?}");
                failure_record(failures, spec, &cp, &session, &incoming, cursor, None,
                    "begin", &reason, false, stage_safe::Work::default())?;
                terminal = Some(reason); break;
            }
        };
        assert_eq!(session.attempts(), cp.attempts + 1);
        counts.step_size(lease.h);
        let mut forbidden = 0_u64;
        let mut rhs = |_: usize, z: &[f64; 30], out: &mut [f64; 30]| {
            for j in 0..30 { out[j] = -trajectory::rate(j) * z[j] - model.alpha() * z[j] * z[j]; }
            Ok(())
        };
        let mut visit = |_: usize, _: &[f64; 30], inside: bool, called: bool| {
            if !inside && called { forbidden += 1; }
        };
        let trial_result = stage_safe::attempt(&cp.accepted.y, lease.h, model.alpha(),
            true, &mut rhs, &mut visit);
        counts.forbidden_rhs += forbidden;
        let trial = match trial_result {
            Ok(trial) => { counts.work(trial.work); trial }
            Err(error) => {
                counts.work(error.work);
                let refinable = matches!(error.kind, stage_safe::FailureKind::StageDomain
                    | stage_safe::FailureKind::Callback(stage_safe::CallbackFault::Recoverable));
                let refusal = Refusal { phase: "stage", reason: format!("{error:?}"), refinable };
                if refinable { counts.stage_rejected += 1; }
                abort_record(failures, spec, &cp, &mut session, &incoming, cursor, lease,
                    &refusal, error.work)?;
                if refinable { continue; }
                terminal = Some(refusal.reason); break;
            }
        };
        let assessment = (|| {
            let old = admission::lift(&cp.accepted.y)?;
            let new = admission::lift(&trial.candidate)?;
            admission::assess(&trial.error, &old, &new, &metric)
        })();
        let assessment = match assessment {
            Ok(assessment) => assessment,
            Err(error) => {
                let refusal = Refusal::fatal("preassessment", error);
                abort_record(failures, spec, &cp, &mut session, &incoming, cursor, lease,
                    &refusal, trial.work)?;
                terminal = Some(refusal.reason); break;
            }
        };
        if assessment.decision != Decision::Accept {
            let outcome = session.finish(lease, trial.candidate, trial.error, &metric);
            let (reason, refinable) = match outcome {
                Ok(outcome) if outcome.decision == Decision::Reject => {
                    counts.norm_rejected += 1;
                    (format!("local_norm: {:?}", outcome.decision), true)
                }
                Ok(outcome) if outcome.decision == Decision::Indeterminate =>
                    ("local_norm: Indeterminate".to_string(), false),
                Ok(_) => panic!("same-thread local assessment changed to Accept without a certificate"),
                Err(error) => (format!("local_finish: {error:?}"), false),
            };
            failure_record(failures, spec, &cp, &session, &incoming, cursor, Some(lease),
                "local_norm", &reason, refinable, trial.work)?;
            if refinable { continue; }
            terminal = Some(reason); break;
        }

        let prepared = match prepare_dense(spec, &model, &cp, lease, &trial, requested, &mut counts) {
            Ok(prepared) => prepared,
            Err(refusal) => {
                if refusal.refinable { counts.dense_rejected += 1; }
                abort_record(failures, spec, &cp, &mut session, &incoming, cursor, lease,
                    &refusal, trial.work)?;
                if refusal.refinable { continue; }
                terminal = Some(format!("{}: {}", refusal.phase, refusal.reason)); break;
            }
        };
        let outcome = session.finish(lease, trial.candidate, trial.error, &metric);
        match outcome {
            Ok(outcome) if outcome.decision == Decision::Accept => {
                let committed = session.snapshot();
                assert_eq!(committed.t.to_bits(), lease.next.to_bits());
                assert!(same_values(&committed.y, &trial.candidate));
                assert_eq!(committed.epoch, cp.accepted.epoch + 1);
                assert!(same_values(&incoming, &prepared.diagnostics.incoming));
                incoming = prepared.certificate.endpoint_radius();
                cursor = prepared.cursor;
                counts.requested += cursor - cp.cursor;
                counts.interior += 1;
                counts.max_uniform_full33 = counts.max_uniform_full33.max(prepared.max_uniform);
                counts.max_query_radius = counts.max_query_radius.max(prepared.max_query);
                save_segment(segments, spec, counts.segments, &prepared)?;
                counts.segments += 1;
            }
            other => {
                let (reason, refinable) = match other {
                    Ok(outcome) if outcome.decision == Decision::Reject => {
                        counts.post_certificate_rejected += 1;
                        (format!("post_certificate: decision={:?}, norm={:?}, domain={:?}",
                            outcome.decision, outcome.norm_decision, outcome.domain_violation), true)
                    }
                    Ok(outcome) => (format!("post_certificate: {:?}", outcome.decision), false),
                    Err(error) => (format!("post_certificate_finish: {error:?}"), false),
                };
                failure_record(failures, spec, &cp, &session, &incoming, cursor, Some(lease),
                    "post_certificate", &reason, refinable, trial.work)?;
                if !refinable { terminal = Some(reason); break; }
            }
        }
    }

    if terminal.is_none() {
        let complete = session.snapshot().t.to_bits() == TF.to_bits()
            && cursor == REQUEST_COUNT && counts.requested == REQUEST_COUNT
            && counts.interior == counts.segments
            && session.accepted() == counts.segments as u64
            && session.attempts() == session.accepted() + session.rejected()
            && session.rejected() == counts.dense_rejected + counts.norm_rejected
                + counts.stage_rejected + counts.post_certificate_rejected
            && counts.forbidden_rhs == 0
            && counts.max_uniform_full33 <= spec.budget && counts.max_query_radius <= spec.budget;
        if !complete { terminal = Some("frozen completion criteria failed".to_string()); }
    }
    Ok(Summary {
        spec, counts, status: if terminal.is_none() { "COMPLETE" } else { "FAILED" },
        reason: terminal, accepted: session.accepted(), attempts: session.attempts(),
        rejected: session.rejected(), domain_rejected: session.domain_rejected(),
        final_time: session.snapshot().t, final_incoming: incoming,
    })
}

fn save_summary(writer: &mut impl Write, runs: &[Summary], requested: &[f64; REQUEST_COUNT]) -> io::Result<()> {
    writeln!(writer, "{{\"schema\":\"vigilode-d3-dense-campaign-v1\",\"attempt_cap\":{},\"h0\":{},\"guard\":true,\"rate_cap_role\":\"nonlinear proposal heuristic\",\"output_policy\":\"queries prepared before accepted commit\",\"runs\":[", ATTEMPT_CAP, number(H0))?;
    let lambda: [f64; 30] = std::array::from_fn(trajectory::rate);
    for (i, run) in runs.iter().enumerate() {
        if i != 0 { writeln!(writer, ",")?; }
        let c = &run.counts;
        let reason = run.reason.as_deref().map_or_else(|| "null".into(), quoted);
        let min_h = c.min_h.map_or_else(|| "null".into(), number);
        write!(writer,
            "{{\"run\":{},\"alpha\":{},\"degree\":{},\"curve_budget\":{},\"initial_time\":0,\"initial_state\":{},\"lambda\":{},\"tf\":{},\"status\":{},\"reason\":{},\"accepted\":{},\"segments\":{},\"attempts\":{},\"rejected\":{},\"stage_rhs\":{},\"solves\":{},\"stage_checks\":{},\"curve_builds\":{},\"curve_rhs_vectors\":{},\"analytic_jf_vectors\":{},\"certificate_requests\":{},\"query_attempts\":{},\"query_counts\":{{\"requested\":{},\"interior\":{}}},\"requested_times\":{},\"dense_rejected\":{},\"norm_rejected\":{},\"stage_rejected\":{},\"post_certificate_rejected\":{},\"domain_rejected\":{},\"forbidden_rhs\":{},\"max_uniform_full33\":{},\"max_query_radius\":{},\"min_h\":{},\"max_h\":{},\"final_time\":{},\"final_incoming\":{}}}",
            run.spec.run, number(run.spec.alpha), run.spec.degree, number(run.spec.budget),
            array(&trajectory::initial()), array(&lambda), number(TF), quoted(run.status), reason,
            run.accepted, c.segments, run.attempts, run.rejected, c.stage_rhs, c.solves,
            c.checked, c.curve_builds, c.curve_rhs_vectors, c.analytic_jf_vectors,
            c.certificate_requests, c.query_attempts, c.requested, c.interior, array(requested),
            c.dense_rejected, c.norm_rejected, c.stage_rejected, c.post_certificate_rejected,
            run.domain_rejected, c.forbidden_rhs, number(c.max_uniform_full33), number(c.max_query_radius),
            min_h, number(c.max_h), number(run.final_time), array(&run.final_incoming))?;
    }
    writeln!(writer, "\n]}}")
}

fn create_new(path: &Path) -> io::Result<BufWriter<File>> {
    Ok(BufWriter::new(OpenOptions::new().write(true).create_new(true).open(path)?))
}

fn execute() -> io::Result<bool> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() > 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "usage: campaign_dense [output-directory]"));
    }
    let output = args.get(1).map_or_else(|| Path::new("evidence"), |p| Path::new(p));
    fs::create_dir_all(output)?;
    // Reserve all names before the first scientific trial. Existing evidence is never overwritten.
    let mut segments = create_new(&output.join("segments.jsonl"))?;
    let mut failures = create_new(&output.join("failures.jsonl"))?;
    let mut summary_file = create_new(&output.join("SUMMARY.json"))?;
    let requested: [f64; REQUEST_COUNT] = std::array::from_fn(|i|
        if i + 1 == REQUEST_COUNT { TF } else { TF * ((i + 1) as f64) / (REQUEST_COUNT as f64) });
    let mut runs = Vec::new();
    for alpha in [0.0, 4.0] {
        for degree in [3, 5] {
            for budget in [2.0e-6, 2.0e-8] {
                let spec = Spec { run: runs.len(), alpha, degree, budget };
                let run = run_case(spec, &requested, &mut segments, &mut failures)?;
                segments.flush()?;
                failures.flush()?;
                println!("run={} alpha={} degree={} budget={:.2e} status={} accepted={} attempts={} rejected={} stage_rhs={} solves={} dense_rejected={} queries={}/{}",
                    spec.run, alpha, degree, budget, run.status, run.accepted,
                    run.attempts, run.rejected, run.counts.stage_rhs, run.counts.solves,
                    run.counts.dense_rejected, run.counts.requested, run.counts.interior);
                runs.push(run);
            }
        }
    }
    save_summary(&mut summary_file, &runs, &requested)?;
    summary_file.flush()?;
    Ok(runs.iter().all(|run| run.status == "COMPLETE"))
}

fn main() -> std::process::ExitCode {
    match execute() {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::FAILURE,
        Err(error) => {
            eprintln!("campaign external I/O/configuration failure (no rollback claim): {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
