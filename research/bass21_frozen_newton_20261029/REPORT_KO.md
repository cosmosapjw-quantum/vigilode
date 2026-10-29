# VigilODE ↔ BASS21 고정 실제 원천 rate / N03 original-stage certificate 연구 보고서

2026-10-29. `VIGILODE_BASS21_FROZEN_NEWTON_BOX_20261029`. Production HOLD. Original VigilODE 7/23, N03 historical 17/20 scoped, central REI 28/88 (working33/88 separate), successor admitted 0/12: **all delta 0**. 이번 신규 R/C 단위 18/20 scoped; R6/C9 actual different-author review NOT_RUN; native subagent dispatch NOT_RUN. Stage checkpoints are retrospective source/evidence receipt, not 20 independent computational executions and not contemporaneous provider ACK.

## Repository cross-check

- VigilODE PR112 HEAD `f28938cd17dca8c5d1390f432b11f6dcfd8f307c` Draft/Unmerged. N02/Givens + N03 interval crate; source stable. PR113 HEAD `beee9aaf4f83eaedb5e4a3167cd3f42059215ec8` independent point N01 on PR111. PR114 **live** HEAD `a62c4215fa2c329c6a48be2ccf544ac056b747a5` time-affine JVP, no BASS nonlinear admission; CI failures preserved.
- Original BASS PR234 physical chemistry source `cc602e209e7deeb511ab2a1f3b2268d55206c43c`. Newer BASS PR256 `24d94a6f1743fa4b1b7bdd16c7476e04b11c9240` repairs a distinct MPFR false-certificate issue and is **not** chemical-stage owner. BASS original source ZIP exactly preserved. Rate projection numeric-content identity 129/129 from PR269; original rate JSON raw byte identity not asserted.
- REI PR265 `9b0b97aadeebf368802f865c10eb8b633f2726e4` original pilot mismatch; PR266 `a3470c88b4bddb38a08623d37ff6aa61210e04ae` Li6 exact BASS-internal lift; PR267 rank-one Jacobian; PR268/269 two sibling actual-rate/source-shape paths. PR269 HEAD `17d676810cb59552e3f6f0d7f981ebccf380d955`: restored 828,561-byte archive SHA256 `cecc0920ed52124d04a7ec3224d8f7862e9f3730dbf96058bd8d95ec910f9179`, nested original BASS 222,443-byte ZIP SHA256 `6a9c84ce076a3545a992f0b364d0fd67fe1063b7a33a290dbe77d44ecee98838`, newly run 14/14 Python PASS, no IVP. PR269 CI workflow 40876859684 overall FAIL from `rust/rei_microphysics` cargo fmt check; a prior snapshot's queued status is superseded.
- Adjacent newest inspected: bass_cr PR124 (`a6b49b352fed31587955d55b1cad30c0b55a81eb`, exact Ferrers flow), BASS_HE PR111 (`0e643121b62a65dd821ea563051c59722a828a8f`, conditional angular ratio), rec_bianchi PR158 (`ee78d209281c28ad7b8a9fe04a1634da80b7ad54`, HD recoil). None provides BASS21 RODAS stage b, JVP source owner or shared isotope/UV/clock; not imported as such.

## New science

`THEORY_KO.md` gives source-structured exact whole-state-box `dJ,dF` formula with real BASS represented rates, and a valid **one frozen implicit-Euler Newton correction** `b=hF(Y)`, `J=DF(Y)`, not actual RODAS5P tableau. This binds the frozen original chemical RHS and J from a single BASS state/rate family. Arbitrary callback JVP, source-time bath change, full BASS 62/33/30 coupling, REI17 cross-model identity, native RODAS stage-b membership and production global-IVP remain OPEN/BLOCKED. Source identity is a pinned numerical content, not a live authoritative epoch lock.

## Fresh execution

