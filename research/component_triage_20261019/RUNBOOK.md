# Component triage 본 campaign 실행 안내서

이 문서는 사용자 머신에서 본 campaign을 실행하는 순서를 적은 것이다. 판정 규칙과
임계값은 `PREREGISTRATION.md`와 `THRESHOLDS.json`에 동결되어 있다. 이 안내서는
그것을 바꾸지 않는다. 각 도구의 입출력 형식은 `tools/triage/README.md`에 있다.

## 0. 준비

| 항목 | 요구 사항 |
|---|---|
| Rust | `rust-toolchain.toml`의 1.94.1 (`cargo --locked`) |
| callgrind | `valgrind` 3.2x 이상 |
| Python | 3.12 이상, venv에 `numpy`, `scipy`, `pytest` (`tools/reference_v2/requirements.txt` 참고) |
| SUNDIALS/Hairer | `libsundials-dev` 6.4.x, `gfortran`, OpenBLAS. Assimulo 3.0 tarball은 SHA-256 고정이며, 처음 한 번만 받으면 이후 offline으로 빌드 가능 |
| Diffsol | 처음 빌드할 때만 crates.io 접근 필요. 이후 `--offline` |
| disk | runs의 상태 sidecar와 callgrind scratch에 수십 GB. scratch는 cell 단위로 지워짐 |

```sh
export N=research/component_triage_20261019 C=ct-$(date +%Y%m%d)-main
export RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 VIGILODE_BUILD_PROFILE=bench
cargo build --locked --profile bench -p rodas5p-cli
mkdir -p bin && cp target/release/rodas5p target/release/rodas5p-allocation-audit bin/   # 실행 중 재빌드와 분리
export VIGILODE_BINARY_SHA256=$(sha256sum bin/rodas5p | cut -d' ' -f1)
(cd tools/benchmarks/diffsol/triage && cargo build --release --locked)       # README 참고
tools/native_stiff/build.sh build/native_stiff                               # CVODE, RADAU5, RODAS
python3 -m venv .venv && .venv/bin/pip install -r tools/reference_v2/requirements.txt pytest
```

## 1. 동결 확인

```sh
bin/rodas5p triage-list > /tmp/list.json && cmp /tmp/list.json $N/TRIAGE_LIST.json
python3 tools/triage/triage_manifest.py --components $N/COMPONENTS.json --corpus $N/CORPUS.json \
    --list $N/TRIAGE_LIST.json --dry-run
.venv/bin/python -m pytest -q tools/test_triage_*.py
```

`cmp`가 다르면 binary가 등록 시점과 다른 것이다. 이 경우 실행하지 말고
`build_manifests.py`로 manifest를 다시 만들어 amendment로 기록한다.

## 2. Parity와 참조해

```sh
.venv/bin/python tools/triage/triage_parity.py --binary bin/rodas5p --corpus $N/CORPUS.json \
    --thresholds $N/THRESHOLDS.json --diffsol-binary tools/benchmarks/diffsol/triage/target/release/diffsol-triage \
    --output $N/PARITY.json
for p in $(python3 -c "import json;print(' '.join(p['id'] for p in json.load(open('$N/CORPUS.json'))['problems']))"); do
  .venv/bin/python tools/triage/triage_references.py --corpus $N/CORPUS.json --binary bin/rodas5p \
      --problem $p --method-timeout 900 --output refs/$p.json
done   # 문제별 파일을 합쳐 $N/REFERENCES.json(.gz)
```

- parity가 1e-13을 넘으면 campaign을 중단한다. 한도를 완화하지 않는다.
- 참조해의 정규화 반경이 0.1·ε보다 큰 cell은 판정 엔진이 `reference_limited`로 제외한다.
- 문제별 반경은 smoke 기록에 있다.

## 3. 실행 (runs, probes)

```sh
python3 tools/triage/triage_runner.py --binary bin/rodas5p --list $N/TRIAGE_LIST.json --corpus $N/CORPUS.json \
    --campaign $C --output $N/RUNS.jsonl --references $N/REFERENCES.json --states-dir states \
    --timeout 600 -j $(( $(nproc) - 1 ))
python3 tools/triage/triage_runner.py --probes --binary bin/rodas5p --list $N/TRIAGE_LIST.json \
    --campaign $C --output $N/PROBES.jsonl --timeout 600
```

