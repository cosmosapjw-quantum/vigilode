# Probe A1: an outer-coupled stage target for the matrix-free U form (exploratory)

These are exploratory replica results for a future preregistered Rust node. They are not ledger authority.

`coupled_target.py` implements the rule. The rest of the code lives in the same directory:

- `rep.py` is the replica. With MGS2 it matches the SPD07 Rust counters exactly in 12 of 14 cells. The two Robertson cells are off by 0.1-0.3% in JVPs.
- `arms.py` and `ladder.py` run the adaptive ladders.
- `ladders.py` runs the fixed-step contract ladders.
- `ref.py` computes the 80-bit references.
- `final.py` and `ana.py` produce the tables.

Results are in `tables.txt`, `abs_table.txt` and `res/*.jsonl`.

## Rule

For each stage i = 1..8 of an attempt (t_n, y_n, h_n), measure the true residual in the outer error test's WRMS metric. The weights are D = 1/(atol + rtol*|y_n|), so the residual is in tolerance units:

    ||b_i - W U_i||_w  <=  eps_i

    theta_n = Theta * (h_n / T) * min(1, e_hat_n / e_ref)^p
    eps_i   = theta_n / (8 * max(tau_y,i, tau_e,i))        (i = 1..8)
    eps_8   = min(eps_8, kappa8 * e_sat)

Krylov realisation (scaled form):

- Run GMRES on D W D^-1 with right-hand side D b. Its Givens-projected residual is sqrt(n) times the WRMS residual.
- Stop inside the cycle at `max(eps_i*sqrt(n), RHO_FL*||D b||_2)`, then confirm with one true residual.
- Stall rule: if a restart cycle cuts the true residual by less than 4x, accept when `||r|| <= RHO_STALL*(||Db|| + ||DU|| + ||D(U - WU)||)`.

There is no L2 absolute floor and no relative 1e-10 target.

## Constants

| symbol | value | meaning |
|---|---|---|
| Theta | 0.2 | Global contamination budget in tolerance units. It is spread over the span in proportion to h ("error per unit step", EPUS). Failure starts at Theta = 2 (HIRES 5.7x, quad-4 2.8x base); Theta = 0.5 still passes. |
| T | t_end - t_0 | Integration span. The driver knows it. |
| e_hat_n | err of the last accepted step | Before the first acceptance, e0 = 1e-6 is used. |
| e_ref, p | 0.5, 6/5 | Order factor. It makes the per-step budget scale as h^6 at fixed rtol. With e_ref = 0.1 the endpoint gate passes, but contamination reaches 0.85x base error on HIRES at 1e-3, so the robust gate fails. |
| tau_y,i | 1.729, 0.511, 6.246, 6.081, 3.820, 4.771, 1.011, 1.000 | sup over Re z <= 0 of the residual-to-y_new transfer. |
| tau_e,i | 1.393, 2.057, 2.266, 2.160, 1.510, 1.792, 2.011, 1.000 | sup over Re z <= 0 of the residual-to-U8 (err) transfer. |
| kappa8 * e_sat | 0.1 * (0.9/5)^5 = 1.89e-5 | U8 cap. Removing it gives HIRES 1.6-1.8x base. |
| RHO_FL | 16 eps = 3.6e-15 | Attainable-accuracy guard relative to `||D b||_2`. |
| RHO_STALL | 1024 eps = 2.3e-13 | Stall acceptance. Without it vdP has 4-11 linear failures per run and costs up to 1.7x. |

The transfer functions are T_y = b_code^T L^-1 and T_e = (L^-1)_{8,:}, with L(z) = (1 - g z) I - g (z A + C) (`hyp/tau_table.npz`, row a = 0).

## Why EPUS rather than a per-step or relative target

To first order, with J frozen and von Neumann's bound holding (μ_D(hJ) <= 0), each step's contamination is bounded by `||dy_n||_w <= theta_n`. Summed over steps, the global contamination is at most M * sum(theta_n) = M * Theta. That bound does not depend on rtol or on N, so the rule is tolerance proportional.

Measured contamination |y_arm - y_LU| in rtol units on Bruss-50 from rtol 1e-3 to 1e-11:

- Per-step theta = 1e-2 grows from 0.003 to 0.075.
- The EPUS rule with Theta = 0.2 stays flat between 0.004 and 0.007.
- The SPD07 relative target (proj) grows from 0 to 1.82.

## API

- `stage_wrms_targets(h, span, err_hat)` returns the 8 targets.
- `Target().stage_eps(...)` and `Target().l2_threshold(...)` give the per-stage targets and the scaled-GMRES threshold.
- `scaled_gmres_threshold(eps_i, Db)` returns the threshold directly.
- `Target.stall_accept(...)` applies the stall rule.
- `default_target()` returns the recommended configuration.
