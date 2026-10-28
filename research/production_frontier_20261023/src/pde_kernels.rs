//! Research-only reusable conservative graph diffusion kernels.
//!
//! No derivative callback, assembled Jacobian, Newton iteration or heap allocation
//! inside a successful step. Graph and workspace setup allocate O(N+E) storage.
//! Mathematical contracts cover autonomous reversible scalar diffusion only;
//! this is not yet connected to VigilODE adaptivity, events or remeshing.
#![allow(dead_code)]

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidGraph,
    InvalidInput,
    InvalidTime,
    InvalidTolerance,
    Dimension,
    Stability,
    NonFinite,
}
#[derive(Clone)]
pub struct Graph {
    weights: Vec<f64>,
    edges: Vec<(usize, usize, f64)>,
    exit: Vec<f64>,
    q: f64,
}
pub struct Workspace {
    current: Vec<f64>,
    v0: Vec<f64>,
    v1: Vec<f64>,
    v2: Vec<f64>,
    temp: Vec<f64>,
    accum: Vec<f64>,
}
#[derive(Debug)]
pub struct Work {
    pub actions: usize,
    pub macrosteps: usize,
    /// Analytic truncation only; floating-point roundoff is excluded.
    pub relative_truncation_bound: f64,
}
impl Workspace {
    pub fn new(n: usize) -> Self {
        Self {
            current: vec![0.; n],
            v0: vec![0.; n],
            v1: vec![0.; n],
            v2: vec![0.; n],
            temp: vec![0.; n],
            accum: vec![0.; n],
        }
    }
}
impl Graph {
    pub fn new(weights: Vec<f64>, edges: Vec<(usize, usize, f64)>) -> Result<Self, Error> {
        let n = weights.len();
        if n == 0 || weights.iter().any(|x| !x.is_finite() || *x <= 0.) {
            return Err(Error::InvalidGraph);
        }
        let mut exit = vec![0.; n];
        for &(i, j, c) in &edges {
            if i >= n || j >= n || i == j || !c.is_finite() || c < 0. {
                return Err(Error::InvalidGraph);
            }
            exit[i] += c / weights[i];
            exit[j] += c / weights[j];
        }
        if exit.iter().any(|x| !x.is_finite()) {
            return Err(Error::InvalidGraph);
        }
        let q = exit.iter().copied().fold(0., f64::max);
        if !(2. * q).is_finite() {
            return Err(Error::InvalidGraph);
        }
        Ok(Self {
            weights,
            edges,
            exit,
            q,
        })
    }
    pub fn len(&self) -> usize {
        self.weights.len()
    }
    pub fn q(&self) -> f64 {
        self.q
    }
    pub fn edges(&self) -> usize {
        self.edges.len()
    }
    pub fn mass(&self, y: &[f64]) -> f64 {
        self.weights.iter().zip(y).map(|(w, x)| w * x).sum()
    }
    pub fn norm2(&self, y: &[f64]) -> f64 {
        self.weights
            .iter()
            .zip(y)
            .map(|(w, x)| w * x * x)
            .sum::<f64>()
            .sqrt()
    }
    fn validate(&self, y: &[f64], out: &[f64], h: f64, ws: &Workspace) -> Result<(), Error> {
        if y.len() != self.len() || out.len() != self.len() || ws.current.len() != self.len() {
            return Err(Error::Dimension);
        }
        if y.iter().any(|x| !x.is_finite()) {
            return Err(Error::InvalidInput);
        }
        if !h.is_finite() || h < 0. || !(h * self.q).is_finite() {
            return Err(Error::InvalidTime);
        }
        Ok(())
    }
    // Internal functions use validated dimensions. Edge traversal is the only Q action.
    fn apply(&self, y: &[f64], out: &mut [f64]) {
        out.fill(0.);
        for &(i, j, c) in &self.edges {
            let f = c * (y[j] - y[i]);
            out[i] += f / self.weights[i];
            out[j] -= f / self.weights[j];
        }
    }
    fn markov(&self, y: &[f64], out: &mut [f64]) {
        if self.q == 0. {
            out.copy_from_slice(y);
            return;
        }
        for i in 0..self.len() {
            out[i] = (1. - self.exit[i] / self.q) * y[i];
        }
        for &(i, j, c) in &self.edges {
            out[i] += (c / self.weights[i] / self.q) * y[j];
            out[j] += (c / self.weights[j] / self.q) * y[i];
        }
    }
    pub fn euler(
        &self,
        y: &[f64],
        out: &mut [f64],
        t: f64,
        steps: usize,
        ws: &mut Workspace,
    ) -> Result<Work, Error> {
        self.validate(y, out, t, ws)?;
        if steps == 0 {
            return Err(Error::InvalidInput);
        }
        // Preserve finite exact identities without entering a recurrence whose
        // intermediate coefficients can overflow for very large finite states.
        if self.q == 0. || t == 0. {
            out.copy_from_slice(y);
            return Ok(Work {
                actions: 0,
                macrosteps: 0,
                relative_truncation_bound: 0.,
            });
        }
        let theta = t / steps as f64 * self.q;
        if theta > 1. {
            return Err(Error::Stability);
        }
        ws.current.copy_from_slice(y);
        for _ in 0..steps {
            self.markov(&ws.current, &mut ws.temp);
            for i in 0..self.len() {
                ws.current[i] = (1. - theta) * ws.current[i] + theta * ws.temp[i];
            }
        }
        if ws.current.iter().any(|x| !x.is_finite()) {
            return Err(Error::NonFinite);
        }
        out.copy_from_slice(&ws.current);
        Ok(Work {
            actions: steps,
            macrosteps: steps,
            relative_truncation_bound: f64::NAN,
        })
    }
    pub fn rkl2(
        &self,
        y: &[f64],
        out: &mut [f64],
        t: f64,
        steps: usize,
        ws: &mut Workspace,
    ) -> Result<Work, Error> {
        self.validate(y, out, t, ws)?;
        if steps == 0 {
            return Err(Error::InvalidInput);
        }
        // Preserve finite exact identities without entering a recurrence whose
        // intermediate coefficients can overflow for very large finite states.
        if self.q == 0. || t == 0. {
            out.copy_from_slice(y);
            return Ok(Work {
                actions: 0,
                macrosteps: 0,
                relative_truncation_bound: 0.,
            });
        }
        let h = t / steps as f64;
        let z = h * (2. * self.q);
        if !z.is_finite() || z > 1e12 {
            return Err(Error::Stability);
        }
        let s = (((-1. + (9. + 8. * z).sqrt()) / 2.).ceil() as usize).max(2);
        let k = (s as f64) * (s as f64 + 1.) - 2.;
        if 2. * z > k * (1. + 4. * f64::EPSILON) {
            return Err(Error::Stability);
        }
        let actions = steps.checked_mul(s).ok_or(Error::InvalidInput)?;
        let b = k / (2. * s as f64 * (s as f64 + 1.));
        let wh = 4. * h / k;
        ws.current.copy_from_slice(y);
        for _ in 0..steps {
            ws.v0.copy_from_slice(&ws.current);
            self.apply(&ws.current, &mut ws.temp);
            for i in 0..self.len() {
                ws.v1[i] = ws.current[i] + wh * ws.temp[i];
            }
            for j in 2..=s {
                self.apply(&ws.v1, &mut ws.temp);
                let a = (2 * j - 1) as f64 / j as f64;
                let c = (j - 1) as f64 / j as f64;
                for i in 0..self.len() {
                    ws.v2[i] = a * (ws.v1[i] + wh * ws.temp[i]) - c * ws.v0[i];
                }
                std::mem::swap(&mut ws.v0, &mut ws.v1);
                std::mem::swap(&mut ws.v1, &mut ws.v2);
            }
            for i in 0..self.len() {
                ws.current[i] = (1. - b) * ws.current[i] + b * ws.v1[i];
            }
        }
        if ws.current.iter().any(|x| !x.is_finite()) {
            return Err(Error::NonFinite);
        }
        out.copy_from_slice(&ws.current);
        Ok(Work {
            actions,
            macrosteps: steps,
            relative_truncation_bound: f64::NAN,
        })
    }
    pub fn uniformization(
        &self,
        y: &[f64],
        out: &mut [f64],
        t: f64,
        tol: f64,
        ws: &mut Workspace,
    ) -> Result<Work, Error> {
        self.validate(y, out, t, ws)?;
        if !tol.is_finite() || tol <= 0. || tol >= 1. {
            return Err(Error::InvalidTolerance);
        }
        if self.q == 0. || t == 0. {
            out.copy_from_slice(y);
            return Ok(Work {
                actions: 0,
                macrosteps: 0,
                relative_truncation_bound: 0.,
            });
        }
        let product = t * self.q;
        if product > 1e8 {
            return Err(Error::InvalidTime);
        }
        let splits = ((product / 32.).ceil() as usize).max(1);
        let mu = product / splits as f64;
        let target = tol / (2. * splits as f64);
        if target == 0. {
            return Err(Error::InvalidTolerance);
        }
        ws.current.copy_from_slice(y);
        let mut actions = 0;
        let mut bound = 0.;
        for _ in 0..splits {
            ws.v1.copy_from_slice(&ws.current);
            let mut p = (-mu).exp();
            let mut total = p;
            let mut k = 0usize;
            for i in 0..self.len() {
                ws.accum[i] = p * ws.v1[i];
            }
            let tail = loop {
                let next = p * mu / (k + 1) as f64;
                let ratio = mu / (k + 2) as f64;
                let tail = if ratio < 1. {
                    next / (1. - ratio)
                } else {
                    f64::INFINITY
                };
                if tail <= target {
                    break tail;
                }
                if k >= 100000 {
                    return Err(Error::InvalidTolerance);
                }
                self.markov(&ws.v1, &mut ws.temp);
                std::mem::swap(&mut ws.v1, &mut ws.temp);
                p = next;
                total += p;
                k += 1;
                actions += 1;
                for i in 0..self.len() {
                    ws.accum[i] += p * ws.v1[i];
                }
            };
            for i in 0..self.len() {
                ws.current[i] = ws.accum[i] / total;
            }
            bound += 2. * tail;
        }
        if ws.current.iter().any(|x| !x.is_finite()) {
            return Err(Error::NonFinite);
        }
        out.copy_from_slice(&ws.current);
        Ok(Work {
            actions,
            macrosteps: splits,
            relative_truncation_bound: bound,
        })
    }
}

