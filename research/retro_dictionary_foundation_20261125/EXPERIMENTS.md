# Opt-in experiment entry points

Build:
    cargo check --locked --offline -p vigilode --release --features retro-dictionary
    cargo test --locked --offline -p rodas5p-core --release --features retro-dictionary --test retro_dictionary_contracts -- --nocapture
    cargo test --locked --offline -p rodas5p-integrators --release --features retro-dictionary --test retro_foundation_contracts -- --nocapture

Owner-host diagnostic timing (use cores belonging to distinct physical cores):
    cargo test --locked --offline -p rodas5p-integrators --release --features retro-dictionary --test retro_foundation_contracts measure_foundation -- --ignored --nocapture --test-threads=1

Core crate entry: rodas5p_core::retro_dictionary::Dictionary.
Integrator entry: rodas5p_integrators::retro_foundation.
Facade re-export: vigilode::retro_research (feature retro-dictionary).

## Combinations (only compatible contracts may be composed)

| Work item | Independent experimental axes | Acceptance contract |
|---|---|---|
| real scalar functions | j=0..8, 57 cells, TableFast/TableDirected/Scaled/Hybrid | represented argument and source table error |
| Bernstein | p=0..7, k=0..p, table/scale/hybrid | integral of declared Bernstein basis |
| batched table | 9/36 lanes, scalar/packed, caller local pool | exact equal interval endpoints in tested cases |
| Green | legacy vs full dictionary, step cap, three/five/seven-node weights, triangular carrier | original ODE or explicitly normalized linear action |
| complex phi | j=0..8, Taylor vs recurrence | candidate-only; no directed complex certificate |
| Legendre | p=1..6, normal equations/rational endpoint, product table p<=8 | scalar rational enclosure or candidate matrix |
| Picard | three normalized-time matrices, ordered words up to length 5 | original linear action plus tail |
| prepared RHS | exact same matrix, RHS count, serial/faer/local Rayon | original matrix residual and strict row inverse gap |
| tensor quotient | 1..3 axes, separate moduli, monomial terms, direct/folded | original coefficient residual and inverse-gap |
| aperiodic | different transitions, noncommuting order, sequential/ordered scan | original all-node infinity error |
| existing research | cubic, Fourier, nonlinear jet, temporal, causal, affine event, spatial | retain each existing typed contract |

These are available axes, not a claim that their Cartesian product was
exhaustively executed. The direct candidate report can contain a bound
greater than the requested tolerance; only Portfolio::choose's Qualified
receipt authorizes acceptance under that tolerance. Table mode and thread
policy are explicit. Do not reinterpret local action bounds as a full
nonlinear physical IVP bound. A changed matrix or shifted stage requires
a fresh owner unless exact identity is established.

## Follow-up integration

M0/M6 benchmark: retain reference flags, both original datasets and all
expanded external solver arms. Add only physical-output compatible clients,
and record source identity, preparation vs reuse, actual steps, output
contract, rejected work, constraint/event residual and actual global error.
The new table/action and represented-recurrence microbenchmarks are not
additional full solver benchmark rows.

M1 event: use existing affine_event_certificate and facade event contracts;
for general polynomial/tangent events add source-bound root isolation and
transactional stop/reset before making a generic event claim.
M2 DAE: prepared solves are a building block for declared algebraic blocks;
consistent initialization, nonlinear constraint projection, index/conditioning
checks and DAE dense output still required.
M3: reuse PreparedRhs and existing shared_shift_policy; measure true
cold/warm crossover with changing shift/matrix/epoch and amortized preparation.
M4: data migration and typed computational access done; next build further
physical clients and directed complex support.
M5: use normalized polynomial actions and Aperiodic, preserve physical clock
and original defect on arbitrary slabs. Ordered scan is slower in the
measured 256-transition cases and is not the default.
