"""Generate report tables/plots only from the completed, qualified raw packet."""
from pathlib import Path
import json
import statistics
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parent
DATA = ROOT.parent / "evidence/benchmarks/SUMMARY.json"
if not DATA.exists():
    DATA = DATA.with_name("SUMMARY_COMPACT.json")


def sci(x):
    mant, exp = f"{x:.3e}".split("e")
    return rf"${mant}\times10^{{{int(exp)}}}$"


def fmt_range(row):
    return f"{row['min_block_ratio']:.3f}--{row['max_block_ratio']:.3f}"


def main():
    data = json.loads(DATA.read_text())
    assert data["status"] == "COMPLETE"
    inactive_adapter = [r for r in data["adapter"] if not r["qualified"]]
    data["adapter"] = [r for r in data["adapter"] if r["qualified"]]
    assert all(r["qualified"] for category in ["lu", "dense", "adapter"] for r in data[category])
    confirmation = json.loads((ROOT.parent/"evidence/confirmation/SUMMARY.json").read_text())
    assert confirmation["status"] == "COMPLETE"
    profile = json.loads((ROOT.parent/"evidence/instruction_profile/SUMMARY.json").read_text())
    assert profile["status"] == "COMPLETE"
    plt.rcParams.update({"font.family": "DejaVu Sans", "font.size": 9,
                         "axes.spines.top": False, "axes.spines.right": False,
                         "axes.titlesize": 10, "pdf.fonttype": 42, "ps.fonttype": 42})
    fig, axs = plt.subplots(1, 3, figsize=(7.1, 2.65), constrained_layout=True)
    for ax, category, title in zip(axs, ["lu", "dense", "adapter"],
                                  ["Faer solve kernel", "Full dense trajectory", "Full adapter solve"]):
        rows = [r for r in confirmation["rows"] if r["kind"] == category]
        x = np.arange(len(rows))
        ax.axhline(1, color="#7b8188", lw=.9, ls="--")
        for metric, offset, color, label in [("internal_wall",-.07,"#245d80","Internal wall"),
                                              ("process_cpu",.07,"#c98524","Process CPU")]:
            ratios = np.array([r[metric]["median_ratio"] for r in rows])
            lo = np.array([r[metric]["min_ratio"] for r in rows])
            hi = np.array([r[metric]["max_ratio"] for r in rows])
            ax.errorbar(x+offset, ratios, yerr=[ratios-lo, hi-ratios], color=color,
                        marker="o", ms=4, capsize=3, lw=1.2,label=label)
        ax.set_xticks(x, [str(r["n"]) for r in rows])
        ax.set_xlabel("State dimension n")
        ax.set_title(title, loc="left", fontweight="bold")
        ax.set_ylim(.5, 1.55)
        ax.set_xlim(-.45, len(rows)-.55)
        ax.grid(axis="y", color="#e7eaee", lw=.6)
    axs[0].set_ylabel("Baseline time / candidate time")
    axs[0].legend(frameon=False,fontsize=7,loc="upper left")
    fig.savefig(ROOT/"figures/performance.pdf")
    fig.savefig(ROOT/"figures/performance.png", dpi=180)
    plt.close(fig)

    theory = json.loads((ROOT.parent/"theory/SHIFTED_REUSE_RESULTS.json").read_text())
    fig, axs = plt.subplots(1, 2, figsize=(7.1, 2.6), constrained_layout=True)
    ratios = np.sort(np.append(np.geomspace(.08, 12, 300), 1.0))
    axs[0].plot(ratios, abs(ratios-1)/(ratios+1), color="#245d80", lw=1.5,
                label=r"Uniform $q_*=|r-1|/(r+1)$")
    for family, marker, color in [("diagonal", "o", "#245d80"),
                                  ("damped_nonnormal", "s", "#d28a24"),
                                  ("skew", "^", "#53745b")]:
        rows = [r for r in theory["rows"] if r["family"] == family]
        rs = [r["ratio"] for r in rows]
        axs[0].scatter(rs, [r["operator_norm"] for r in rows], s=14, marker=marker,
                       facecolor="none", edgecolor=color, linewidth=.7)
        axs[1].plot(rs, [r["relaxed_iterations"] for r in rows], marker=marker,
                    ms=3, color=color, lw=1, label=family.replace("_", " "))
        for r in rows:
            if not r["relaxed_converged"]:
                axs[1].scatter([r["ratio"]], [r["relaxed_iterations"]], s=50,
                               marker="x", color="#b13435", lw=1.1, zorder=5)
    axs[0].set_xscale("log"); axs[1].set_xscale("log"); axs[1].set_yscale("log")
    axs[0].set_ylabel("Contraction factor")
    axs[1].set_ylabel("Relaxed iteration count")
    axs[0].set_title("Exact-arithmetic contraction\nConditional on a dissipative metric", loc="left", fontweight="bold",fontsize=9)
    axs[1].set_title("Finite-precision iteration\n25/27 reach the residual target", loc="left", fontweight="bold",fontsize=9)
    for ax in axs:
        ax.set_xlabel(r"Shift ratio $r=a/a_0$")
        ax.grid(color="#e7eaee", lw=.6)
        ax.legend(frameon=False, fontsize=7, loc="best")
    fig.savefig(ROOT/"figures/theory.pdf")
    fig.savefig(ROOT/"figures/theory.png", dpi=180)
    plt.close(fig)

    dense_ratios = ", ".join(f"{r['internal_wall']['median_ratio']:.3f}" for r in confirmation["rows"] if r["kind"]=="dense")
    ad_ratios = ", ".join(f"{r['internal_wall']['median_ratio']:.3f}" for r in confirmation["rows"] if r["kind"]=="adapter")
    summary = rf"""\paragraph{{Measured outcome.}}
The warmed retained-LU solve adds zero allocations in the seven-size contract
cohort. The adapter removes 17 of 2119 and 50 of 2655 charged work units in the
8- and 16-dimensional fixtures, respectively, with exactly equal accepted
trajectories. All three dense fixtures likewise preserve every common observation
value and native counter. A separate Callgrind comparison measures instruction
reductions of 13.196\% for the 32-dimensional LU kernel, 2.605\% for the full
65-dimensional dense trajectory, and 1.731\% for the 16-dimensional adapter.
These establish implementation-work reductions in the profiled cohorts.
Wall-clock acceleration is \emph{{not established}}:
after an initial short-batch pass, a single longer confirmation gives paired
dense baseline/candidate ratios {dense_ratios} at dimensions 65 and 192 and
an adapter ratio {ad_ratios} at dimension 16, but every observed block range
crosses unity. Process CPU ratios are also variable. Ratios above one favor
the candidate; neither attractive medians nor lower work counts are promoted
to reliable time-speedup claims. The shifted-factor research candidate is
excluded from production timing claims.
"""
    (ROOT/"results_summary.tex").write_text(summary)
    parts = [r"""\subsection{Measured implementation economy and unresolved timing}
\paragraph{Instruction-count confirmation.}
After native timing remained inconclusive, a bounded Callgrind 3.22.0 pass
compared the unchanged binaries on three specified cells. Counts at two batch
lengths subtract fixed command setup and diagnostic work. The resulting
per-repetition instruction slopes decrease in all three cases
(Table~\ref{tab:instructions}). Every profiled output retains the analytic
accuracy cap; baseline/candidate dense observations and counters also agree
within the profiler. Callgrind emulates instructions and may expose a different
SIMD feature set from native execution. Its instruction reduction is an actual
measured computational-work improvement, not a claim about native IPC or seconds.
The small fractional counts arise from differencing batch totals; small
serialization/control-path variation can remain in these estimates.

\begin{table}[t]
\caption{Measured Callgrind instruction slopes per solve/trajectory. Fixed
command costs are removed using two repetition lengths. All three changes
retain the same qualified numerical output in their profiled cohort.}
\label{tab:instructions}
\begin{ruledtabular}
\begin{tabular}{lrrr}
Scope & Baseline Ir & Candidate Ir & Reduction\\
\hline
"""]
    for r in profile["rows"]:
        parts.append(f"{r['kind']} $n={r['n']}$ & {r['baseline_Ir_per_rep']:,.0f} & "
                     f"{r['candidate_Ir_per_rep']:,.0f} & {100*r['reduction_fraction']:.3f}\\% \\\\\n")
    parts.append(r"""\end{tabular}
\end{ruledtabular}
\end{table}

\paragraph{Wall and process CPU timing.}
The first pass took 27.57 seconds. All eight active timing cohorts had paired
wall-time ranges spanning unity. The one-dimensional adapter case had correct,
identical trajectories but did not activate the new stop: its work remained
1644 units and its RHS count 1030 in both arms. It therefore received no
performance timing; this was not an accuracy failure.

Before further measurements, a single confirmation protocol selected four
diagnostic cells, increased baseline batches to approximately one second, and
added process user-plus-system CPU accounting. The confirmation took 99.17
seconds. No third timing pass was run. All four wall-time and process-CPU
block ranges still cross unity. In particular, the initially attractive
dimension-192 dense median of 1.252 reverses to 0.923 in the confirmation.
Reporting a reliable 25\% gain from the first pass would therefore be false.
The present environment supports the exact-work and allocation conclusions,
but does not resolve a stable wall-time benefit or regression for these paths.

\begin{table*}[t]
\caption{Long-batch confirmation. Each ratio is a median of five paired block
ratios. Ranges are observed minima/maxima, not confidence intervals. Process
CPU includes setup, diagnostic warmup and serialization; the adapter command
includes two initial diagnostic solves. Every range spans unity.}
\label{tab:confirmation}
\begin{ruledtabular}
\begin{tabular}{llrrrr}
Scope & $n$ & Wall ratio & Wall range & CPU ratio & CPU range\\
\hline
""")
    for r in confirmation["rows"]:
        w,c=r["internal_wall"],r["process_cpu"]
        parts.append(f"{r['kind']} & {r['n']} & {w['median_ratio']:.3f} & {w['min_ratio']:.3f}--{w['max_ratio']:.3f} & "
                     f"{c['median_ratio']:.3f} & {c['min_ratio']:.3f}--{c['max_ratio']:.3f} \\\\\n")
    parts.append(r"""\end{tabular}
\end{ruledtabular}
\end{table*}
\begin{figure*}[t]
\includegraphics[width=\textwidth]{figures/performance.pdf}
\caption{Long-batch confirmation baseline/candidate time ratios. Points are medians of five
paired ABBA/BAAB block ratios; bars span the observed block minimum and maximum,
not a statistical confidence interval. Internal kernel wall time excludes
factorization. Full-solve wall intervals include setup, integration, output,
and counters. Process CPU additionally includes command warmup and serialization.
The adapter panel is a same-binary option comparison. All ranges cross unity.}
\label{fig:performance}
\end{figure*}
\begin{table*}[t]
\caption{Retained initial short-batch diagnostic timing. Values are median arm times;
speed ratios are medians of paired block ratios and need not equal the ratio
of the two displayed marginal medians. Every row passes its declared accuracy contract,
but none establishes a stable wall-time benefit.}
\label{tab:timing}
\begin{ruledtabular}
\begin{tabular}{llrrrrl}
Scope & $n$ & Baseline & Candidate & Ratio & Unit & Block-ratio range\\
\hline
""")
    for category, label, scale, unit in [("lu", "LU solve", 1e6, r"$\mu$s"),
                                        ("dense", "Dense trajectory", 1e3, "ms"),
                                        ("adapter", "Adapter solve", 1e6, r"$\mu$s")]:
        for r in data[category]:
            parts.append(f"{label} & {r['n']} & {r['baseline_median_seconds']*scale:.3f} & "
                         f"{r['candidate_median_seconds']*scale:.3f} & "
                         f"{r['baseline_over_candidate']:.3f} & {unit} & {fmt_range(r)} \\\\\n")
    parts.append(r"""\end{tabular}
\end{ruledtabular}
\end{table*}
\begin{table}[b]
\caption{Dense trajectory accuracy on all 33 common times and all components.
All states, observation times, native counters and attempt counts are identical
in the baseline and candidate, with the same $10^{-7}$ error cap.}
\label{tab:dense-accuracy}
\begin{ruledtabular}
\begin{tabular}{lrl}
$n$ & Attempts & Maximum component error\\
\hline
""")
    for r in data["dense"]:
        parts.append(f"{r['n']} & {r['signature']['attempts']} & {sci(r['max_grid_error'])} \\\\\n")
    parts.append(r"""\end{tabular}
\end{ruledtabular}
\end{table}
\begin{table*}[t]
\caption{Adapter work and accuracy. All accepted trajectory values are equal
between feature-off and feature-on runs. Every returned component, including
the common requested windows, meets the $10^{-5}$ cap. Work includes discarded
comparators; these reductions are not themselves time ratios.}
\label{tab:adapter-work}
\begin{ruledtabular}
\begin{tabular}{lrrrrrl}
$n$ & RHS off/on & Work off & Work on & Work reduction & Prefix wins & Maximum error\\
\hline
""")
    for r in data["adapter"]:
        a, b = r["baseline"], r["optimized"]
        parts.append(f"{r['n']} & {a['rhs']}/{b['rhs']} & {a['work']:.0f} & {b['work']:.0f} & "
                     f"{100*(1-b['work']/a['work']):.2f}\\% & {b['lower_bound_returns']} & "
                     f"{sci(b['trajectory_max_error'])} \\\\\n")
    parts.append(r"""\end{tabular}
\end{ruledtabular}
\end{table*}
The sentinel-on control has identical complete trajectories and counters with
the new option off/on, as expected because the option does not change that path.
The targeted native contracts separately verify encountered-fatal priority,
conservative arithmetic near the exact-integer limit, and incomplete-evidence
semantics. The full raw records include rejected or failed build attempts as
well as successful validation. No broad historical test suite is relabeled
as a new execution.

\subsection{Negative and conditional theory results}
\begin{figure*}[t]
\includegraphics[width=\textwidth]{figures/theory.pdf}
\caption{Shifted-system research experiment. Left: the exact-arithmetic
uniform contraction bound and finite computed operator norms. Right: iteration
counts of the relaxed candidate. Red crosses mark two runs that reached the
512-iteration limit without attaining the unchanged $10^{-10}$ residual
threshold. Numeric norm checks are diagnostic checks, not interval certificates.}
\label{fig:theory}
\end{figure*}
\begin{table*}[t]
\caption{Selected shifted-system cells exposing both benefit and cost.
``Fail'' preserves the original stopping failure. Iteration counts are not
wall times and should not be compared as if their kernels had equal cost.}
\label{tab:theory-cells}
\begin{ruledtabular}
\begin{tabular}{llrrll}
Family & $a/a_0$ & Relaxed iterations & Unrelaxed iterations & Relaxed target & Unrelaxed target\\
\hline
""")
    selected = [("diagonal", .1), ("diagonal", .25), ("diagonal", 4.0), ("diagonal", 10.0),
                ("damped_nonnormal", 2.0), ("damped_nonnormal", 10.0), ("skew", 1.0), ("skew", 4.0)]
    for f, ratio in selected:
        r = next(r for r in theory["rows"] if r["family"] == f and r["ratio"] == ratio)
        parts.append(f"{f.replace('_', ' ')} & {ratio:g} & {r['relaxed_iterations']} & {r['plain_iterations']} & "
                     f"{'Pass' if r['relaxed_converged'] else 'Fail'} & "
                     f"{'Pass' if r['plain_converged'] else 'Fail'} \\\\\n")
    parts.append(r"""\end{tabular}
\end{ruledtabular}
\end{table*}
The two failed relaxed residuals are approximately $4.80\times10^{-10}$ and
$4.67\times10^{-10}$, while their actual forward errors are small. This is
consistent with residual evaluation at a floating floor; it does not justify
changing the requested target after observing the failures. The new rule cures
two diagonal unrelaxed divergences but generally increases iterations elsewhere.
Consequently the theorem supplies a conditional convergence option, not an
unconditional performance replacement.
""")
    (ROOT/"results_detail.tex").write_text("".join(parts))
    print(json.dumps({"status": "GENERATED_FROM_COMPLETED_RAW_DATA", "dense_ratios": dense_ratios,
                      "adapter_ratios": ad_ratios, "figures": ["performance", "theory"]}))


if __name__ == "__main__":
    main()
