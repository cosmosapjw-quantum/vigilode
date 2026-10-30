# PR134 successor | BASS21 J/c Generator Physical Data

**Unit:** `BASS21_SOURCE_OWNED_JC_COEVOLUTION_GENERATOR_20261030` (scoped enabling result, not a full native E_l evolution).
**Immutable parent:** VigilODE PR134 HEAD `21373cf012f348ad6911580461ab466016968433`, original BASS owner commit `cc602e209e7deeb511ab2a1f3b2268d55206c43c`, original source ZIP SHA-256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`.
**Prospectively pinned** new calculation before 50-case campaign in child commit `ffa82d2885590c097b808bef1c70e210a5a41dbe`.

Original 50 cases×193 source epochs restored and crosschecked against PR133 deltaJ_0..10 and PR134 c2..10. No original or new IVPs and no source substitution. A new 115,800-value coupled scalar-J / screen-spin2 local RHS atlas and 48,250 new high moments were calculated on identical source-owned frozen background; exact Legendre projection coefficients derived and independent low-rank Wolfram check passed. New higher coupling through J12/c14 is nonzero at high shear and resolves relative to GL64/96; low-shear c12 is below numerical resolution. Exact radiative transfer *operator* conditional on source model only, not full BASS-native J/E coevolution.

Target validation: source SHA 50/50, canonical ZIP SHA, CRC, 13/13 focused unit tests, numerical clean replay and SHA on artifact package; previous `J0` work residual <=4.41e-11 and sampled-grid derivative residuals retained. Hostile reviewer same author, not external; native R6/C9 NOT_RUN. Scope and source/precision limitations detailed in `THEORY_AND_AUDIT_KO.md`.

**DAG unchanged:** VigilODE 7/23; REI central 28/88 and working 33/88; BASS CE1 9/15; successor admitted 0/12; official delta 0. Auxiliary JC generator scoped 1/1, production HOLD. No main merge, no force push, no native source replacement, no new IVPs, no weakened gates.
