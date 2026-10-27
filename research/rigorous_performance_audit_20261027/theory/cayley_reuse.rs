//! Research primitive for native W = I/a - J and cached P = I/a0 - J0.
//! No solver default is changed. A caller must establish dissipativity and
//! drift on a declared full domain, in the SAME fixed Hilbert metric.
//! Scalar bounds round outward. They do NOT enclose errors of an external
//! floating-point factor solve or residual callback.
//! Enclosure premise: correctly rounded binary64 basic operations, gradual
//! underflow, and no fast-math/reassociation. Identity tags do not prove the
//! caller's full-domain dissipativity/drift declaration.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub model_epoch: u64,
    pub metric_epoch: u64,
    pub domain_id: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct DeclaredBounds {
    pub identity: Identity,
    /// Re <v,J0 v> <= -alpha ||v||^2; units inverse time.
    pub cached_dissipation: f64,
    /// ||J-J0|| in the same metric on the declared domain; inverse time.
    pub jacobian_drift: f64,
    /// Caller reference to a mathematical proof/provider, not a sampled test.
    pub evidence_reference: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct Proposal {
    pub omega: f64,
    /// Includes represented omega error and declared drift; assumes exact P^-1.
    pub contraction_upper: f64,
    pub omega_rounding_allowance: f64,
    pub shift_only_upper: f64,
    pub drift_upper: f64,
    /// Strict positivity of 1-q is required before using a contraction bound.
    pub admitted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal { InvalidInput, StaleIdentity, MissingEvidence, ArithmeticRange }

fn nonnegative(x: f64) -> bool { x.is_finite() && x >= 0.0 }
fn up_add(x: f64, y: f64) -> f64 {
    if x == 0.0 { y } else if y == 0.0 { x } else { (x+y).next_up() }
}
fn up_mul(x: f64, y: f64) -> f64 {
    if x == 0.0 || y == 0.0 { 0.0 } else { (x*y).next_up() }
}
fn down_mul(x: f64, y: f64) -> f64 {
    if x == 0.0 || y == 0.0 { 0.0 } else { (x*y).next_down().max(0.0) }
}
fn gain_upper(a: f64, alpha: f64) -> f64 {
    // If a*alpha overflows, its lower enclosure is MAX, still a valid lower
    // denominator. No rounded infinity is treated as an exact real number.
    let product=down_mul(a,alpha);
    let denominator=(1.0+product).next_down().max(1.0);
    (a/denominator).next_up()
}

/// Construct an exact-inverse contraction proposal. IDs bind the proof domain.
pub fn propose(a: f64, a0: f64, query: Identity, bounds: DeclaredBounds)
    -> Result<Proposal,Refusal> {
    if !a.is_finite() || !a0.is_finite() || a <= 0.0 || a0 <= 0.0
        || !nonnegative(bounds.cached_dissipation) || !nonnegative(bounds.jacobian_drift) {
        return Err(Refusal::InvalidInput);
    }
    if query != bounds.identity { return Err(Refusal::StaleIdentity); }
    if bounds.evidence_reference == 0 { return Err(Refusal::MissingEvidence); }
    let smaller=a.min(a0);
    let larger=a.max(a0);
    let r=smaller/larger;
    let rlo=r.next_down().max(0.0);
    let rhi=r.next_up().min(1.0);
    let den_lo=(1.0+rlo).next_down().max(1.0);
    let den_hi=(1.0+rhi).next_up();
    let shift=((1.0-rlo).next_up()/den_lo).next_up();
    let (omega,lo,hi)=if a<=a0 {
        (2.0*r/(1.0+r),
         (down_mul(2.0,rlo)/(1.0+rlo).next_up()).next_down().max(0.0),
         (up_mul(2.0,rhi)/(1.0+rhi).next_down()).next_up())
    } else {
        (2.0/(1.0+r), (2.0/den_hi).next_down(), (2.0/den_lo).next_up())
    };
    let eps=(omega-lo).abs().max((hi-omega).abs()).next_up();
    let system_norm=if a>=a0 {1.0} else {(a0/a).next_up()};
    if !system_norm.is_finite() || omega == 0.0 {
        return Err(Refusal::ArithmeticRange);
    }
    let scalar=up_mul(eps,system_norm);
    let drift=up_mul(omega,up_mul(gain_upper(a0,bounds.cached_dissipation),
                               bounds.jacobian_drift));
    let q=up_add(up_add(shift,scalar),drift);
    if !q.is_finite() { return Err(Refusal::ArithmeticRange); }
    Ok(Proposal {omega,contraction_upper:q,omega_rounding_allowance:scalar,
        shift_only_upper:shift,drift_upper:drift,admitted:q<1.0})
}

/// Original scaled residual -> state error, given a TRUE actual-J dissipation
/// bound and a TRUE residual upper bound including residual-evaluation error.
pub fn residual_state_bound(a:f64, actual_alpha:f64, residual_upper:f64)
    -> Result<f64,Refusal> {
    if !a.is_finite() || a<=0.0 || !nonnegative(actual_alpha)
        || !nonnegative(residual_upper) { return Err(Refusal::InvalidInput); }
    let bound=up_mul(gain_upper(a,actual_alpha),residual_upper);
    if bound.is_finite() {Ok(bound)} else {Err(Refusal::ArithmeticRange)}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn declared()->DeclaredBounds {
        DeclaredBounds {identity:Identity {model_epoch:3,metric_epoch:2,domain_id:1},
            cached_dissipation:0.0,jacobian_drift:0.0,evidence_reference:17}
    }
    #[test]
    fn same_shift_and_scalar_endpoints_are_enclosed() {
        let b=declared();
        for ratio in [0.1,0.25,0.5,0.9,1.0,1.1,2.0,4.0,10.0] {
            let p=propose(ratio,1.0,b.identity,b).unwrap();
            let q=(ratio-1.0).abs()/(ratio+1.0);
            assert!(p.contraction_upper>=q && p.admitted);
            let slow=(1.0-p.omega/ratio).abs();
            let infinitely_stiff=(1.0-p.omega).abs();
            assert!(slow<=p.contraction_upper && infinitely_stiff<=p.contraction_upper);
        }
    }
    #[test]
    fn large_equal_shifts_avoid_sum_overflow() {
        let b=declared();
        let p=propose(f64::MAX,f64::MAX,b.identity,b).unwrap();
        assert_eq!(p.omega,1.0);
        assert!(p.admitted && p.contraction_upper<1e-13);
    }
    #[test]
    fn unrepresentable_ratio_is_refused() {
        let b=declared();
        assert_eq!(propose(f64::MIN_POSITIVE,f64::MAX,b.identity,b).unwrap_err(),
                   Refusal::ArithmeticRange);
    }
    #[test]
    fn wrong_metric_or_domain_refuses() {
        let b=declared();
        let mut q=b.identity;q.metric_epoch+=1;
        assert_eq!(propose(1.0,1.0,q,b).unwrap_err(),Refusal::StaleIdentity);
        q=b.identity;q.domain_id+=1;
        assert_eq!(propose(1.0,1.0,q,b).unwrap_err(),Refusal::StaleIdentity);
    }
    #[test]
    fn drift_can_remove_convergence_admission() {
        let mut b=declared();b.jacobian_drift=2.0;
        assert!(!propose(1.0,1.0,b.identity,b).unwrap().admitted);
    }
    #[test]
    fn dimensional_residual_factor_and_zero_are_preserved() {
        let bound=residual_state_bound(0.1,0.0,2.0).unwrap();
        assert!(bound>=0.2 && bound<0.200000000000001);
        let damped=residual_state_bound(1.0,2.0,1.0).unwrap();
        assert!(damped>=1.0/3.0 && damped<0.333333333333334);
        assert_eq!(residual_state_bound(1.0,2.0,0.0).unwrap(),0.0);
    }
    #[test]
    fn nonfinite_and_negative_declarations_refuse() {
        let mut b=declared();b.cached_dissipation=-1.0;
        assert_eq!(propose(1.0,1.0,b.identity,b).unwrap_err(),Refusal::InvalidInput);
        assert_eq!(residual_state_bound(1.0,0.0,f64::NAN),Err(Refusal::InvalidInput));
    }
}
