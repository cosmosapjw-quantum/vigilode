# BASS21 represented-rate finite-state enclosure → VigilODE N03 bounded implicit-stage prototype

Date 2026-10-29. Unit: `VIGILODE_BASS21_FROZEN_NEWTON_BOX_20261029`. Prereg note: the initial goal of actual proper-time BASS21 source-owner interval membership and stage error was taken from prior user-approved static bridge; the tests below are **not prospectively preregistered** in a new PR. No new IVP was run. Original VigilODE 7/23, REI central 28/88, successor 0/12 admitted; production HOLD throughout.

## Authority and units

Actual parent source: BASS PR234, commit `cc602e209e7deeb511ab2a1f3b2268d55206c43c`; original 21-state network Git blob `e3a0603176304f64b953c0b2faf898d6b75eceb5`; original original-source ZIP SHA256 `6a9c84ce076a3545a992f0b364d0fd67fe1063b7a33a290dbe77d44ecee98838`. Replayed from REI PR269 source-inclusive ZIP SHA256 `cecc0920ed52124d04a7ec3224d8f7862e9f3730dbf96058bd8d95ec910f9179`. 129 rate coefficients match BASS Git object `2eba600a9d7a7359ca55f566f1b841a363a3e5c7` at **numeric IEEE754 binary64 value** level, but the coefficient JSON raw bytes were not recovered as such. REI PR269 project 14/14 PASS freshly re-executed in the current session. The source-independent Puchwein19 comparison belongs to PR269, not this same-source theorem.

Let Y_i=n_i/n_b, finite 21-state BASS chemistry, with *fixed* external temperature T, phenomenological UV, baryon density n_b and proper-time rate coefficients (not cosmological x-clock). M=G0+D has units s^-1; R=Gne has units m^3 s^-1; q_i are fixed electronic charge numbers; Y dimensionless. Rates returned by the actual `AtomicNetwork.rates(T,uv)` are treated as **exact represented rational coefficients after IEEE754 conversion**. This is a modeling convention: it is not an experimental rate-uncertainty certificate nor a verified floating-program expression enclosure.

\[ F(Y)=MY+n_b(q^{\mathsf T}Y)RY, \qquad J(Y)=M+n_b(q^{\mathsf T}Y)R+n_b(RY)q^{\mathsf T}. \]

The source's `AtomicNetwork.evaluate` binary64 F/J agrees with this independent reconstruction under explicitly scoped numerical tolerances; exact baryon-conservation of the resulting *represented* J does not hold strictly due to a ~2.03e-25 s^-1 assembly residue (source float accumulation). By contrast the exact rational F for this constructed state gives A^T F=0. Do not silently replace the source's rounded M by a projected conservative matrix; preserve representation roundoff as a separate numerical error.

## Exact whole-state-box lemma (derived)

Fix Y0 and independent box |Y-Y0|≤d componentwise with d_i≥0. Define Q_d=sum |q_i|d_i, R_di=sum |R_ij|d_j and J0=J(Y0). The exact quadratic expansion yields:

\[ F(Y0+δ)-F(Y0)=J0 δ+n_b(q^\mathsf Tδ)Rδ. \]
\[ J(Y0+δ)-J(Y0)=n_b(q^\mathsf Tδ)R+n_b(Rδ)q^\mathsf T. \]

Therefore the following inclusion holds for *every* real Y in the box, even in nonnormal and non-diagonally-dominant cases:

\[ |\Delta F_i|\le dF_i:=\sum_j |J0_{ij}|d_j+n_b Q_d R_{di}, \]
\[ |\Delta J_{ij}|\le dJ_{ij}:=n_b(|R_{ij}|Q_d+|q_j|R_{di}). \]

The same elementwise dJ also uniformly controls the *source-exact analytic* JVP gap: `||[J(Y)-J0]v||_(w,∞) ≤ max_i sum_j dJ_ij |v_j| /w_i`, which is at most `||ΔJ||_(w,∞) ||v||_(w,∞)`, with `||ΔJ||_(w,∞) ≤ max_i sum_j dJ_ij w_j/w_i`. This does **not** bound an arbitrary numerical matrix-free callback's additional application error. The state box can be intersected with the baryon/isotope conservation simplex; keeping the larger box is a valid conservative relaxation.

## Valid one-stage meaning (not RODAS)

