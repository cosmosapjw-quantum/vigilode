# GCRO-DR scalar generalized-eigen root cause

The unified stiff gate exposed a deterministic panic in `faer 0.24.4` while GCRO-DR
updated a recycle space for a one-dimensional shifted system.  The stack trace entered
`faer::linalg::gevd::qz_to_gevd_real` with an empty dynamic scratch buffer.  The project
wrapper delegated even the 1x1 generalized eigenproblem to the general QZ path, although
that problem has the analytic eigenpair `(a00/b00, [1])`.

The fix belongs in the project generalized-eigen wrapper, not in the scientific gate:
handle dimension zero and one analytically and leave dimensions two and higher on the
existing faer path.  This removes a backend corner-case panic without changing the GCRO-DR
algorithm for nontrivial projected spaces.
