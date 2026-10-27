//! Explicit, dimensionless method-of-lines benchmark definitions.
//! A forward dual evaluation supplies Jv and f_t; it is never invoked by DOPRI5.
// Shared by several examples and model-only tests, each using a different
// subset of the fixture/report API.
#![allow(dead_code)]
use rayon::prelude::*;
use serde_json::{Value, json};
use std::f64::consts::PI;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug)]
struct Dual {
    v: f64,
    d: f64,
}
impl From<f64> for Dual {
    fn from(v: f64) -> Self {
        Self { v, d: 0.0 }
    }
}
impl Add for Dual {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self {
            v: self.v + b.v,
            d: self.d + b.d,
        }
    }
}
impl Sub for Dual {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self {
            v: self.v - b.v,
            d: self.d - b.d,
        }
    }
}
impl Mul for Dual {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self {
            v: self.v * b.v,
            d: self.d * b.v + self.v * b.d,
        }
    }
}
impl Div for Dual {
    type Output = Self;
    fn div(self, b: Self) -> Self {
        Self {
            v: self.v / b.v,
            d: (self.d - self.v / b.v * b.d) / b.v,
        }
    }
}
impl Neg for Dual {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            v: -self.v,
            d: -self.d,
        }
    }
}
macro_rules! scalar_ops {($trait:ident,$method:ident,$op:tt)=>{impl $trait<f64> for Dual {type Output=Self;fn $method(self,b:f64)->Self{self $op Dual::from(b)}} impl $trait<Dual> for f64 {type Output=Dual;fn $method(self,b:Dual)->Dual{Dual::from(self) $op b}}};}
scalar_ops!(Add,add,+);
scalar_ops!(Sub,sub,-);
scalar_ops!(Mul,mul,*);
scalar_ops!(Div,div,/);
impl Dual {
    fn sin(self) -> Self {
        Self {
            v: self.v.sin(),
            d: self.d * self.v.cos(),
        }
    }
    fn cos(self) -> Self {
        Self {
            v: self.v.cos(),
            d: -self.d * self.v.sin(),
        }
    }
    fn exp(self) -> Self {
        let v = self.v.exp();
        Self { v, d: self.d * v }
    }
    fn pow(self, p: f64) -> Self {
        Self {
            v: self.v.powf(p),
            d: if self.d == 0.0 {
                0.0
            } else {
                p * self.v.powf(p - 1.0) * self.d
            },
        }
    }
    fn sq(self) -> Self {
        self * self
    }
}

