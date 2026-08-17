# Next node — attempt-level causal event/prefix replay

1. Extend the R-JF research trace to include every attempted step (accepted and rejected),
   the controller-proposed `h`, error/result, and full work counters.
2. Event evidence may update only after an accepted R step.
3. If an event fires, launch the research prefix from the next committed state
   using the *first controller-proposed h* before the next R trial.
4. Retain prefix cost if that R trial later rejects; do not remap it to the next
   accepted row.
5. Keep committed trajectory R-JF-only and prove exact identity to baseline.
6. Separate G4-S4 Arnoldi acquisition prefix from full pexprb54s4 `(U2,D2)` level-1.
7. Run N=128/N=512 development first. Do not open N=2048 until the attempt-level
   policy is frozen and all speculative work is accounted.
