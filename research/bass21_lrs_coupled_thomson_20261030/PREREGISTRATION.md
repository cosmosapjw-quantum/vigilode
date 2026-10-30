# Prospective preregistration | BASS21 LRS frozen-background polarized Thomson response

Registered BEFORE running the new coupled Volterra numerical experiment. This is not a retroactive registration of PR130.

Date: 2026-10-30 KST. Parent immutable PR130 HEAD: `90304ae420ff61d20b3ccb8d6ca8feeea896c05b`.
Source: original BASS21 50 x 193 62-state trajectories, immutable nested archive SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`, BASS owner `cc602e209e7deeb511ab2a1f3b2268d55206c43c`. Parent archive SHA256 `d6aaaf75aa82f083eddd577f9d0f44d54fe5fdfdd2978fe4882a6d986edc3e2d`.

## Prediction and actual experiment

On the frozen original unpolarized angular intensity baseline `I0(mu,x)=sum_l(2l+1)J_l(x)P_l(mu)`, derive the axisymmetric full Thomson **intensity/polarization gain coupling** for a perturbation `deltaI` and Stokes `Q` with an unaltered geometry, free-electron history, and optical clock. Use normal-frame `Q=I_meridional-I_azimuthal`; screen transport along exact LRS diagonal Bianchi I null rays. This is a self-consistent linear polarized-photon response on frozen BASS21 fields, NOT a backreacting Einstein-Boltzmann/chemistry IVP.

Define `j0=integral_0^1 deltaI dmu`, `j2=integral_0^1 deltaI P2 dmu`, `B=2 integral_0^1 Q(1-mu^2)dmu`. Derive and independently check source terms `S_I=j0+(j2/2-3 B/8)P2`; `S_Q=[-3(J2_original+j2)/4+9B/16](1-mu^2)`. Redshift/extinction Green function `e^(-Delta tau)/D^4`. Solve both coupled causal second-kind Volterra equations with sampled-grid composite Simpson and fixed-point iteration. Initial Q=deltaI=0, NO NEW IVPS.

Original outputs: all 50 histories x 193 saved epochs x 17 nonnegative half-sky directions for Q, deltaI, full normalized I; 25 matched shear-pair deltaQ; endpoint 50 and pair CSV and new observables. Internal angular quadrature: GL24, GL48 refinement. Other checks: homogeneous b=0, constant J2/g exact matrix exponential, zero-pole Q, zero-initial state, finite physical I and |Q| <= I, nonnegative photon trace. Compare first Picard iterate against PR130 Q. Time-grid 193/97/49 checks are quadrature-only, NOT original BDF global certification.

Acceptance: exact immutable binary source hash; Wolfram symbolic angular coefficients; independent tensor-quadrature or equivalent exact check; numerical residual for fully coupled operator < 1e-11 in source normalized units; stable 24 vs 48 angular refinement reported quantitatively, without tuning tolerance to obtain a pass; all 50 histories retained. Record failures rather than conceal. Extinction and polarized feedback included, but full polarized source evolution of BASS baseline and electron/Einstein backreaction are NOT claimed; finite z=2 model frame, no observed CMB E/B.

Policy: independent R6/C9 distinct-author = NOT_RUN unless actual dispatch; research ledger of original parent must remain failed; all parent/main history protected. No force-push, no merge, no production promotion, no tolerance weakening, no new original or polarized IVP; stop after this independent node.

New scoped research node can be marked complete only if full physical rows, source reconstruction, residual and verification pass; original VigilODE 7/23, REI central 28/88, working 33/88, BASS CE1 9/15 and successor 0/12 remain unchanged pending formal admission.