pub fn periodic(n: usize, d: usize) -> (Graph, Vec<f64>, f64) {
    assert!(n >= 3 && (d == 2 || d == 3));
    let size = n.pow(d as u32);
    let w = 1. / size as f64;
    let mut edges = Vec::new();
    let mut mode = vec![1.; size];
    for i in 0..size {
        let mut stride = 1;
        for _ in 0..d {
            let pos = i / stride % n;
            let j = if pos + 1 == n {
                i - (n - 1) * stride
            } else {
                i + stride
            };
            edges.push((i, j, 0.1 * w * (n * n) as f64));
            mode[i] *= (2. * std::f64::consts::PI * (pos as f64 + 0.5) / n as f64).cos();
            stride *= n;
        }
    }
    let rate = -0.4 * (n * n * d) as f64 * (std::f64::consts::PI / n as f64).sin().powi(2);
    (Graph::new(vec![w; size], edges).unwrap(), mode, rate)
}
pub fn sphere(nt: usize, np: usize) -> (Graph, Vec<f64>, f64) {
    assert!(nt >= 2 && np >= 3);
    let pi = std::f64::consts::PI;
    let dp = 2. * pi / np as f64;
    let faces: Vec<_> = (0..=nt)
        .map(|i| -(pi * i as f64 / nt as f64).cos())
        .collect();
    let centers: Vec<_> = (0..nt).map(|i| (faces[i] + faces[i + 1]) / 2.).collect();
    let mut weights = Vec::new();
    let mut edges = Vec::new();
    let mut mode = Vec::new();
    for i in 0..nt {
        let width = faces[i + 1] - faces[i];
        for j in 0..np {
            let pos = i * np + j;
            weights.push(width * dp);
            mode.push(centers[i]);
            if i + 1 < nt {
                edges.push((
                    pos,
                    (i + 1) * np + j,
                    (1. - faces[i + 1].powi(2)) * dp / (centers[i + 1] - centers[i]),
                ));
            }
            edges.push((
                pos,
                i * np + (j + 1) % np,
                width / ((1. - centers[i].powi(2)) * dp),
            ));
        }
    }
    (Graph::new(weights, edges).unwrap(), mode, -2.)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn geometric_mass_and_axis() {
        let (g, m, _) = sphere(10, 20);
        assert!((g.mass(&vec![1.; g.len()]) - 4. * std::f64::consts::PI).abs() < 1e-12);
        let mut out = vec![0.; g.len()];
        g.apply(&m, &mut out);
        assert!(out.iter().zip(m).all(|(a, b)| (a + 2. * b).abs() < 1e-12));
    }
    #[test]
    fn exact_positivity_counterexample() {
        let (g, _, _) = periodic(8, 2);
        let mut y = vec![0.; 64];
        y[0] = 1.;
        let mut out = vec![0.; 64];
        let mut ws = Workspace::new(64);
        let work = g.rkl2(&y, &mut out, 2.5 / g.q(), 1, &mut ws).unwrap();
        assert_eq!(work.actions, 3);
        assert!((out.iter().copied().fold(f64::INFINITY, f64::min) + 5. / 512.).abs() < 1e-14);
        assert!(g.norm2(&out) <= g.norm2(&y));
    }
    #[test]
    fn normalized_poisson_positive() {
        let (g, _, _) = periodic(8, 2);
        let mut y = vec![0.; 64];
        y[0] = 1.;
        let mut out = vec![0.; 64];
        let mut ws = Workspace::new(64);
        let work = g
            .uniformization(&y, &mut out, 2.5 / g.q(), 1e-12, &mut ws)
            .unwrap();
        assert!(out.iter().all(|x| *x >= 0.));
        assert!((g.mass(&out) - g.mass(&y)).abs() < 1e-14);
        assert!(work.relative_truncation_bound <= 1e-12);
    }
    #[test]
    fn temporal_error_and_work() {
        let (g, m, rate) = periodic(16, 2);
        let y: Vec<_> = m.iter().map(|x| 1. + 0.2 * x).collect();
        let exact: Vec<_> = m
            .iter()
            .map(|x| 1. + 0.2 * (rate * 0.1).exp() * x)
            .collect();
        let mut out = vec![0.; g.len()];
        let mut ws = Workspace::new(g.len());
        let w = g.rkl2(&y, &mut out, 0.1, 32, &mut ws).unwrap();
        let err: Vec<_> = out.iter().zip(&exact).map(|(a, b)| a - b).collect();
        assert!(g.norm2(&err) / g.norm2(&exact) < 1e-5);
        assert_eq!(w.actions, 64);
    }
    #[test]
    fn signed_uniformization_reference() {
        let g = Graph::new(vec![1., 1.], vec![(0, 1, 3.)]).unwrap();
        let y = [1., -1.];
        let mut out = [0., 0.];
        let mut ws = Workspace::new(2);
        let work = g.uniformization(&y, &mut out, 0.7, 1e-9, &mut ws).unwrap();
        let e = (-4.2_f64).exp();
        assert!((out[0] - e).abs() <= work.relative_truncation_bound + 1e-14);
        assert!((out[1] + e).abs() < 1e-9);
    }
    #[test]
    fn fail_atomic_and_dimension() {
        let (g, _, _) = periodic(4, 2);
        let y = vec![1.; 16];
        let mut out = vec![42.; 16];
        let mut ws = Workspace::new(16);
        assert_eq!(
            g.euler(&y, &mut out, 100., 1, &mut ws).unwrap_err(),
            Error::Stability
        );
        assert!(out.iter().all(|x| *x == 42.));
        assert_eq!(
            g.rkl2(&y[..15], &mut out, 0.1, 1, &mut ws).unwrap_err(),
            Error::Dimension
        );
        assert_eq!(
            g.uniformization(&y, &mut out, 0.1, 0., &mut ws)
                .unwrap_err(),
            Error::InvalidTolerance
        );
    }
    #[test]
    fn invalid_and_zero_graph() {
        assert!(Graph::new(vec![0.], vec![]).is_err());
        assert!(Graph::new(vec![1.], vec![(0, 1, 1.)]).is_err());
        let g = Graph::new(vec![1., 2.], vec![]).unwrap();
        let mut ws = Workspace::new(2);
        let mut out = [0., 0.];
        assert_eq!(
            g.uniformization(&[2., 3.], &mut out, 1., 1e-10, &mut ws)
                .unwrap()
                .actions,
            0
        );
        assert_eq!(out, [2., 3.]);
    }
    #[test]
    fn zero_graph_identity_avoids_intermediate_overflow() {
        let g = Graph::new(vec![1., 2.], vec![]).unwrap();
        let y = [1.5e308, -1.5e308];
        let mut out = [42., 42.];
        let mut ws = Workspace::new(2);
        assert_eq!(g.rkl2(&y, &mut out, 1., 1, &mut ws).unwrap().actions, 0);
        assert_eq!(out, y);
        assert_eq!(g.euler(&y, &mut out, 1., 1, &mut ws).unwrap().actions, 0);
        assert_eq!(out, y);
        assert_eq!(
            g.uniformization(&y, &mut out, 1., 1e-10, &mut ws)
                .unwrap()
                .actions,
            0
        );
        assert_eq!(out, y);
        out = [42., 42.];
        assert_eq!(
            g.rkl2(&y, &mut out, 1., 0, &mut ws).unwrap_err(),
            Error::InvalidInput
        );
        assert_eq!(out, [42., 42.]);
    }
    #[test]
    fn zero_time_identity_avoids_intermediate_overflow() {
        let g = Graph::new(vec![1., 2.], vec![(0, 1, 3.)]).unwrap();
        let y = [1.5e308, -1.5e308];
        let mut out = [42., 42.];
        let mut ws = Workspace::new(2);
        assert_eq!(g.rkl2(&y, &mut out, 0., 1, &mut ws).unwrap().actions, 0);
        assert_eq!(out, y);
        assert_eq!(g.euler(&y, &mut out, 0., 1, &mut ws).unwrap().actions, 0);
        assert_eq!(out, y);
        assert_eq!(
            g.uniformization(&y, &mut out, 0., 1e-10, &mut ws)
                .unwrap()
                .actions,
            0
        );
        assert_eq!(out, y);
    }
}
