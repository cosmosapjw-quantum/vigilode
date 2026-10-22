# Imported research record — retrospective declaration

The original 16 files are imported byte-for-byte from remote source commit f9da4aae865623d71bff20c6157318cbaf19ba6c. They did not contain a PREREGISTRATION.md. This added record is explicitly retrospective and must not be represented as prospective registration. No original result is changed.

Scope: audit the affine 30-to-33 error-norm construction and replay its existing C99 verifier once with `python3 research/lifted_error_norm_20261022/code/verify_norm.py --output <outside-checkout>/AFFINE_NORM.json`. The replay found 10 finite oracle cases and 5 negative controls passing; it does not execute the VigilODE integrator, establish temporal accuracy, or measure NCP performance. Source identity and replay are recorded under `research/production_review_20261022/`.

No confirmatory production claim is admitted from this retrospective import. The first implementation step remains connecting the lifted norm to actual step acceptance with a full-33 trajectory reference.
