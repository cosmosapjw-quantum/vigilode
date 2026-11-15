# Original REI F05: native Rust 46,080 × 9 thermal source roots

Research 2026-11-15, VigilODE Draft PR #183, opt-in native source-physical research. This result is NOT a complete native 26-event MPRK22/SDIRK2 interval IVP, NOT the fourteen native FINAL ledgers, NOT a global continuum error bound and NOT a production promotion.

## Locked source and implementation identity

- Previous VigilODE c1132ed4d1d9d2490924154150232e57a3bb7c13: genuine 46,080 × 3 positive H/He MPRK22(1) final-corrector interval implementation.
- Original REI source owner f1b20b132e1ecc631c379e2d7ca6960cfdb8daaa was checked out read-only. Sealed original tar SHA256 a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b; kernel SHA256 579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644; original accepted tick160 state array SHA256 cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01.
- Read-only Python original-map hook extracted predictor/stage/final root parameter intervals on FULL/HALF1/HALF2 original nonautonomous proper clocks and all 46,080 source cells, including exact REI photoheat/OTS source context, expansion, nine cooling factors, parent U0, original SDIRK weights and thermal reference boxes. Source original owner 14/14 final ledgers PASS and zero table events. The original source and V1 history were not mutated.
- Frozen 414,720-context source binary (106,168,632 bytes) SHA256 73ed05ef73ebc2b098d70aa66970b9e6ceb2bcb567c13fa80b2efc4017ebce3a.
- Copied exactly the historical REI Rust verified ln/powf interval arithmetic (SHA256 e4639a4386feee536dbe7e7ffb1260f26fa00697b6b05477083c182ddf03de18) and used existing VigilODE rodas5p_core directed arithmetic for all basic interval operations and exp/sqrt; no generic solver default or protected core changed.

## R5 theorem and numerical formulation

Let x=ln(T/K) and U=A exp(x) where A=3/2 k_B (N_H+N_He+N_e), gas U in erg, per-cell extensive particle counts. With original step h seconds, Alexander SDIRK2 weight w and source parameter vector θ:
F_θ(x) = A_θ exp(x) - U0_θ - h [ C_θ + w R_θ(x) ].
R_θ(x) = Q_photo,θ - sum_(j=1 to9) K_(j,θ) Λ_j(exp x) - E_exp,θ exp(x).
Here R_θ already is total Udot, do not double-add an electron-count energy term. All nine source H/He recombination, excitation, collisional ionization, and free-free cooling formulas/derivatives are reused, no twice-counted 2p excitation.

For every θ in the original source interval family, if outward F_Θ(L).hi<=0, outward F_Θ(H).lo>=0 and outward derivative ∂_xF_Θ([L,H]).lo>0, then F_θ has exactly one root for each θ in [L,H]. Uniform interval Newton [L,H] intersect (m-F_Θ(m)/D_Θ([L,H])) retains the true root; any failed sign/derivative/positivity/source identity yields not certified, never a false PASS.

Primary method references: Kopecz & Meister, On Order Conditions for modified Patankar–Runge–Kutta schemes (2017), https://arxiv.org/abs/1702.04589; R. E. Moore, A Test for Existence of Solutions to Nonlinear Systems, SIAM J. Numer. Anal. 14 (1977), https://epubs.siam.org/doi/10.1137/0714040. Original source formulas govern astrophysical specifics. A Wolfram symbolic test gave {0,0} for two nontrivial source cooling logarithmic derivatives.

## Actual original science and independent check