- Python 3.13.5 and NumPy 2.3.5: new standalone 9/9 tests PASS; 21,168/21,168 exact Fraction J cell bounds and 1,008/1,008 exact Fraction F bounds; nonstationary owner/source errors excluded by scope. Initial syntax RED and an authentic `A^T J` roundoff RED preserved; repaired test assumption without mutating physics, source or solver tolerance.
- Original PR269 `bash VERIFY.sh`: 14/14 PASS from restored exact ZIP.
- Rust 1.94.1: source-original PR112 N03 `src/lib.rs` SHA256 `1a805d198255a18263acad5b3a3a1b7f52d67468d99a9574265b3409e283d35d` copied **byte-identically** into standalone, with new 21-state input binary; original tests 11/11 PASS; strict Clippy PASS; rustfmt PASS. No full native VigilODE workspace build in this run.
- `h=1 s`, `d_i=Y0_i/256`, w=(2 for i mod5=0; 1 otherwise), p=1 all, T≈11997.560879134957 K, UV≈0.9997950318600803, nb=1e6 m^-3. Python rational μ≈0.0022768670319683712/s, inverse ≈1.002282062985946, original-residual bound ≈2.946962419325222e-15, physical stage bound ≈5.907375146366675e-15. Rust outward same source box: μ≈0.00227686703196838076/s, ρ≈2.94696241932524035e-15, bound≈5.90737514636671833e-15; correct outward direction. These are intentionally loose **research budgets 1e-6**, not physical33 production thresholds.
- Five radius values (1/256...1/4096) and 120 numerical spot error solves, all below exact rational-source bound; plot generated and *actually visually read*: monotonic ~linear shrinking envelope, all samples below bound, no anomalous sign reversal. These samples are not rigorous global error/IVP or speedup.
- Wolfram actual rational CAS for J and F inclusion at fixed positive 2×2 state returned `{True,True}`. Diff assumptions tested; different-author R6/C9 still NOT_RUN.

## New useful failure classification

Original BASS source rounded `M=G0+D` has `A^T J` column5 nonzero ≈2.0295813369526192e-25 s^-1 in exact representation. The nonlinear *analytical* network conservation remains true at source-formalism level; the observed finite-roundoff implementation residue is not a new physical conservation violation. `A^T F0=0` for this represented-frozen sample, and numerical source matches independent polynomial within declared diagnostic tolerances. CI of PR114 fails at least fresh-clone rustfmt in native shared tree; other historical authority/usable-solver failures retained and not declared repaired. REI PR269 verify fails inherited Rust fmt in unrelated `rei_microphysics`, not the newly rerun focused 14-test script.

## Scientific verdict and next node

- **Admitted only scoped**: source-native BASS21 *represented-rate frozen-state* polynomial F/J exact rational box; its JVP **source variation** bound, conditional N03 original residual and stage-error enclosure of *one implicit-Euler Newton correction*.
- **Not admitted**: source-owned RODAS5P stage RHS, full62/33/30 source Jacobian, arbitrary JVP callback, real epoch-atomic output transaction, whole-slab variable coefficient interval, global error, physical tau, REI-BASS common-model fit and production.
- **Next:** `ACTUAL_RODAS_STAGE_RHS_AND_ATOMIC_EPOCH_BIND`, in isolated future branch, without IVP. Read actual VigilODE RODAS tableau and stage history; derive b and J using exactly one BASS source/clock/geometry owner. Implement a single owner-issued snapshot that keeps source/epoch read lock through N02→N03 final output transfer; add race/negative tests. Do not alter parent, force push, change tolerance, merge or promote. `NO_MORE_IVPs=true` stays.

All original Q-LEDGER/Q-API/Q-GIVENS, eight suites, ten interactions, correction budgets, historic CI failures and negative evidence retained as constraints. New packaged files, exact SHA256 and independent provider acknowledgements are in `DELIVERY_RECEIPT.json` *outside* the immutable source ZIP when those actions actually succeed.
