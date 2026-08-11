# Generic Vectorized/Jacobian-Free Stiff Solver — G0 Scope Lock

Date: 2026-08-11
Branch: `generic-vectorized-jf-v1`
Base tag: `vectorized-jf-path-controller-v0.17.0-alpha1`
Base commit: `71608a0b160a410dccbe0bf54827e748d01d79fc`

## 1. 단일 목적함수

이 branch의 중심 연구 질문은 다음 하나다.

> RODAS5P 수준의 accepted-step economy를 유지하면서 8-stage sequential critical path를
> batching/vectorization으로 줄이고, 명시적 Jacobian 조립과 fast-path Newton iteration 없이,
> stiff·oscillatory·nonnormal regular ODE에서 protected sequential matrix-free RODAS5P보다
> 동일 external global error 기준으로 더 빠른 범용 solver를 만들 수 있는가?

초기 quantitative gates는 다음과 같다.

- explicit Jacobian builds in primary fast path: `0`
- Newton iterations in primary fast path: `0`
- median critical-path common-W batch depth: `< 8` 선호, `<= 8` 필수
- P95 critical depth: `<= 6` 목표
- protected fallback fraction: `<= 0.10`
- observed global order: `>= 4.8`
- same-error median wall speedup over protected JF RODAS5P: `>= 1.15`
- candidate accepted steps: `<= 1.2 ×` protected RODAS5P on the generic holdout

## 2. G0에서 보존하는 정본

- Rust-only same-kernel numerical baseline
- official RODAS5P coefficients and sign/mass conventions
- protected sequential/adaptive matrix-free RODAS5P
- within-step 8-stage batching and component-major data path
- strict matrix-free common-W operator and multiple-RHS Krylov research backends
- q=1/q=2/q=7 causal-propagation and endpoint-correction reference paths
- adaptive/output observer and same-error Pareto authority
- failed/speculative/fallback work preservation

## 3. 동결 comparator

다음은 비교용으로만 유지하며 이 DAG에서 최적화하지 않는다.

- adaptive BDF2 reference: `crates/rodas5p-integrators/src/bdf.rs`
- adaptive Radau IIA3 reference: `crates/rodas5p-integrators/src/radau.rs`
- SABR5P
- q=7 homotopy correctness oracle

BDF/Radau production work, Nordsieck/LTE/STALD/lagging 확장은 금지한다.

## 4. Core DAG에서 제외

아래는 generic solver의 calibration/holdout 이후에만 read-only client test로 사용할 수 있다.

- RABBIT-specific provenance, collision or gate logic
- `rec_bianchi` accepted-parent/history or physical ledger mutation
- `rei_bianchi` four-site/Krawczyk/event/ledger completion
- FLEX-JF-TX source-binding and transaction adapters

물리 client 결과를 본 뒤 core threshold나 schedule을 재튜닝하면 해당 run은 exploratory로 강등한다.

## 5. G0 완료 조건

- branch가 exact v0.17 tag의 descendant임
- BDF/Radau comparator source hashes가 base와 동일함
- required generic modules가 존재함
- production Rust source에 physical-client binding이 없음
- full Rust tests, strict Clippy, formatting, deterministic scope validator 통과
- 다음 G1 contract가 machine-readable artifact로 존재함

## 6. 다음 node

`G1_TRANSACTIONAL_Q1_Q2_ENDPOINT_ENGINE`

- q=1/c=2 fast attempt
- q=1 endpoint와 work를 보존한 q=2 one-correction escalation
- 실패 시 protected independent matrix-free RODAS5P fallback
- BDF/Radau source 변경 금지
- physical client 실행 금지
