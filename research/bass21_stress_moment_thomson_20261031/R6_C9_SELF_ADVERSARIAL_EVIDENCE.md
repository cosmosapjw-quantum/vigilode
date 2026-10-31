# BASS21 moving-electron Thomson stress-moment extension | bounded self-adversarial R6/C9

Date 2026-10-31. SELF_ADVERSARIAL_ONLY, no distinct author C9, native subagent NOT_RUN. The exact identities are derived; their numerical implementation was validated only in a standalone sandbox model, NOT original owner 50x193 BASS data. This branch makes no authority or DAG admission claim.

## Science audit
1. Q sign: The stated Q=G11-G22 uses outgoing electron-rest screen (z,-y) for n'=x. The exact rest-electron axis limit Q/I0=-3q/20 is a sign-sensitive control.
2. Doppler dipole: Omitting beta*d/2 or finite-beta denominator is incorrect for anisotropic incident radiation. Correct exact axis result: [-3q/20+beta*d/2-beta^2*(1-q/20)]/(1-beta^2).
3. Fake high moments: With unpolarized incident radiation, rank-2 stress tensor has no ell>=3 angular information. A false +beta*o/(1-beta^2) candidate yields exact rational residual 5/264 at a fixed rational probe. This no-coupling fact does NOT extend to full polarized collision hierarchy/streaming.
4. Numerical accumulation: Early float64 direct angular quadrature had up to 4.8e-13 accumulated mismatch and three tight negative unit outcomes. Code switched SUMMATION ONLY to numpy.longdouble without relaxing tolerances; subsequently fresh 14/14 tests passed. Max Lorentz/direct moment error 1.7763568394002505e-15; max GL64/GL96 Q difference 1.7208456881689926e-15.
5. Gauge and frame: Q+iU is spin -2 relative to local screen; electron-rest gain has not been fully transformed back to normal-frame optical-depth collision operator. Global pole patches and frequency/boost screen restoration remain implementation tasks.
6. Authority: Historical parent scoped CI 41181443097 SUCCESS; global 41181443124 FAIL in Check authority commit references are published, which is unrelated to this local scientific test and not fixed/suppressed. No main/owner/production mutation.
7. Proof tooling: SymPy exact and independent Wolfram symbolic axis residual were evaluated. Native OpenEvolve, opam, OCaml, HOL Light kernel, native subagents and distinct-author reviewers were NOT_RUN. pip install openevolve==0.4.0 failed due offline/DNS. Eight actual alternating UCT/archive feedback waves evaluated 19 model-preauthored symbolic expressions, six exact equivalent, thirteen refuted by exact rational counterexamples and eight elite archive cells. Not library-driven generation or autonomous LLM calls.

## Measured physics and preservation
Four prescribed backgrounds x three epochs x three electron velocities x eight outgoing = 288 actual source-free gain rows; 36 direct angular quadrature vs Lorentz moment checks; 36 stress rows. 14/14 new tests pass. New exact SymPy receipt and five figure pairs PNG+SVG.

New full local downloadable ZIP VIGILODE_BASS21_STRESS_MOMENT_THOMSON_EXPLORATORY_20261031_SOURCE_FULL.zip has 1,069,733 bytes, SHA256 8280350e04b2c4d715c3abe1a95d6311da0a0b315b00ce798506cf2c0cb66111. ZIP has 43 entries, 15/15 source manifest, 18/18 generated science/figures byte-exact, embedded predecessor sourcefree ZIP SHA256 8797aac0207095a8e8aa2a004c487a42d18922aee8c6e4f55b1a8472fd908641. Cleanroom extraction and VERIFY.sh exited 0 with 18/18 regeneration. Git receipt is NOT a second independently provider-backed binary ZIP; new ZIP Google Drive/Dropbox ACK is NOT_VERIFIED.

Git SOURCEFREE_REPLAY.py is a compact alternative generator. Full sandbox-tested code, tests, figures, datasets and proof search are in local ZIP. Compact Git script has NOT yet passed separate remote/native CI.

DECISION: formula DERIVED; sandbox code IMPLEMENTATION_VERIFIED_SCOPED; values NUMERICALLY_CHECKED; observational and owner linkage UNRESOLVED; proof assistant / distinct-author C9 NOT_RUN. NO_MERGE, PRODUCTION HOLD, NEW_OWNER_IVPS=0, VIGILODE=7/23; REI CENTRAL=28/88 WORKING=33/88, BASS CE1=9/15.
