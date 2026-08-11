# G0 Scope Reset Report — Generic Vectorized/Jacobian-Free Stiff Solver

Date: 2026-08-11

## Executive result

`vectorized-jf-path-controller-v0.17.0-alpha1`의 exact commit에서 새 branch
`generic-vectorized-jf-v1`을 만들고, 이후 개발을 generic solver 목적함수에만 묶는
machine-readable scope lock을 추가했다. 이 node는 numerical method를 변경하지 않는다.

## Base identity

```text
base tag     vectorized-jf-path-controller-v0.17.0-alpha1
base commit  71608a0b160a410dccbe0bf54827e748d01d79fc
base tree    571acb30b087d9a38182626106398594afd7cc11
```

## Preserved generic assets

- protected sequential/adaptive matrix-free RODAS5P
- within-step stage batching
- strict matrix-free common-W and multiple-RHS Krylov backends
- q=1/q=2/q=7 causal-propagation/path-controller references
- adaptive/output observer contract
- same-error global-error/Pareto authority
- failed/speculative/fallback work accounting

## Frozen comparators

BDF2와 Radau IIA3는 comparator로 남지만 source optimization은 금지한다.

```text
bdf.rs
68c681b58686e706ab6890d3be68bb1ca4a9398517e0e755a5c64b2cef746a04

radau.rs
edec0453a5338fc8357a7c026aff894869641b486a3aa3df858353a315bb441b
```

새 regression test와 scope validator가 두 hash를 강제한다.

또한 clone-root에서 sibling offline vendor archive를 사용하도록 `.cargo/config.toml`의 vendored-source path를 `../rust-offline-rodas5p-rs-20260806/vendor`로 고정했다. 이는 수치 코드 변경이 아니라 clean-clone 재현성 수정이다.

## Removed from the core DAG

다음은 generic core의 prerequisite가 아니다.

- RABBIT gate/physics completion
- rec_bianchi accepted-history or physical-owner mutation
- rei_bianchi four-site proof completion
- FLEX-JF-TX source-binding logic
- BDF/Radau production tuning

이들 프로젝트는 generic calibration과 sealed holdout 뒤의 read-only client test로만 사용한다.

## New durable artifacts

- `00_SCOPE_LOCK_KO.md`
- `state/BASELINE_PROVENANCE.json`
- `state/DAG_STATUS.json`
- `handoff/G1_TRANSACTIONAL_Q1_Q2_CONTRACT.json`
- `scripts/validate_scope_lock.py`
- `generic_scope_contracts.rs`

## Validation meaning

G0 validation은 다음만 보증한다.

1. active branch가 exact v0.17 base의 descendant다.
2. BDF/Radau comparator bytes가 변하지 않았다.
3. generic vectorized/JF modules가 존재한다.
4. production Rust source에 물리 client binding이 들어오지 않았다.
5. 기존 numerical tests가 그대로 통과한다.

G0는 speedup, global order, fast-path acceptance 또는 production solver readiness를 보증하지 않는다.

## Next node

`G1_TRANSACTIONAL_Q1_Q2_ENDPOINT_ENGINE`

```text
q1/c2 fast attempt
  -> retain endpoint/state/work
  -> q2 one-correction escalation
  -> protected sequential JF fallback
```

G1에서 물리 client는 실행하지 않으며, BDF/Radau source는 계속 byte-frozen이다.
