# Matrix-free W research claim ledger — 2026-08-30

RESULT_VALIDITY: 18 new contracts and 13 small same-trial independent explicit
original-action checks, plus three stencil sizes and one analytic diagonal
preconditioner control. All are diagnostic/non-authoritative. They do not
replay or reclassify the earlier 13 trial states or historical54 campaign.
PROVENANCE_VALIDITY: exact c5 parent, protected source/evidence unchanged;
current Git HEAD/tree supplied by the live Draft PR and publication receipt.
PACKAGING_VALIDITY: source/tests executed locally; final remote CI is late-bound.

Allowed: the added function applies W through analytic JVP, performs eight
successive solves with one preconditioner setup/workspace, and separately
re-evaluates the unpreconditioned residual of each returned row. The tested
candidate makes no Jacobian builds or direct factorizations. Failure records
keep already-returned reports, verified rows and available work. The 128-state
analytic diagonal PC control uses one setup and eight Arnoldi iterations.

Not claimed: zero allocations, peak-memory measurement, universally scalable
nonidentity mass or arbitrary callback internals, exhaustive inherited work on
failure, access to the failed kernel's unfinished iterate, cross-step cache
reuse, nonlinear/output certification, order/accuracy of an activated solver,
production integration, timing/ranking/speedup, BDF comparison, holdout/freeze,
PM-7/K0 closure, scientific admission, PR merge, tag or release.

Known boundaries are explicit data, not hidden successes:
- inherited_work_complete=false on failed preparation/nonlinear callback paths;
- failed_kernel_iterate_available=false on kernel Err;
- kernel counters on Err may omit unfinished-cycle iteration counts, while new
  W/JVP/PC attempted/completed application counts remain observable;
- factory costs require caller reporting, and context-construction costs are
  outside the function receipt;
- existing nonidentity mass is a dense input and may be cloned by preparation;
- only retained Krylov workspace capacity is measured.

Self-check and two actual source mutants were performed. No fresh independent
review occurred here. The next local host review is limited to this new delta;
it must not be described as a review already completed by this delivery.
