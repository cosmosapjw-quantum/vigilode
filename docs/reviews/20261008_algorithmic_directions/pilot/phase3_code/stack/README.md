# Probe B1: the integrated Tier-1 stack in closed loop

These are exploratory replica results, meant as pilots for future preregistered Rust nodes. They are not ledger authority. The worktree `/home/user/wt-speed` (a49f7e4) was only read, and no Rust was built.

## Files

- **`stack.py`** is the closed-loop matrix-free U-form RODAS5P replica. It derives from probe A1's `rep.py` (copied as `rep_a1.py`, unchanged) and adds:
  - the PRED+cap and I725 controllers from `probe/ctrl/core.py`;
  - stacked WRMS targets (`StackTarget`);
  - maxit and the stagnation guard, with an uncounted shadow continuation that classifies false aborts;
  - finite-difference JVPs with gap diagnostics;
  - the controller-noise diagnostic;
  - full admitted-cost counters and the flop model.
- **`coupled_target.py`** is A1's rule, copied unchanged.
- **`tprobs.py`** holds A1's problem transcriptions. Bruss-300 is `tprobs.bruss(300)`.
- **`run.py`** is the arm registry and ladder runner. It writes `res/<problem>.jsonl`.
- **`ana.py`** and **`tables.py`** produce the error metrics, gates, matched accuracy (regression frontier and cheapest-run rule) and the attribution, robustness and FD tables. `tables.py` writes `tables.txt`. `report_tables.py` produces the markdown tables.
- **`fid.py`** checks fidelity against the SPD07 BASE.json Rust counters. Output is `res/fidelity.json` and `logs/fid.log`.
- **`pred_noise.py`** runs the PRED × target noise diagnostic. Output is `res/noise.jsonl`.
- **`ladders_b1.py`** runs the fixed-step contract ladders for the stacked targets. Output is `res/ladders_b1.jsonl`.
- **`fdfloor.py`** measures the FD-JVP attainable-residual floor on Bruss-50.
- **References**:
  - `ref_{hires,robertson,vdp,bruss50}.json` are A1's 80-bit RODAS5P runs at rtol 1e-15.
  - `ref_bruss{50,160,300}_radau.json` are scipy Radau runs at rtol = atol = 1e-12 and 1e-13; 1e-13 is used.
  - PR and quad-4 use their exact solutions.

## Arms

The arms are cumulative.

| arm | definition |
|---|---|
| A0 | SPD07 base: full GMRES(40) cycles, L2 target max(g·1e-14, 1e-10·‖b‖), duplicate final diagnostic residual |
| A1 | A0 without the duplicate residual (derived from A0 counters; the trajectory is identical, verified) |
| A2 | in-cycle projected stop at the same L2 target, plus one true residual |
| A3 | A2 with probe A1's coupled WRMS target (Θ = 0.2, EPUS, order factor, U8 cap, 16ε guard, 1024ε stall rule) |
| A4a / A4b | A3 with INO's per-step budget as a floor: θ_n = max(Θ·h/T, θ)·min(1, ê/0.5)^1.2, with θ = 0.001 / 0.002 |
| A5a / A5b | A4 with the PRED+cap controller |
| A6 | A5a with maxit 2000 and the stagnation guard (q ≥ 0.98 or predicted overrun) |
| S | **recommended**: A3 + PRED+cap + maxit 2000 + guard (A6 without INO) |

Rivals:

- R3: L2 coupling, the cheapest rival for the stage target.
- R5 / R3U: I controller with a uniform safety factor of 0.725, the set-point rival to PRED.
- Rbig: maxit 2000, the budget rival.
- SU: S with I725 in place of PRED.

Controls:

- C4a / C4b: INO alone.
- C0P / C2P / C3P: PRED added to A0 / A2 / A3.
- C0G / C3G: the guard added to A0 / A3.
- C6ng: A6 without the guard.
- LU / LUP: exact solves under the I / PRED+cap controller.
- X02I / X02P and XrI / XrP: deliberately loose targets (positive controls for noise).
- *s5 / *s4: h0 seeds of 1e-5 and 1e-4.
- FD arms:
  - A0f, A2f, A5af: as calibrated.
  - A3fg, A5afg: with the FD-aware guard.
  - A5ag: the same guard with exact JVPs.
  - A0f8s, A2f8s: the production form at rtol_lin 1e-8 plus an FD stall backstop.

## Reproduce

From this directory:

```
OPENBLAS_NUM_THREADS=1 python3 fid.py
./runall.sh S
./runall.sh B
./runall.sh B160
./runall.sh B300
```

Then run the FD, X, S/SU and seed ladders listed in `queue2.sh` and `queue3b.sh`, run `pred_noise.py` and `ladders_b1.py`, and finally:

```
python3 tables.py > tables.txt
```
