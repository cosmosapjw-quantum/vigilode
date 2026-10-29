# Copyable VigilODE N00F → native N01 handoff (2026-10-29)

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
PR109_PARENT=2d430a7aa81070da17a7a5dba60a2d56639b82db
PR104_NATIVE=10854bc0bff81b6d3cf5cb0a44520ed9165c205f
NATIVE_GMRES_BLOB=90bfaaa62afeda9dec27b3dfd594e67cd3034da1
PR107=1e604c7d9235e773f95a95efadff3258bbc3d0d3
PR108_CMODE01=611e697ac4c4beb10b87573e985a95a9bc70ce09
N00F_PREREG=4939cf8460171c7a2e07a7c1e58147e471488174
RESEARCH_BRANCH=research/native-gmres-flex-guard-20261029
ARCHIVE_SHA256=c05bf14dc5ca3916710663caa77608eb37da2946d24c1b9bd453277a58437d2b
DAG_CORE=7/23; REI=28/88; SUCCESSOR=0/12 admitted; AUX_N00F=1/1 scoped
PRODUCTION=HOLD

Restore original 77,957-byte 22-file N00F archive from Drive file `1PpiLA4it9973yLNWdlyGBCcGnixoTQpi` (raw SHA256 checked) or Dropbox id `id:BSpOijBcT10AAAAAAEHVYQ`; validate SOURCE_MANIFEST.json and run Rust1.94.1:
```
cargo test --locked --offline --all-targets
cargo clippy --locked --offline --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run --locked --offline --release --bin oracle > oracle.csv
python3 verify_oracle.py
cargo run --locked --offline --release > bench.csv # only on explicit new bench loop
python3 analyze.py
```
These are isolated synthetic tests, NOT native tests; do not increment an official DAG node. One of max three benchmark loops already used.

N00F exact negative: for A=I, b=2e1, nonlinear `P(z)=(1+||z||²)z/2`, projected residual=0, naive P(Vy)=5e1 residual3; FGMRES stored Z*y=2e1 residual0. Native `gmres_into.rs:699-720` reconstructs with P applied to combined V*y each Arnoldi column and monitors afterward. Require stationary-linear PC contract or opt-in stored-Z FGMRES. Do not silently conflate projected left residual/WRMS/L2 with original physical weighted-infinity residual. Source/epoch/clock are not owner-verified by N00F.

NEXT_NODE_NATIVE=N01 source owner: actual J0, b0, callback apply error eta, deltaJ, deltab, physical weights w,p and epoch/clock. With sigma>=0 and sigma[mu_w(J0)+deltaJ]<1, certify rho_star<=||b0-y||_w+deltab+eta+sigma deltaJ||xhat||_w, stage<=max(w/p)*rho_star/[1-sigma(mu_w+deltaJ)]. Use outward bounds and original-residual AND physical-error gates. Compare native point-J with independent exact oracle.

THEN_N02=opt-in incremental Givens trigger BEFORE candidate/backsolve; preconditioner fixed-linear proof or flexible stored Z. Preserve warm starts, restarts, budgets, counters, partial callbacks, no output mutation and native default path. Same-physics/tolerance/achieved-error ABBA full-IVP before speed claim.
SYMBOLIC_DYNAMICS=use whole-slab cell enclosure and certified transition graph, NOT endpoint-only itineraries or asserted global conjugacy.
NO=main merge, force push, tolerance drift, native performance claim from this synthetic timing, undocumented source or profile changes.
RETURN=exact source SHA, commands/exits, negative controls, full-IVP error and work profile, independent reviewers R6/C9, valid full authority, dual-provider checksum readback and next handoff.