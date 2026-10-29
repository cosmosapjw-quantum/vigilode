# N01 native connector map (source scope)

| Formula / owner | Native file and method | Boundary |
|---|---|---|
| polynomial RHS f(z) | `crates/rodas5p-core/src/n01_mass_action_owner.rs::rhs_at` | source-owned represented callback, exact polynomial separately interval-evaluated |
| J(z)=Df(z) | `n01_mass_action_owner.rs::jacobian_at` | same immutable coefficient owner |
| JVP(z,v) | `n01_mass_action_owner.rs::jvp_at`, `source_owned_jvp_report` | interval error bound for own rounded callback only |
| epoch+clock+state | `advance`, `freeze` | source-owned, previous snapshots stale |
| point-stage original residual, physical error | `try_commit` | outward and two gates before output mutation |
| actual ODE Problem adapter | `crates/rodas5p-integrators/src/n01_mass_action_client.rs` | opt-in Arc<Mutex<same owner>> RHS/Jac/JVP/epoch; new default not installed |
| existing donor | `crates/rodas5p-integrators/src/problems.rs::robertson_problem` | exact real RHS/J law same, NOT byte-identical callback or same provenance |
| actual adaptive solver stage admission | NOT WIRED | user-visible gate remains HOLD |

The native `OdeProblem` API has `eval_rhs_into(t,y,out,counters)`, `linearize(t,y,counters)`, and `linearize_matrix_free(t,y)`; it does NOT have `eval_jacobian` or `eval_jvp_into` methods. The interface mirror reflects these actual method names. Callback exceptions preserve output and are charged by the existing `eval_rhs_into`/`LinearOperator` counted mechanisms. No existing driver, step tolerance, GMRES or work counter implementation was modified.