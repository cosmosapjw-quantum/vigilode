# V1p: portable arithmetic enclosure for the fixed full33 physical norm

Start with [REPORT_KO.md](REPORT_KO.md), [THEORY.md](THEORY.md), [DAG.json](DAG.json) and the copyable [HANDOFF_KO.md](HANDOFF_KO.md).

The standalone Rust kernel is [source/portable_norm.rs](source/portable_norm.rs). It uses exact fixed-size signed accumulation and exponent-separated outward squared-norm intervals. This is not a native RODAS hook or global ODE error certificate. Caller-owned positive scales, fixed PR79 +/-1 lift, IEEE gradual underflow and no fast-math/FTZ remain assumptions. No runtime FP-mode guard is implemented.

Original S2 evidence:12 Rust tests,343 exact-sum plus398 physical-norm oracle records,6 retained Indeterminate cases,128 kernel calls with zero measured heap requests,36 diagnostic timing cells. [ORACLE_SUMMARY.json](ORACLE_SUMMARY.json), [S2_DELIVERY.json](S2_DELIVERY.json).

Recovery rechecks source/file identities and stored outputs without compiling or rerunning a solver. See [READBACK.json](READBACK.json) and [REPRODUCE.md](REPRODUCE.md). Full raw inputs/outputs, failed RED, old budgets and review are in the [S3 companion on Drive](https://drive.google.com/file/d/1w-kqXbIABnXjA0KaOEmpZtQjH6HDdmrY/view?usp=drivesdk), with Dropbox identity in [FINAL_DELIVERY.json](FINAL_DELIVERY.json).

S2 source publication: `909ed1577e49903443c4d6d94fb89bd6d0a01ea1`. Exact executed source identity is its matching file-blob set, not a retroactive execution-time Git commit. Existing production/default sources, local uncommitted work and NCP results are untouched. Scope complete, broad production and native integration held. No merge.
