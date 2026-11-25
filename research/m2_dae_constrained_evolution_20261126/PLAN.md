# M2-DAE-CONSTRAINED-EVOLUTION-02
One bounded node. Use immutable DaeSource, declared fixed algebraic branch
box, actual native mass and Schur-reduced ODE. Initial projection explicit.
Point checks at accepted nodes and arbitrary dense queries project z only
and keep represented differential x. For reduced RHS/Jacobian/time
derivative use certified initial-point projection for each callback;
no mutation based on rejected callback history. Deterministic seed.
Original residual and exact branch authority per query. Raw mass and
projected paths distinguished, no mutation of stored driver trajectory.
Continuous branch certificate on each actual dense polynomial slab:
fixed B, time interval and differential polynomial hull, uniform
preconditioned contraction, local self-map in B. For physical state
tube use additional prescribed E_x radius (bootstrap); distinguish
branch along numerical x from physical IVP trajectory. Derive constraint
distance <= ||C g(t,U,V)||/(1-q) on B, original differential defect
r=U'-h f(t,U,V). Reduced Jacobian enclosure from inverse defect, include
h ||f_z|| times constraint error. Propagate uniform norm bound via
logarithmic norm, exp or Green phi; physical error must stay inside tube.
Compute separate global certificate or unresolved, not weaken to residual
claim. Reconstruction for reduced z uses actual dictionary Lagrange
nodes 3/5, power coefficients, original source residual; no fitting proof.
Mass native z polynomial raw. WARP TensorPencil nilpotent quotient
predictor for structurally time-affine g_z independent of x,z; evaluate
jet in actual t; original projection/certificate mandatory. Not general
nonlinear quotient. PreparedRhs within projection. Candidate portfolio:
mass/reduced, direct/prepared, dictionary reconstruction, WARP predictor,
adaptive measured selection. Same-contract numerical acceptance on fixed
query grid: all point projections and accepted nodes, raw constraint
diagnostic; independent reference abs error <= 2e-5 and 2e-7 at nominal
rtol 1e-5/1e-7; global certificate evaluated separately at tube 1e-3
and target 1e-3, no false qualification for global certificate.
Use 4 training source families incl Robertson, nonlinear scalar branch,
time-pencil, affine/coupled; separate holdout and rank-loss/ambiguity.
No DAE event/reset/high-index or M3; no default change. Publish research
branch, verified evidence and incremental bundle.