At a trial state Y with fixed external bath, the implicit-Euler Newton correction with initial predictor equal to the previous state is `A(Y) δ=h F(Y), A=I-h J(Y), h≥0 proper seconds`. This defines the local *family* of linear systems tested here. It does not assert that the actual production RODAS RHS equals `hF`, nor that the source is frozen throughout a trajectory. Fix weights w_i,p_i>0 and `mu_up=max_i (J_ii^upper+ sum_{j≠i} max(|J_ij^-|,|J_ij^+|)w_j/w_i)`. When `1-h mu_up>0`,

\[ \|\hat\delta-\delta_*\|_{p,∞}\le C_{wp}\frac{\rho_w^\uparrow}{1-h\mu^\uparrow},\quad C_{wp}=\max_i w_i/p_i, \]

where `rho` is the outward original residual of the same hF/J box. Both original residual and p-physical stage-error budgets must pass. The BASS source-node owns F/J and is explicit, but the user-selected w/p, h, and the generated trial belong to this research **fixture**, not an authenticated owner-issued RODAS transaction.

## New exact and computational evidence

- Source-native REI PR269 archive SHA/CRC restored, re-execution 14/14 PASS.
- Source-specific polynomial reconstructed from the *actual 129 numeric BASS coefficient values*. Python 9/9 PASS after two preserved RED findings: (i) a test-code SyntaxError; (ii) a true source representation defect in `A^T J` (j=5 ~2.02958e-25), which was separated from theory instead of erasing it.
- Rational exact 21x21 Jacobian box: 48×21×21=21,168 inequalities PASS. RHS: 48×21=1,008 exact inclusions PASS. Source-exact JVP and signed norm dual gate negative tests PASS.
- Rust1.94.1 original PR112 N03 interval source used byte-identically in `rust_bridge/src/lib.rs` plus a new read-only 21-state CLI. Rust 11/11 PASS, strict Clippy and rustfmt PASS. Rust BASS21 source-box proof `rho=2.94696241932524035e-15`, stage error `5.90737514636671833e-15`, μ=0.00227686703196838076 s^-1, upper inverse norm 1.00228206298594635. Exact Fraction model's rho=2.946962419325222e-15; interval encloses it. Source box relative radius 1/256, sigma=1 s.
- Five finite radius values 1/256–1/4096; 24 spot checks per radius and a plot. Largest sampled error is below source-conditional bound; these finite samples are not a rigorous trajectory/global interval proof. No speedup claim or timed full-IVP.
- Wolfram exact 2×2 nonnormal electron feedback and F/J box checks using explicit rational values gave TRUE, TRUE. This is CAS verification, **not a distinct-author R6/C9 independent audit**.

## Important finite trust and transaction boundary

VigilODE PR112 N02 right FGMRES calls N03 `monitor`, then performs final `output.copy_from_slice` *after* callback return. A pair of live SourceStamp reads cannot make this output copy atomic if a different thread/owner may mutate the physics epoch in between. Correct designs include (i) hold an owner-issued read lock across the certified monitor and final output publication while all source mutation requires the corresponding write lock; or (ii) copy into an isolated unpublished buffer and use an owner-issued compare-and-swap/transactional publish callback. The current research CLI has no actual such shared owner implementation. A string, SHA, epoch bit pattern or filename without authoritative ownership cannot substitute for that lock.

The current BASS rate evaluation has frozen T,UV,n_b, proper-second clock. It does **not** prove any time/cosmological-geometry-variation envelope, RODAS tableau RHS b, dynamical 17→21 REI equivalence, BASS full62/33/30 Jacobian or full physical JVP callback accuracy. REI PR265 stored pilot isotope/T and He/H mismatch remains; PR269 different UV source shapes remain separate models. Source-owner/cross-API real stage membership stays BLOCKED; no main merge/force push/production.

## Reuse context

Methodological context: J. Maidens and M. Arcak, *Reachability Analysis of Nonlinear Systems Using Matrix Measures*, IEEE Trans. Automatic Control 60 (2015), DOI 10.1109/TAC.2014.2325635. Y. Lin and M. A. Stadtherr, *Validated solution of ODEs with parametric uncertainties* (2006), source `https://academicweb.nd.edu/~markst/lin-stadtherr-cacheg2006.pdf`. These references motivate interval/measure methods; they are **not** implementation proofs for this code.
