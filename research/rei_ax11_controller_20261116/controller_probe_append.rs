// Append-only diagnostic seam for a PRIVATE copy of exact original axbi_driver.rs.
// It does not replace tentative(), error_metric(), adaptive_step() or any output.
#[derive(Debug)]
pub struct Ax11TripletProbe {
    pub full: DriverState,
    pub twohalf: DriverState,
    pub components: [f64; 6],
    pub metric: f64,
    pub rhs_per_trial: [usize; 3],
    pub trial_ns: [u128; 3],
}
impl Driver {
    pub fn ax11_checkpoint_text(&self, s: &DriverState) -> Result<String, ForwardError> {
        self.checkpoint_text(s)
    }
    pub fn ax11_components(&self, full: &DriverState, half: &DriverState) -> Result<[f64; 6], ForwardError> {
        let mut out = [0.0; 6];
        for i in 0..3 {
            out[i] = (full.gas.fractions[i] - half.gas.fractions[i]).abs()
                / half.gas.fractions[i].abs().max(1e-5);
        }
        let w = w_erg_per_h_to_ev(half.gas.w_erg_per_h)?;
        out[3] = (w_erg_per_h_to_ev(full.gas.w_erg_per_h)? - w).abs() / w.abs().max(1e-6);
        let ne = self.energies.len();
        let (n, u) = self.photon_inventory(half);
        let dn = full.photons.iter().zip(&half.photons).map(|(a,b)| (a-b).abs()).sum::<f64>();
        let du = full.photons.iter().zip(&half.photons).enumerate()
            .map(|(i,(a,b))| (a-b).abs()*self.energies[i%ne]).sum::<f64>();
        out[4] = dn / n.max(1e-8);
        out[5] = du / u.max(1e-8);
        for value in out { f(value)?; }
        Ok(out)
    }
    pub fn ax11_triplet_probe(&self, old: &DriverState, dln_a: f64) -> Result<Ax11TripletProbe, ForwardError> {
        if !dln_a.is_finite() || dln_a <= 0.0 { return Err(err("AX11_PROBE_STEP")); }
        let to = (old.a * dln_a.exp()).min(1.0/(1.0+self.config.end_zbar));
        let mid = (old.a * to).sqrt();
        let mut counter = old.rhs_evaluations;
        let mut previous = counter;
        let t = Instant::now();
        let full = self.tentative(old, to, &mut counter)?;
        let t0 = t.elapsed().as_nanos(); let n0 = counter-previous; previous=counter;
        let t = Instant::now();
        let first = self.tentative(old, mid, &mut counter)?;
        let t1 = t.elapsed().as_nanos(); let n1 = counter-previous; previous=counter;
        let t = Instant::now();
        let twohalf = self.tentative(&first, to, &mut counter)?;
        let t2 = t.elapsed().as_nanos(); let n2 = counter-previous;
        let components = self.ax11_components(&full, &twohalf)?;
        let metric = self.error_metric(&full, &twohalf)?;
        let recomposed = components.into_iter().fold(0.0_f64, f64::max);
        if recomposed.to_bits() != metric.to_bits() { return Err(err("AX11_ORIGINAL_METRIC_CHANGED")); }
        Ok(Ax11TripletProbe { full, twohalf, components, metric,
            rhs_per_trial: [n0,n1,n2], trial_ns: [t0,t1,t2] })
    }
}
#[cfg(test)]
mod vigilode_ax11_probe_tests {
    use super::*;
    #[test]
    fn six_component_max_matches_original_metric_including_photon_floor() {
        let (d, h) = Driver::new(DriverConfig::baseline()).unwrap();
        let mut f = h.clone();
        f.gas.fractions[0] += 1e-8;
        f.gas.w_erg_per_h *= 1.0001;
        f.photons[0] = 1e-9;
        let e = d.ax11_components(&f, &h).unwrap();
        let original = d.error_metric(&f, &h).unwrap();
        let max = e.into_iter().fold(0.0_f64, f64::max);
        assert_eq!(max.to_bits(), original.to_bits(), "component maximum changed original controller metric");
        assert!(e[5] > e[4] && e[4] > e[3], "photon-energy and photon-number floors absent");
        assert_eq!(e[1], 0.0);
        assert_eq!(e[2], 0.0);
    }
    #[test]
    fn source_triplet_diagnostic_preserves_original_parent_and_decision_metric() {
        let (d, s) = Driver::new(DriverConfig::baseline()).unwrap();
        let before = d.checkpoint_text(&s).unwrap();
        let r = d.ax11_triplet_probe(&s, 3.814697265625e-10).unwrap();
        assert_eq!(before, d.checkpoint_text(&s).unwrap(), "read-only diagnostic mutated source parent");
        assert_eq!(r.metric.to_bits(), d.error_metric(&r.full, &r.twohalf).unwrap().to_bits());
        assert!(r.rhs_per_trial.iter().all(|n| *n > 0));
        assert!(r.metric < d.config.local_target);
    }
}