pub const METHODS: [&str; 4] = ["dopri5", "rodas5p-mf", "rodas5p-dense", "switching"];
pub const LAYERS: [&str; 3] = ["serial", "data", "ensemble"];
#[derive(Clone, Debug)]
pub struct Model {
    pub id: usize,
    pub n: usize,
    pub nx: usize,
    pub ny: usize,
    pub end: f64,
    pub smoke: bool,
}
impl Model {
    pub fn new(id: usize, smoke: bool) -> Result<Self, String> {
        let (n, nx, ny, end) = match id {
            1 => (3, 0, 0, if smoke { 0.01 } else { 40.0 }),
            2 => (8, 0, 0, if smoke { 0.01 } else { 321.8122 }),
            3 => (28, 0, 0, if smoke { 0.01 } else { 3.0 }),
            4 => {
                let n = if smoke { 16 } else { 4096 };
                (n, n, 0, if smoke { 0.02 } else { 10.0 })
            }
            5 => (2, 0, 0, if smoke { 0.0001 } else { 2.0 }),
            6 => {
                let n = if smoke { 16 } else { 1024 };
                (n, n, 0, if smoke { 0.002 } else { 2.0 })
            }
            7 => (1, 0, 0, if smoke { 0.01 } else { 10.0 }),
            8 => (3, 0, 0, if smoke { 0.01 } else { 30.0 }),
            9 => {
                let m = if smoke { 8 } else { 256 };
                (2 * m, m, 0, if smoke { 0.002 } else { 1.0 })
            }
            10 => (6, 0, 0, if smoke { 0.01 } else { 1.0 }),
            11 => {
                let m = if smoke { 5 } else { 36 };
                (11 * m, m, 10, if smoke { 0.01 } else { 5.0 })
            }
            12 => {
                let n = if smoke { 16 } else { 1024 };
                (n, n, 0, if smoke { 0.01 } else { 10.0 })
            }
            13 => {
                let n = if smoke { 8 } else { 1024 };
                (3 * n, n, 0, if smoke { 0.00001 } else { 0.1 })
            }
            14 => {
                let n = if smoke { 16 } else { 4096 };
                (n, n, 0, if smoke { 0.0001 } else { 1.0 })
            }
            15 => (2, 0, 0, if smoke { 0.1 } else { 8.0 }),
            16 => (2, 0, 0, if smoke { 0.01 } else { 2.49 }),
            17 => {
                let (a, b) = if smoke { (4, 8) } else { (32, 64) };
                (a * b, a, b, if smoke { 0.01 } else { 1.0 })
            }
            18 => {
                let (a, b) = if smoke { (4, 8) } else { (32, 128) };
                (a * b, a, b, if smoke { 0.001 } else { 0.1 })
            }
            19 => {
                let l = if smoke { 4 } else { 32 };
                (3 * l * l, l, 0, if smoke { 0.01 } else { 1.0 })
            }
            20 => {
                let l = if smoke { 4 } else { 32 };
                (l * l, l, 0, if smoke { 0.001 } else { 2.0 })
            }
            21 => {
                let n = if smoke { 4 } else { 64 };
                (2 * n * n, n, n, if smoke { 0.01 } else { 1.0 })
            }
            22 => {
                let n = if smoke { 4 } else { 64 };
                (n * n, n, n, if smoke { 0.001 } else { 0.1 })
            }
            23 => {
                let n = if smoke { 4 } else { 24 };
                (2 * n * n * n, n, n, if smoke { 0.001 } else { 5.0 })
            }
            24 => {
                let n = if smoke { 4 } else { 32 };
                (n * n * n, n, n, if smoke { 0.00001 } else { 0.1 })
            }
            _ => return Err("case must be 1..24".into()),
        };
        Ok(Self {
            id,
            n,
            nx,
            ny,
            end,
            smoke,
        })
    }
    pub fn name(&self) -> &'static str {
        NAMES[self.id - 1]
    }
    pub fn variant(&self) -> &'static str {
        VARIANTS[self.id - 1]
    }
    pub fn autonomous(&self) -> bool {
        !matches!(self.id, 5 | 6 | 15 | 20)
    }
    pub fn exact_kind(&self) -> &'static str {
        match self.id {
            6 | 10 | 15 | 16 | 19 | 22 => "analytic-semidiscrete-endpoint",
            7 if self.end <= 10.0 => "analytic-negligible-evolution-bound",
            _ => "external-reference-required",
        }
    }
    pub fn metadata(&self) -> Value {
        json!({"id":self.id,"case":self.id,"name":self.name(),"variant":self.variant(),"dimension":self.n,"dimension_smoke":Self::new(self.id,true).unwrap().n,"dimension_standard":Self::new(self.id,false).unwrap().n,"size":if self.smoke{"smoke"}else{"standard"},"grid":[self.nx,self.ny],"t_span":[0.0,self.end],"autonomous":self.autonomous(),"units":"dimensionless; normalized variables, not SI physical claims","reference_kind":self.exact_kind(),"supported_methods":METHODS,"parallel_layers":LAYERS,"dense_dimension_limit":256,"jvp_provider":"forward-dual directional evaluation; explicit method never invokes it","initial_state":self.initial(),"parameters":self.parameters(),"observation_semantics":"whole-span solve, endpoint error only; switching retains its own intermediate window records"})
    }
    pub fn parameters(&self) -> Value {
        match self.id {
            1 => json!({"k1":0.04,"k2":1e4,"k3":3e7}),
            2 => json!({"source":"HIRES test-set equations; initial y8=.0057; no POLLU claim"}),
            3 => json!({"masses":[1,2,3,4,5,6,7],"G":1.0,"softening":0.0}),
            4 => {
                json!({"radius":if self.smoke{4}else{32},"coupling":1.0,"alpha":0.2,"frequency":"0.5+0.1cos(2pi i/N)"})
            }
            5 => {
                json!({"epsilon":1e-6,"classification":"persistent transverse stiffness negative switch-down control"})
            }
            6 => {
                json!({"velocity":0.1,"gamma0":1e5,"tau":0.05,"gamma_exponent":4,"boundary":"periodic"})
            }
            7 => {
                json!({"epsilon":0.02,"classification":"literal attachment equation: no ignition; f'(1)=-1","reference_time_max":10.0,"reference_absolute_bound":1e-190})
            }
            8 => json!({"s":77.27,"q":8.375e-6,"w":0.161}),
            9 => json!({"spring":1e6,"quartic":1.0,"boundary":"periodic"}),
            10 => {
                json!({"omega_c":500.0,"B":[0,0,1],"E":[0,0,0],"c":1,"initial_momentum":[1,0,0.2]})
            }
            11 => {
                json!({"F":10,"c":10,"b":10,"h":1,"fast_ring":"global periodic ring; stated forward-advection variant"})
            }
            12 => {
                json!({"length":32.0*PI,"nu":1.0,"boundary":"periodic","discretization":"centered D1,D2,D2 squared; RHS=-u D1u-D2u-D2^2u"})
            }
            13 => {
                json!({"Mach":1e-3,"gamma":1.4,"cooling":1e3,"boundary":"periodic","variables":["rho","velocity","pressure"],"variant":"primitive isentropic acoustic-advection plus pressure relaxation"})
            }
            14 => json!({"epsilon":0.02,"velocity":0.1,"length":2.0*PI,"boundary":"periodic"}),
            15 => json!({"omega0":1.0,"beta":1.2,"gamma":0.8}),
            16 => {
                json!({"alpha":0.1,"omega0":1.0,"r0":1.0,"collision_time":2.5,"coordinates":"two Cartesian first-order variables"})
            }
            17 => {
                json!({"rotation_axis":[std::f64::consts::FRAC_1_SQRT_2,0.0,std::f64::consts::FRAC_1_SQRT_2],"shear":0.1,"discretization":"first-order conservative upwind equal-mu FVM; zero polar face flux"})
            }
            18 => {
                json!({"d_mu":0.01,"d_phi":"0.1(1+mu^2/2)","opacity":"1+1000 I(mu>.5 and cos(phi)>.5)","discretization":"conservative equal-mu fluxes, periodic phi"})
            }
            19 => {
                json!({"gravity_depth":1.0,"nu":1e-4,"variant":"linearized nonrotating spherical modal acoustic pairs, hyperviscosity on all components; not nonlinear SWE"})
            }
            20 => {
                json!({"k":1.0,"gamma0":1e4,"gamma_exponent":4,"baryon_dipole":0.0,"normalization":"orthonormal fixed-m antisymmetric chains; top moment zero; external fixed matter bath"})
            }
            21 => {
                json!({"A":1.0,"B":3.4,"diffusion":0.01,"advection_x":0.1,"boundary":"periodic","variant":"periodic Brusselator; not canonical BRUSS2D forcing/BC"})
            }
            22 => {
                json!({"kappa_parallel":1.0,"kappa_perp":1e-6,"b":[std::f64::consts::FRAC_1_SQRT_2,std::f64::consts::FRAC_1_SQRT_2],"boundary":"periodic","discretization":"constant tensor 9-point centered stencil"})
            }
            23 => {
                json!({"a":0.75,"b":0.02,"epsilon":0.02,"Du":0.001,"Dv":0.0,"boundary":"periodic","variant":"Barkley kinetics, matching supplied polynomial; not Gray-Scott/Schnakenberg"})
            }
            24 => {
                json!({"D":0.01,"velocity":[0.1,0.05,0.02],"opacity":"1e6 inside central radius .2 ball, 1 outside","source":"exp(-100*((x-.2)^2+(y-.5)^2+(z-.5)^2))","boundary":"periodic"})
            }
            _ => json!({}),
        }
    }
    pub fn initial(&self) -> Vec<f64> {
        let mut y = vec![0.0; self.n];
        match self.id {
            1 => y[0] = 1.0,
            2 => {
                y[0] = 1.0;
                y[7] = 0.0057
            }
            3 => y.copy_from_slice(&[
                3., 3., -1., -3., 2., -2., 2., 3., -3., 2., 0., 0., -4., 4., 0., 0., 0., 0., 0.,
                1.75, -1.5, 0., 0., 0., -1.25, 1., 0., 0.,
            ]),
            4 => {
                for (i, v) in y.iter_mut().enumerate() {
                    *v = 2.0 * PI * i as f64 / self.n as f64 + 0.1 * (3.0 * i as f64).sin()
                }
            }
            5 => y.copy_from_slice(&[0., 1.]),
            6 => {
                for (i, v) in y.iter_mut().enumerate() {
                    *v = 1.0 + 0.1 * (2.0 * PI * i as f64 / self.n as f64).cos()
                }
            }
            7 => y[0] = 0.1,
            8 => y.copy_from_slice(&[1., 2., 3.]),
            9 => {
                for (i, v) in y.iter_mut().take(self.nx).enumerate() {
                    *v = 0.01 * (2.0 * PI * i as f64 / self.nx as f64).sin()
                        + 0.001 * (0.5 * PI * i as f64).sin()
                }
            }
            10 => y.copy_from_slice(&[0., 0., 0., 1., 0., 0.2]),
            11 => {
                for (i, v) in y.iter_mut().enumerate() {
                    *v = if i < self.nx {
                        5.0 + 0.01 * (i as f64).sin()
                    } else {
                        0.01 * (i as f64).cos()
                    }
                }
            }
            12 => {
                for (i, v) in y.iter_mut().enumerate() {
                    let x = 32.0 * PI * i as f64 / self.n as f64;
                    *v = (x / 16.0).cos() * (1.0 + (x / 16.0).sin())
                }
            }
            13 => {
                for i in 0..self.nx {
                    let v = 0.001 * (2.0 * PI * i as f64 / self.nx as f64).sin();
                    y[i] = 1.0 + v;
                    y[self.nx + i] = 0.1;
                    y[2 * self.nx + i] = 1.0 + v
                }
            }
            14 => {
                for (i, v) in y.iter_mut().enumerate() {
                    *v = 0.1 * (2.0 * PI * i as f64 / self.n as f64).cos()
                }
            }
            15 => y.copy_from_slice(&[1., -0.8]),
            16 => y[0] = 1.0,
            17 | 18 => {
                for (i, v) in y.iter_mut().enumerate() {
                    let mu =
                        -1.0 + (i / self.ny) as f64 * 2.0 / self.nx as f64 + 1.0 / self.nx as f64;
                    let phi = 2.0 * PI * (i % self.ny) as f64 / self.ny as f64;
                    *v = 1.0 + 0.1 * (1.0 - mu * mu).sqrt() * phi.cos()
                }
            }
            19 => {
                for q in 0..self.n / 3 {
                    let l = (q as f64).sqrt().floor();
                    y[3 * q] = 0.01 / (1.0 + l);
                    y[3 * q + 2] = 0.002 / (1.0 + l)
                }
            }
            20 => {
                for (i, v) in y.iter_mut().enumerate() {
                    *v = 0.01 / (1.0 + i as f64)
                }
            }
            21 => {
                for q in 0..self.n / 2 {
                    let x = 2.0 * PI * (q % self.nx) as f64 / self.nx as f64;
                    let z = 2.0 * PI * (q / self.nx) as f64 / self.nx as f64;
                    y[2 * q] = 1.0 + 0.1 * x.sin() * z.sin();
                    y[2 * q + 1] = 3.4 + 0.1 * x.cos() * z.cos()
                }
            }
            22 => {
                for (q, v) in y.iter_mut().enumerate() {
                    *v = 1.0
                        + 0.1
                            * (2.0 * PI * (q % self.nx) as f64 / self.nx as f64
                                + 4.0 * PI * (q / self.nx) as f64 / self.nx as f64)
                                .cos()
                }
            }
            23 => {
                for q in 0..self.n / 2 {
                    y[2 * q] = if q % self.nx < self.nx / 2 { 1.0 } else { 0.0 };
                    y[2 * q + 1] = if q / (self.nx * self.nx) < self.nx / 2 {
                        0.1
                    } else {
                        0.0
                    }
                }
            }
            24 => {}
            _ => unreachable!(),
        };
        y
    }
    pub fn apply(
        &self,
        t: f64,
        y: &[f64],
        direction: Option<&[f64]>,
        time_direction: bool,
        out: &mut [f64],
        parallel: bool,
    ) {
        let td = Dual {
            v: t,
            d: if time_direction { 1.0 } else { 0.0 },
        };
        let mean = if self.id == 6 {
            Some(Dual {
                v: y.iter().sum::<f64>() / self.n as f64,
                d: direction.map_or(0.0, |v| v.iter().sum::<f64>() / self.n as f64),
            })
        } else {
            None
        };
        let evaluate = |i| {
            let z = self.component(td, y, direction, mean, i);
            if direction.is_some() || time_direction {
                z.d
            } else {
                z.v
            }
        };
        if parallel {
            out.par_iter_mut()
                .enumerate()
                .for_each(|(i, z)| *z = evaluate(i))
        } else {
            for (i, z) in out.iter_mut().enumerate() {
                *z = evaluate(i)
            }
        }
    }
    fn component(
        &self,
        t: Dual,
        y: &[f64],
        direction: Option<&[f64]>,
        mean: Option<Dual>,
        i: usize,
    ) -> Dual {
        let x = |j: usize| Dual {
            v: y[j],
            d: direction.map_or(0.0, |v| v[j]),
        };
        let c = Dual::from;
        let wrap = |j: isize, n: usize| j.rem_euclid(n as isize) as usize;
        match self.id {
            1 => {
                let a = 0.04 * x(0);
                let b = 1e4 * x(1) * x(2);
                let z = 3e7 * x(1).sq();
                match i {
                    0 => -a + b,
                    1 => a - b - z,
                    _ => z,
                }
            }
            2 => match i {
                0 => -1.71 * x(0) + 0.43 * x(1) + 8.32 * x(2) + 0.0007,
                1 => 1.71 * x(0) - 8.75 * x(1),
                2 => -10.03 * x(2) + 0.43 * x(3) + 0.035 * x(4),
                3 => 8.32 * x(1) + 1.71 * x(2) - 1.12 * x(3),
                4 => -1.745 * x(4) + 0.43 * x(5) + 0.43 * x(6),
                5 => -280.0 * x(5) * x(7) + 0.69 * x(3) + 1.71 * x(4) - 0.43 * x(5) + 0.69 * x(6),
                6 => 280.0 * x(5) * x(7) - 1.81 * x(6),
                _ => -280.0 * x(5) * x(7) + 1.81 * x(6),
            },
            3 => {
                if i < 14 {
                    return x(i + 14);
                }
                let body = (i - 14) % 7;
                let axis = (i - 14) / 7;
                let mut a = c(0.0);
                for j in 0..7 {
                    if j != body {
                        let dx = x(j) - x(body);
                        let dy = x(7 + j) - x(7 + body);
                        let r3 = (dx.sq() + dy.sq()).pow(1.5);
                        a = a + (j + 1) as f64 * (if axis == 0 { dx } else { dy }) / r3
                    }
                }
                a
            }
            4 => {
                let radius = if self.smoke { 4 } else { 32 };
                let mut sum = c(0.0);
                for delta in -(radius as isize)..=radius as isize {
                    if delta != 0 {
                        sum = sum + (x(wrap(i as isize + delta, self.n)) - x(i) - 0.2).sin()
                    }
                }
                c(0.5 + 0.1 * (2.0 * PI * i as f64 / self.n as f64).cos())
                    + sum / (2 * radius) as f64
            }
            5 => {
                let f = -x(1) / (1.0 + t) + 0.2 * x(0);
                if i == 1 {
                    f
                } else {
                    -(x(0) - x(1).cos()) / 1e-6 - x(1).sin() * f
                }
            }
            6 => {
                let mean = mean.expect("freeze-out mean");
                let gamma = 1e5 / (1.0 + t / 0.05).pow(4.0);
                -0.1 * self.n as f64 * (x(i) - x(wrap(i as isize - 1, self.n)))
                    + gamma * (mean - x(i))
            }
            7 => (1.0 - x(0)) * ((x(0) - 1.0) / (0.02 * x(0))).exp(),
            8 => match i {
                0 => 77.27 * (x(1) - x(0) * x(1) + x(0) - 8.375e-6 * x(0).sq()),
                1 => (-x(1) - x(0) * x(1) + x(2)) / 77.27,
                _ => 0.161 * (x(0) - x(2)),
            },
            9 => {
                let m = self.nx;
                if i < m {
                    return x(m + i);
                }
                let q = i - m;
                let a = x(wrap(q as isize + 1, m)) - x(q);
                let b = x(q) - x(wrap(q as isize - 1, m));
                1e6 * (a - b) + a * a * a - b * b * b
            }
            10 => {
                let gamma = (1.0 + x(3).sq() + x(4).sq() + x(5).sq()).pow(0.5);
                match i {
                    0..=2 => x(i + 3) / gamma,
                    3 => 500.0 * x(4) / gamma,
                    4 => -500.0 * x(3) / gamma,
                    _ => c(0.0),
                }
            }
            11 => {
                let m = self.nx;
                let nf = 10 * m;
                if i < m {
                    let mut s = c(0.0);
                    for j in 0..10 {
                        s = s + x(m + 10 * i + j)
                    }
                    (x((i + 1) % m) - x(wrap(i as isize - 2, m))) * x(wrap(i as isize - 1, m))
                        - x(i)
                        + 10.0
                        - s
                } else {
                    let k = i - m;
                    (x(m + (k + 1) % nf) - x(m + wrap(k as isize - 2, nf)))
                        * x(m + wrap(k as isize - 1, nf))
                        * 100.0
                        - 10.0 * x(i)
                        + x(k / 10)
                }
            }
            12 | 14 => {
                let n = self.n;
                let dx = if self.id == 12 {
                    32.0 * PI / n as f64
                } else {
                    2.0 * PI / n as f64
                };
                let z = |off: isize| x(wrap(i as isize + off, n));
                let d1 = (z(1) - z(-1)) / (2.0 * dx);
                let d2 = (z(1) - 2.0 * z(0) + z(-1)) / (dx * dx);
                let d4 = (z(2) - 4.0 * z(1) + 6.0 * z(0) - 4.0 * z(-1) + z(-2)) / dx.powi(4);
                if self.id == 12 {
                    -z(0) * d1 - d2 - d4
                } else {
                    let cube = |q: Dual| q * q * q;
                    let lapcube = (cube(z(1)) - 2.0 * cube(z(0)) + cube(z(-1))) / (dx * dx);
                    lapcube - d2 - 0.0004 * d4 - 0.1 * d1
                }
            }
            13 => {
                let n = self.nx;
                let k = i % n;
                let deriv = |field: usize| {
                    (x(field * n + (k + 1) % n) - x(field * n + wrap(k as isize - 1, n)))
                        * (n as f64 / 2.0)
                };
                let rho = x(k);
                let u = x(n + k);
                let p = x(2 * n + k);
                match i / n {
                    0 => -u * deriv(0) - rho * deriv(1),
                    1 => -u * deriv(1) - deriv(2) / (1e-6 * rho),
                    _ => -u * deriv(2) - 1.4 * p * deriv(1) - 1e3 * (p - rho),
                }
            }
            15 => {
                if i == 0 {
                    x(1)
                } else {
                    -0.4 * x(1) - ((2.4 * t).exp() - 0.32) * x(0)
                }
            }
            16 => {
                let r2 = x(0).sq() + x(1).sq();
                let radial = -0.1 / r2.sq();
                let w = 1.0 / r2.pow(0.75);
                if i == 0 {
                    radial * x(0) - w * x(1)
                } else {
                    radial * x(1) + w * x(0)
                }
            }
            17 => {
                let a = i / self.ny;
                let b = i % self.ny;
                let dm = 2.0 / self.nx as f64;
                let dp = 2.0 * PI / self.ny as f64;
                let mu = -1.0 + (a as f64 + 0.5) * dm;
                let phi = b as f64 * dp;
                let vm = |m: f64, p: f64| {
                    std::f64::consts::FRAC_1_SQRT_2 * (1.0 - m * m).max(0.0).sqrt() * p.sin()
                        + 0.1 * m * (1.0 - m * m)
                };
                let vp = |m: f64, p: f64| {
                    std::f64::consts::FRAC_1_SQRT_2 * (1.0 - m * p.cos() / (1.0 - m * m).sqrt())
                };
                let flux = |speed: f64, left: Dual, right: Dual| {
                    speed * if speed >= 0.0 { left } else { right }
                };
                let up = if a + 1 < self.nx {
                    flux(vm(mu + dm / 2.0, phi), x(i), x(i + self.ny))
                } else {
                    c(0.0)
                };
                let down = if a > 0 {
                    flux(vm(mu - dm / 2.0, phi), x(i - self.ny), x(i))
                } else {
                    c(0.0)
                };
                let right = flux(
                    vp(mu, phi + dp / 2.0),
                    x(i),
                    x(a * self.ny + (b + 1) % self.ny),
                );
                let left = flux(
                    vp(mu, phi - dp / 2.0),
                    x(a * self.ny + wrap(b as isize - 1, self.ny)),
                    x(i),
                );
                -(up - down) / dm - (right - left) / dp
            }
            18 => {
                let a = i / self.ny;
                let b = i % self.ny;
                let dm = 2.0 / self.nx as f64;
                let dp = 2.0 * PI / self.ny as f64;
                let mu = -1.0 + (a as f64 + 0.5) * dm;
                let phi = b as f64 * dp;
                let up = if a + 1 < self.nx {
                    0.01 * (1.0 - (mu + dm / 2.0).powi(2)) * (x(i + self.ny) - x(i)) / (dm * dm)
                } else {
                    c(0.0)
                };
                let down = if a > 0 {
                    0.01 * (1.0 - (mu - dm / 2.0).powi(2)) * (x(i - self.ny) - x(i)) / (dm * dm)
                } else {
                    c(0.0)
                };
                up + down
                    + 0.1 * (1.0 + mu * mu / 2.0) / (1.0 - mu * mu)
                        * (x(a * self.ny + (b + 1) % self.ny) - 2.0 * x(i)
                            + x(a * self.ny + wrap(b as isize - 1, self.ny)))
                        / (dp * dp)
                    - (if mu > 0.5 && phi.cos() > 0.5 {
                        1001.0
                    } else {
                        1.0
                    }) * x(i)
            }
            19 => {
                let q = i / 3;
                let l = (q as f64).sqrt().floor();
                let w = (l * (l + 1.0)).sqrt();
                let d = 1e-4 * (l * (l + 1.0)).powi(2);
                match i % 3 {
                    0 => w * x(i + 1) - d * x(i),
                    1 => -w * x(i - 1) - d * x(i),
                    _ => -d * x(i),
                }
            }
            20 => {
                let l = (i as f64).sqrt().floor() as usize;
                let m = i as isize - (l * l + l) as isize;
                let low = if l > 0 && m.unsigned_abs() < l {
                    let a =
                        ((l * l) as f64 - (m * m) as f64).sqrt() / ((4 * l * l - 1) as f64).sqrt();
                    a * x(((l * l - l) as isize + m) as usize)
                } else {
                    c(0.0)
                };
                let high = if l + 1 < self.nx {
                    let z = l + 1;
                    let a =
                        ((z * z) as f64 - (m * m) as f64).sqrt() / ((4 * z * z - 1) as f64).sqrt();
                    a * x(((z * z + z) as isize + m) as usize)
                } else {
                    c(0.0)
                };
                let g = 1e4 / (1.0 + t).pow(4.0);
                low - high
                    - (if l == 0 {
                        0.0
                    } else if l == 2 {
                        0.9
                    } else {
                        1.0
                    }) * g
                        * x(i)
            }
            21 | 22 => {
                let stride = if self.id == 21 { 2 } else { 1 };
                let q = i / stride;
                let species = i % stride;
                let n = self.nx;
                let a = q % n;
                let b = q / n;
                let at = |da: isize, db: isize| {
                    x(
                        stride * (wrap(b as isize + db, n) * n + wrap(a as isize + da, n))
                            + species,
                    )
                };
                let h = 1.0 / n as f64;
                let xx = (at(1, 0) - 2.0 * at(0, 0) + at(-1, 0)) / (h * h);
                let yy = (at(0, 1) - 2.0 * at(0, 0) + at(0, -1)) / (h * h);
                if self.id == 22 {
                    let xy = (at(1, 1) - at(1, -1) - at(-1, 1) + at(-1, -1)) / (4.0 * h * h);
                    0.5000005 * (xx + yy) + 0.999999 * xy
                } else {
                    let u = x(2 * q);
                    let v = x(2 * q + 1);
                    let reaction = if species == 0 {
                        1.0 + u * u * v - 4.4 * u
                    } else {
                        3.4 * u - u * u * v
                    };
                    reaction + 0.01 * (xx + yy) + 0.1 * (at(1, 0) - at(-1, 0)) / (2.0 * h)
                }
            }
            23 | 24 => {
                let stride = if self.id == 23 { 2 } else { 1 };
                let q = i / stride;
                let species = i % stride;
                let n = self.nx;
                let a = q % n;
                let b = (q / n) % n;
                let z = q / (n * n);
                let at = |da: isize, db: isize, dz: isize| {
                    x(stride
                        * ((wrap(z as isize + dz, n) * n + wrap(b as isize + db, n)) * n
                            + wrap(a as isize + da, n))
                        + species)
                };
                let h = 1.0 / n as f64;
                let lap = (at(1, 0, 0)
                    + at(-1, 0, 0)
                    + at(0, 1, 0)
                    + at(0, -1, 0)
                    + at(0, 0, 1)
                    + at(0, 0, -1)
                    - 6.0 * at(0, 0, 0))
                    / (h * h);
                if self.id == 23 {
                    let u = x(2 * q);
                    let v = x(2 * q + 1);
                    if species == 0 {
                        0.001 * lap + u * (1.0 - u) * (u - (v + 0.02) / 0.75) / 0.02
                    } else {
                        u - v
                    }
                } else {
                    let xx = (a as f64 + 0.5) * h;
                    let yy = (b as f64 + 0.5) * h;
                    let zz = (z as f64 + 0.5) * h;
                    let sigma =
                        if (xx - 0.5).powi(2) + (yy - 0.5).powi(2) + (zz - 0.5).powi(2) < 0.04 {
                            1e6
                        } else {
                            1.0
                        };
                    0.01 * lap
                        - 0.1 * (at(0, 0, 0) - at(-1, 0, 0)) / h
                        - 0.05 * (at(0, 0, 0) - at(0, -1, 0)) / h
                        - 0.02 * (at(0, 0, 0) - at(0, 0, -1)) / h
                        - sigma * x(i)
                        + c((-100.0
                            * ((xx - 0.2).powi(2) + (yy - 0.5).powi(2) + (zz - 0.5).powi(2)))
                        .exp())
                }
            }
            _ => unreachable!(),
        }
    }
    pub fn exact(&self, t: f64) -> Option<Vec<f64>> {
        if !t.is_finite() || t < 0.0 {
            return None;
        }
        let mut y = self.initial();
        match self.id {
            6 => {
                let k = 2.0 * PI / self.n as f64;
                let re = -0.1 * self.n as f64 * (1.0 - k.cos());
                let im = -0.1 * self.n as f64 * k.sin();
                let integ = 1e5 * 0.05 / 3.0 * (1.0 - (1.0 + t / 0.05).powi(-3));
                for (i, v) in y.iter_mut().enumerate() {
                    *v = 1.0 + 0.1 * (re * t - integ).exp() * (k * i as f64 + im * t).cos()
                }
            }
            7 => {
                if !(0.0..=10.0).contains(&t) {
                    return None;
                }
            }
            10 => {
                let gamma = 2.04_f64.sqrt();
                let w = 500.0 / gamma;
                let phase = w * t;
                y.copy_from_slice(&[
                    phase.sin() / 500.0,
                    (phase.cos() - 1.0) / 500.0,
                    0.2 * t / gamma,
                    phase.cos(),
                    -phase.sin(),
                    0.2,
                ])
            }
            15 => {
                let phase = (1.2 * t).exp_m1() / 1.2;
                let env = (-0.8 * t).exp();
                y[0] = env * phase.cos();
                y[1] = env * (-0.8 * phase.cos() - (1.2 * t).exp() * phase.sin())
            }
            16 => {
                if t >= 2.5 {
                    return None;
                }
                let r = (1.0 - 0.4 * t).powf(0.25);
                let phase = 4.0 * (1.0 - r.powf(2.5));
                y[0] = r * phase.cos();
                y[1] = r * phase.sin()
            }
            19 => {
                for q in 0..self.n / 3 {
                    let l = (q as f64).sqrt().floor();
                    let w = (l * (l + 1.0)).sqrt();
                    let d = 1e-4 * (l * (l + 1.0)).powi(2);
                    let h = y[3 * q];
                    y[3 * q] = h * (-d * t).exp() * (w * t).cos();
                    y[3 * q + 1] = -h * (-d * t).exp() * (w * t).sin();
                    y[3 * q + 2] *= (-d * t).exp()
                }
            }
            22 => {
                let kx = 2.0 * PI / self.nx as f64;
                let ky = 4.0 * PI / self.nx as f64;
                let n2 = (self.nx * self.nx) as f64;
                let lambda =
                    -4.0 * 0.5000005 * n2 * ((kx / 2.0).sin().powi(2) + (ky / 2.0).sin().powi(2))
                        - 0.999999 * n2 * kx.sin() * ky.sin();
                for (q, v) in y.iter_mut().enumerate() {
                    *v = 1.0
                        + 0.1
                            * (lambda * t).exp()
                            * (kx * (q % self.nx) as f64 + ky * (q / self.nx) as f64).cos()
                }
            }
            _ => return None,
        };
        if y.iter().all(|value| value.is_finite()) {
            Some(y)
        } else {
            None
        }
    }
    pub fn diagnostics(&self, y: &[f64]) -> Value {
        let y0 = self.initial();
        let sum = |v: &[f64]| v.iter().sum::<f64>();
        let norm = |v: &[f64]| v.iter().map(|z| z * z).sum::<f64>().sqrt();
        let conserved = match self.id {
            1 => Some((sum(y) - 1.0).abs()),
            2 => Some((y[6] + y[7] - 0.0057).abs()),
            6 | 14 | 17 | 22 => Some((sum(y) - sum(&y0)).abs() / self.n as f64),
            _ => None,
        };
        let energy = if self.id == 9 {
            let m = self.nx;
            let calc = |a: &[f64]| {
                (0..m)
                    .map(|i| {
                        let dq = a[(i + 1) % m] - a[i];
                        0.5 * a[m + i] * a[m + i] + 0.5e6 * dq * dq + 0.25 * dq.powi(4)
                    })
                    .sum::<f64>()
            };
            Some((calc(y) - calc(&y0)).abs() / calc(&y0).max(1e-300))
        } else if self.id == 10 {
            Some(((1.0 + y[3] * y[3] + y[4] * y[4] + y[5] * y[5]).sqrt() - 2.04_f64.sqrt()).abs())
        } else {
            None
        };
        json!({"conservation_absolute_per_component":conserved,"energy_relative_or_gamma_absolute":energy,"minimum_component":y.iter().copied().fold(f64::INFINITY,f64::min),"l2_norm":norm(y),"l2_initial":norm(&y0),"diagnostics_are_not_full_accuracy_reference":true})
    }
}
const NAMES: [&str; 24] = [
    "Robertson",
    "HIRES",
    "Pleiades",
    "Kuramoto-Sakaguchi",
    "Adiabatic-flash negative control",
    "Freeze-out transport",
    "Literal ignition negative control",
    "Oregonator",
    "FPUT beta chain",
    "Uniform-field relativistic gyromotion",
    "Two-scale Lorenz-96 variant",
    "Kuramoto-Sivashinsky",
    "Low-Mach primitive acoustic relaxation",
    "Advected Cahn-Hilliard",
    "Exponential chirp",
    "Inspiral chirp",
    "Spherical rotation-shear FVM",
    "Spherical diffusion-absorption",
    "Linear spherical modal waves",
    "Normalized multipole hierarchy",
    "Periodic 2D Brusselator",
    "Anisotropic 2D conduction",
    "3D Barkley reaction-diffusion",
    "3D absorbing-obstacle transport",
];
const VARIANTS: [&str; 24] = [
    "canonical 3-species; ensemble extension separate",
    "8-species alternative explicitly permitted by request, not POLLU",
    "canonical masses and initial conditions; collision unregularized",
    "local periodic coupling, no self term; radius4 smoke/32standard",
    "literal equations; transverse stiffness persists after boundary layer",
    "periodic upwind, constant velocity, global conservative relaxation, exponent4",
    "literal scalar equation; no thermal ignition occurs",
    "specified 3-variable coefficients",
    "periodic homogeneous stiff springs plus quartic bonds",
    "uniform B and zero E exactly solvable subcase; dimensionless",
    "global fast cyclic ring with attachment's forward cyclic advection",
    "periodic centered semidiscretization; correct negative RHS signs",
    "defined primitive periodic system; not an unspecified IMEX split",
    "periodic 1D centered conservative semidiscretization",
    "specified analytically solvable nonautonomous second-order system",
    "two Cartesian first-order variables; stop before singularity",
    "equal-mu first-order conservative FVM; explicit rotation and shear fields",
    "anisotropic centered fluxes plus fixed angular opacity patch",
    "linearized nonrotating modal acoustic system, not full nonlinear SWE",
    "fixed matter bath and normalized real chains; no full photon-baryon claim",
    "explicit periodic variant; canonical BRUSS2D source/BC not implied",
    "constant oblique tensor9-point stencil, ratio1e6",
    "supplied polynomial is Barkley kinetics, not Gray-Scott/Schnakenberg",
    "periodic upwind advection, centered diffusion, fixed absorption ball",
];