Native Rust with eight Rayon threads:
- 414,720 / 414,720 actual original source thermal roots certified, all nine original source-clock groups, uniform F sign plus positive derivative.
- 414,720 / 414,720 root intervals overlap original source owner intervals; 414,720 / 414,720 independently evaluated thermal RHS intervals overlap source owner boxes.
- Separate stdlib + mpmath 80-digit sampled point source checks, 54/54 PASS, same-author and independent numerical implementation only.
- Three opt-in native Rust executable tests, 3/3 PASS.
- Original native 5-contraction output 17,003,544 bytes SHA256 5b7aef979237dc4aae89d4248e733496949a1707c2587af815eec59c021b9777, native source root-only 67.7721 s.
- Independent additional native extensive first-law subledger, 414,720 / 414,720 U-U0-h(C+wR) residual intervals contain zero. Full FINAL total source ΔU outer [-8.332949746717272e49, +2.236639993473039e49] erg STRADDLES 0. Exactly 25,704 / 46,080 full-final cells separately have strictly negative ΔU upper endpoint. Do NOT claim the full gas globally surely cools.
- Original 14 final ledger set PASS belongs only to original REI source owner, native Rust has NOT implemented the full fourteen final ledgers.
- Source full original source/native/auditor/energy second run reproduced bit-identical raw input, root output, audit JSON, energy subledger and original owner CSV (4 binary/readback checks plus owner CSV). Original runtime source pinned but locally NumPy 2.4.2/Pandas 3.0.0, unlike seven exact original REI V2 production research package versions; seven-pin runtime byte identity NOT CLAIMED.
- Hostile tamper source SHA, fabricated tick161 admission, corrupted native root in normal and optimized independent audit, corrupted raw first-law Rust: 5/5 rejected. Audit fails closed under Python -O; no distinct-author referee, native subagents NOT_RUN.

## Real C8 bounded root-only optimization

Three verified interval Newton contractions vs five default on the EXACT same original 414,720 source root contexts:
- Three-contraction 414,720/414,720 uniformly certified, owner root/RHS overlaps complete, 54 mp80 source-point tests pass, zero new rejected cells.
- Three-step raw SHA256 22e033ca3a102e164676649e765ad46e825f96875288a4b10d15d34b65ea518e.
- Three-step native root-only wall 45.872613992 seconds; independently run current-binary five-step wall 62.048014995 seconds (three/five ratio 0.7393083242984734, 26.1% shorter). Earlier five-step pre-optimization 67.772066553 seconds; timings vary with host background conditions and are not global matched-error IVP speedup.
- The five-contraction root interval is a STRICT mathematical subset of or equal to the three-contraction interval at every one of the 414,720 source contexts; 0 exact inclusion violations. 391,321 source contexts have byte-numerically equal root intervals; maximum observed three/five width ratio across nine groups is 1.0000002423844032. The exact current-binary five-step source output is byte-identical to pre-optimization baseline SHA256 5b7aef979237dc4aae89d4248e733496949a1707c2587af815eec59c021b9777.
- Opt-in environment REI_F05_NEWTON_STEPS=3; original five-step default remains unchanged. All source-physics gates remain enforced. The current-source C8 fully auditable all-cell comparison is implemented in compare_newton_3_5.py and recorded in private FAST3_VS_5_VERIFIED_CURRENT_SOURCE.json.

## Scientific decision, blockers and next work

ADMIT source-correct thermal implicit substep verification and bounded local reduction of redundant interval Newton operations only. HOLD native full source-coupled original 26-event H/He/OTS/thermal interval time map, all fourteen native final energy/photo/nuclei ledgers, positive HeII subnormal transfer, continuous-time all-X/Volterra stability and genuine whole-IVP achieved-error speedup. The original V1 failed independent-box logT 2.1245050576368385e-4 > frozen 2e-4 is preserved; official accepted tick160/131072, tick161 NOT_ACCEPTED. Versioned V2 owner research tick162 and tick163 are separate from original V1.

Canonical DAG unchanged: VigilODE 7/23, successor 0/12, REI central 28/88, working 33/88 (nonadditive), fast 4/7, canonical Δ0. Draft HOLD: no merge/default/main, no force push, no threshold relaxation, no original source/worker/history mutation.

Next ONE productive science unit: connect native verified MPRK full predictor/stage/final original source intervals AND certified thermal roots self-consistently at actual source clock, compute complete native fourteen original final signed set ledgers on full/two-half, compare native interval outputs with actual versioned REI V2 parent. Then rigorously bound continuous-time positive tube/rank3 photon moment coupling and measure full-IVP achieved-error cost before promotion. No additional isolated photon-only atlas or governance loop.
