# Causal indexing amendment

R-JF telemetry for accepted step `j` is available only after that step completes.
Therefore a policy event computed from row `j` can launch an E-K prefix only from
the next committed state (`j+1` in the historical atlas). Same-row E-K audit
labels would constitute look-ahead leakage.

This amendment was made after the six R-JF-only N=128 shards completed but
before any policy threshold was selected and before E-K audit labels were
joined to the new replay. No policy performance output existed at amendment time.
