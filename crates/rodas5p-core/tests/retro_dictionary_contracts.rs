#![cfg(feature = "retro-dictionary")]
use rodas5p_core::retro_dictionary::Dictionary;
#[test]
fn authentic_dictionary_is_available_in_rust() {
    let d = Dictionary::get().unwrap();
    assert_eq!(d.source_count(), 10);
}

use rodas5p_core::retro_dictionary::RealMode;
use rodas5p_core::{direct_solve, directed::Interval};
use serde_json::json;
fn pt(x: f64) -> Interval {
    Interval::point(x).unwrap()
}
fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
fn record(v: Interval) -> [String; 2] {
    [bits(v.lo), bits(v.hi)]
}
#[test]
fn full_source_projection_and_domain() {
    let d = Dictionary::get().unwrap();
    assert!(std::ptr::eq(d, Dictionary::get().unwrap()));
    assert_eq!(
        d.packed_digest(),
        include_str!(
            "../../../research/retro_dictionary_foundation_20261125/PROJECTION_SHA256.txt"
        )
        .trim()
    );
    assert_eq!(d.cell_count(), 57);
    assert_eq!(d.products().len(), 285);
    assert_eq!(d.picard_words().len(), 363);
    assert_eq!(
        d.complex_coefficients().iter().map(Vec::len).sum::<usize>(),
        369
    );
    for s in d.sources() {
        s.verify(s.bytes).unwrap();
        let mut changed = s.bytes.to_vec();
        changed[0] ^= 1;
        assert!(s.verify(&changed).is_err());
    }
    for x in [-1., 16385., f64::NAN, f64::INFINITY] {
        assert!(
            d.phi(Interval { lo: x, hi: x }, 0, RealMode::Hybrid)
                .is_err()
        );
    }
    assert!(d.phi(pt(1.), 9, RealMode::Hybrid).is_err());
    assert!(d.moment(pt(1.), 8, 0, RealMode::Hybrid).is_err());
    assert!(d.moment(pt(1.), 3, 4, RealMode::Hybrid).is_err());
    assert!(d.moment(pt(63.), 1, 0, RealMode::Scaled).is_err());
    assert!(d.basis(4).is_err());
    assert!(d.legendre(7).is_err());
    assert!(d.complex_phi(1., 0.).is_err());
    assert!(
        d.phi(Interval { lo: 2., hi: 1. }, 0, RealMode::Hybrid)
            .is_err()
    );
    println!("PROJECTION {}", d.packed_digest());
}
#[test]
fn all_cells_real_and_scaled_consumers() {
    let d = Dictionary::get().unwrap();
    let mut xs = vec![0.];
    let mut a = 0.;
    for b in d.cell_edges() {
        xs.extend([(a + b) / 2., b, b.next_down().max(0.)]);
        if b < 16384. {
            xs.push(b.next_up());
        }
        a = b;
    }
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    for &x in &xs {
        let phi = d.phi_all_fast(x).unwrap();
        let moment = d.moments_all_fast(x).unwrap();
        let phid = (0..9)
            .map(|j| d.phi(pt(x), j, RealMode::TableDirected).unwrap())
            .collect::<Vec<_>>();
        let phih = (0..9)
            .map(|j| d.phi(pt(x), j, RealMode::Hybrid).unwrap())
            .collect::<Vec<_>>();
        let mut md = Vec::new();
        let mut mh = Vec::new();
        let mut ms = Vec::new();
        for p in 0..8 {
            let mut sum = pt(0.);
            for k in 0..=p {
                let j = p * (p + 1) / 2 + k;
                assert_eq!(
                    record(moment[j]),
                    record(d.moment(pt(x), p, k, RealMode::TableFast).unwrap())
                );
                let b = d.moment(pt(x), p, k, RealMode::TableDirected).unwrap();
                let h = d.moment(pt(x), p, k, RealMode::Hybrid).unwrap();
                sum = sum.add(h).unwrap();
                md.push(b);
                mh.push(h);
                if x >= 64. {
                    ms.push(d.scaled_moment_fast(x, p, k).unwrap());
                }
            }
            assert!(sum.lo <= phih[1].hi && phih[1].lo <= sum.hi);
        }
        for (j, v) in phi.iter().enumerate() {
            assert_eq!(
                record(*v),
                record(d.phi(pt(x), j, RealMode::TableFast).unwrap())
            );
        }
        println!(
            "REAL {}",
            json!({"x":bits(x),"phi":phi.map(record),"moment":moment.map(record).to_vec(),"direct_phi":phid.into_iter().map(record).collect::<Vec<_>>(),"hybrid_phi":phih.into_iter().map(record).collect::<Vec<_>>(),"direct_moment":md.into_iter().map(record).collect::<Vec<_>>(),"hybrid_moment":mh.into_iter().map(record).collect::<Vec<_>>(),"scaled_fast":ms.into_iter().map(record).collect::<Vec<_>>()})
        );
    }
    let v = d
        .phi(Interval::new(63.9, 64.1).unwrap(), 1, RealMode::Hybrid)
        .unwrap();
    assert!(v.contains((1. - (-64_f64).exp()) / 64.));
}
#[test]
fn basis_and_legendre_and_ordered_words_are_computational() {
    let d = Dictionary::get().unwrap();
    for m in [3, 5, 7] {
        let b = d.basis(m).unwrap();
        for (i, row) in b.monomial.iter().enumerate() {
            for (j, t) in b.nodes.iter().enumerate() {
                let v = row
                    .iter()
                    .rev()
                    .try_fold(pt(0.), |a, c| a.mul(t.enclosure)?.add(c.enclosure))
                    .unwrap();
                assert!(v.contains(if i == j { 1. } else { 0. }));
            }
        }
        for a in [0., 0.1, 1., 63., 64., 1000., 16384.] {
            for theta in [0., 0.125, 0.5, 1.] {
                for l in 0..b.damping_lift.len() {
                    let w = (0..m)
                        .map(|i| b.green_weight(d, i, a, theta, l, RealMode::Hybrid).unwrap())
                        .collect::<Vec<_>>();
                    println!(
                        "GREEN {}",
                        json!({"m":m,"a":bits(a),"theta":bits(theta),"l":l,"weights":w.into_iter().map(record).collect::<Vec<_>>()})
                    );
                }
            }
        }
    }
    let mut max_error = 0.0_f64;
    for p in 1..=6 {
        let l = d.legendre(p).unwrap();
        for (i, row) in l.g.iter().enumerate() {
            for (j, a) in row.iter().enumerate() {
                assert!(
                    a.enclosure
                        .contains(if i == j { 1. / (2 * i + 1) as f64 } else { 0. })
                );
            }
        }
        for k in 0..2 * l.gauss_nodes.len() {
            let q = l
                .gauss_nodes
                .iter()
                .zip(&l.gauss_weights)
                .map(|(&x, &w)| w * x.powi(k as i32))
                .sum::<f64>();
            assert!((q - 1. / (k + 1) as f64).abs() < 5e-14);
        }
        for z in [0., -0.125, -1., -4., -100., -16384.] {
            let r = l.endpoint_response(z).unwrap();
            let (a, b) = l.normal_equations(z).unwrap();
            let c = direct_solve(&a, &b).unwrap();
            let v = 1. + l.ell.iter().zip(c).map(|(a, b)| a.value * b).sum::<f64>();
            let delta = (v - (r.lo * 0.5 + r.hi * 0.5)).abs();
            max_error = max_error.max(delta);
            assert!(delta < 1e-7);
            println!(
                "LS {}",
                json!({"p":p,"z":bits(z),"value":record(r),"direct":bits(v),"difference":delta})
            );
        }
    }
    println!("LS_MAX_DIFF {max_error:e}");
    for i in 0..9 {
        for j in 0..9 {
            let mut a = [0.; 9];
            let mut b = [0.; 9];
            a[i] = 1.;
            b[j] = 1.;
            let t = d.legendre_product(&a, &b).unwrap();
            for k in 0..17 {
                if k < i.abs_diff(j) || k > i + j || (i + j - k) % 2 != 0 {
                    assert!(t[k].contains(0.) && t[k].mag() == 0.);
                }
            }
            let sum = t.iter().try_fold(pt(0.), |a, &b| a.add(b)).unwrap();
            assert!(sum.contains(1.));
            println!("PRODUCT {}", json!({"i":i,"j":j,"values":t.map(record)}));
        }
    }
    for w in d.picard_words() {
        let denom = (0..w.word.len())
            .map(|i| w.word[i..].iter().map(|&k| k + 1).sum::<usize>())
            .product::<usize>();
        assert!(w.coefficient.enclosure.contains(1. / denom as f64));
    }
}
#[test]
fn complex_actual_table_candidates() {
    let d = Dictionary::get().unwrap();
    for re in [0., -0.1, -4., -100.] {
        for im in [0., 1e-12, 0.1, 3.999999, 4., 4.000001, 10., 100., 10000.] {
            let r = d.complex_phi(re, im).unwrap();
            println!(
                "COMPLEX {}",
                json!({"re":bits(re),"im":bits(im),"values":r.values.map(|x|x.map(bits)),"taylor":r.used_taylor})
            );
        }
    }
}
