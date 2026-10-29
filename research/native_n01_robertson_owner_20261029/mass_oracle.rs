use vigilode_n01_core_source_mirror::n01_mass_action_owner::N01MassActionOwner;
fn main() {
    println!(
        "case,k1,k2,k3,ra,rb,rc,la,lb,lc,sigma,w0,w1,w2,p0,p1,p2,x0,x1,x2,dj,db,eta,mu,margin,rho,forward,jvp_round,jvp_reference"
    );
    for case in 0..128_usize {
        let k = [
            (1 + (case % 5)) as f64 / 32.0,
            (2 + (case % 7)) as f64,
            (1 + (case % 9)) as f64,
        ];
        let r = [
            (24 + (case % 6)) as f64 / 32.0,
            (2 + (case % 4)) as f64 / 64.0,
            (2 + (case % 5)) as f64 / 64.0,
        ];
        let live = [
            r[0] + ((case % 3) as f64 - 1.0) / 512.0,
            r[1] + ((case % 4) as f64 - 2.0) / 512.0,
            r[2] + ((case % 5) as f64 - 2.0) / 1024.0,
        ];
        let sigma = (1 + (case % 3)) as f64 / 512.0;
        let w = [0.5, 1.0, 2.0];
        let p = [1.0, 2.0, 1.0];
        let x = [
            (1 + (case % 7)) as f64 / 64.0,
            ((case % 11) as f64 - 5.0) / 64.0,
            (1 + (case % 13)) as f64 / 128.0,
        ];
        let mut source = N01MassActionOwner::new(k, r, [1.0 / 32.0; 3], sigma, w, p, 0.0).unwrap();
        source.advance((case + 1) as f64 / 1024.0, live).unwrap();
        let trial = source.freeze();
        let mut out = [123.0; 3];
        let result = source
            .try_commit(&trial, &x, 1.0e5, 1.0e5, &mut out)
            .unwrap();
        assert!(result.accepted && out == x);
        let ev = source
            .source_owned_jvp_report(&trial, &[0.125, -0.25, 0.5])
            .unwrap();
        let mut cols = vec![format!("{case}")];
        for v in k
            .iter()
            .chain(&r)
            .chain(&live)
            .chain(&[sigma])
            .chain(&w)
            .chain(&p)
            .chain(&x)
            .chain(&[
                result.delta_j_upper,
                result.delta_b_upper,
                result.eta_apply_upper,
                result.mu_upper,
                result.margin_lower,
                result.original_residual_upper,
                result.physical_error_upper,
                ev.callback_rounding_upper,
                ev.reference_jvp_difference_upper,
            ])
        {
            cols.push(format!("{v:.17e}"));
        }
        println!("{}", cols.join(","));
    }
}