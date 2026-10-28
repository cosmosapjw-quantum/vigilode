# 로컬 benchmark/profile 요약

범위는 `suites-a1/local-benchmarks`와 `suites-a1/local-profiles`의 완료 원자료입니다. 새 실행은 하지 않았고 저장소도 수정하지 않았습니다. 원본 파일별 SHA-256은 `BENCHMARK_SUMMARY.json`의 `file_hashes`에 기록했습니다. 실행 checkout은 `a1a459f2be186d9f67ff86bf38c19ae24b769768`, 호스트는 `cosmosapjw` / Linux x86_64, 기본 Rayon/OpenMP/OpenBLAS 스레드는 1입니다.

## 실행 및 결과

- release 빌드 2개(command 00–01): 모두 exit 0, offline/locked 빌드였습니다. Cargo target은 checkout 밖 `/home/cosmosapjw/target-suite-20261028T170155Z`입니다.
- cubic 비교(command 02): 128회 반복, 512 step, 10개 paired block. generic 대비 scalar 중앙 비율은 1.244, algebraic 1.134, rational 1.278입니다. 그러나 algebraic/rational checksum은 generic/scalar와 달라 bitwise 동치가 아닙니다. 이는 특정 cubic workload의 비교입니다.
- structural 경로(command 03–06): 차원 1, 64, 2048 및 2048/2-thread 각각 별도 실행, 64 step, 5 paired block입니다. 기록된 공통-grid 오차는 최대 `2.3976e-10`으로 cap `1e-6` 이내였고 원공간 잔차 cap은 `1e-9`였습니다. 각 후보의 모든 grid bits가 PR91과 일치했습니다. 2048차원/1-thread에서 fused의 paired 중앙 speed ratio는 약 1.15, enclosed는 약 2.27입니다. 2-thread 데이터는 별도 cohort이며 1-thread와 합산하거나 일반 스케일링으로 해석하지 않습니다. 단일 실행군의 결과이지 production 속도 승격이 아닙니다.
- adapter performance(command 07): stiff-cubic 64회 반복의 관측 PR91 후보 median ratio 0.975 (범위 0.941–1.005), 즉 이득 근거가 없습니다. reentrant-nonlinear에서 observed-strict-sentinel 대 rhs-only median ratio는 0.378 (범위 0.376–0.386)이며 instruction slope가 약 141.8% 증가했습니다. 이 비교는 지정된 workload에 한정됩니다.
- instruction profile: cubic에서 generic 3,018,359.75 instructions/repetition 대비 scalar 2,187,948.65, algebraic 2,225,167, rational 2,104,980입니다. structural 2048/64/1 profile에서 PR91 310,184,546, fused 312,580,908, enclosed 107,185,903 instructions/repetition입니다. 이는 Callgrind guest instruction count이며 native wall time이 아닙니다.

## 판정 한계

실행 명령과 종료 코드는 JSON의 `commands` 및 suite 영수증으로 연결했습니다. 구조 경로의 오차·잔차는 각 benchmark 출력의 recorded check입니다. 외부 독립 참조 검증으로 간주하지 않았습니다. Callgrind 결과를 시간으로 환산하지 않았고 서로 다른 cubic/structural/adapter 집단의 수치를 합치지 않았습니다. 따라서 결과는 범위가 제한된 진단/비교 근거이며 broad production qualification을 뜻하지 않습니다.