- 예상 규모는 `--dry-run` 기준 runs 56,064 cell. 중단돼도 같은 명령을 다시 실행하면 이어서 돈다.
- n > 64 행은 상태를 `states/`의 sidecar에 저장하고, 행에는 SHA-256과 inline 정확도만 남는다.
- smoke에서 알려진 동작:
  - `mf-gcrodr-*` arm은 pollution에서 timeout이 남. timeout은 C1 실패가 아니라 해당 cell 도달 불가로 처리됨.
  - NaN Jacobian probe에서 BDF/Radau와 일부 MF stage-target arm은 시도 예산을 모두 쓴 뒤 실패함.

## 4. 비용 (callgrind, allocation)

```sh
for p in $(python3 -c "import json;print(' '.join(p['id'] for p in json.load(open('$N/CORPUS.json'))['problems'] if p['dimension']<=2048))"); do
  python3 tools/triage/triage_profile.py --binary bin/rodas5p --assert-bench-profile --list $N/TRIAGE_LIST.json \
      --corpus $N/CORPUS.json --campaign $C --problem $p --output prof/$p.jsonl --scratch /tmp/cg-$p &
done; wait   # 문제별 파일을 $N/PROFILE.jsonl로 합침 (같은 cell은 Ir 일치 확인)
python3 tools/triage/triage_alloc.py --binary bin/rodas5p-allocation-audit --list $N/TRIAGE_LIST.json \
    --corpus $N/CORPUS.json --campaign $C --output $N/ALLOCATIONS.jsonl --warmups 2 --windows 3 --timeout 900
```

- profile 부분집합은 rtol {1e-4, 1e-6, 1e-8}, `uniform101`, h0 1, n ≤ 2048, 총 3,393 cell.
- Ir은 결정적이므로 머신 부하와 무관하다. 병렬도는 core 수에 맞춘다.
- P-tier 큰 문제에 `--cachegrind`를 주면 LL miss도 기록된다. TIMING_CONFLICT 진단에 쓰임.

## 5. 외부 arm

```sh
.venv/bin/python tools/triage/triage_external.py --corpus $N/CORPUS.json --campaign $C --output $N/EXTERNAL.jsonl \
    --native-driver build/native_stiff --diffsol-binary tools/benchmarks/diffsol/triage/target/release/diffsol-triage \
    --references $N/REFERENCES.json --states-dir states
```

외부 arm의 결과는 OPTIMIZE(b) 격차 판정과 참조 교차검증에만 쓰인다. 내부 rival로는 쓰지 않는다.
Python wall time은 기록하지 않는다.

## 6. Wall time (진단용, 선택)

```sh
sudo tools/triage/host_prep.sh --apply --output $N/HOST.json
```

- `paired-timing-campaign`으로 ≥ 6 session, ABBA ≥ 30 pair를 측정한다.
- 이 설계에 대한 `session-median-coverage-study`가 통과하고 독립 리뷰가 끝나기 전까지는 판정에 쓰지 않는다.
- 그 전까지 wall time은 `TIMING_CONFLICT` 표시에만 쓰인다.

## 7. 판정, 보고서, 기록

```sh
python3 tools/triage/triage_check.py --campaign $C --runs $N/RUNS.jsonl --runs $N/EXTERNAL.jsonl \
    --references $N/REFERENCES.json --profile $N/PROFILE.jsonl --allocations $N/ALLOCATIONS.jsonl \
    --probes $N/PROBES.jsonl --states-dir states --components $N/COMPONENTS.json --corpus $N/CORPUS.json \
    --thresholds $N/THRESHOLDS.json --list $N/TRIAGE_LIST.json \
    --results $N/RESULTS.json --classification $N/CLASSIFICATION.json
python3 tools/triage/triage_report.py --results $N/RESULTS.json --classification $N/CLASSIFICATION.json \
    --output $N/REPORT_KO.md
python3 tools/check-research-node.py $N
```

- `CLASSIFICATION.json`이 component마다 PROMOTE / SPECIALIZE / OPTIMIZE / PRUNE 중 하나를 준다. 함께 `provisional`, `blocked_by`, 판정 경로가 붙는다.
- 독립 판정 리뷰(decision-gate reviewer) 후 `research/LEDGER.jsonl`에 기록한다.
- 100 MB를 넘는 파일은 `gzip -n` 사본으로 commit한다. 도구는 `.gz`를 직접 읽는다.
- `states/` sidecar는 commit하지 않고 별도로 보관한다 (Zenodo 등). SHA-256은 행에 남는다.
