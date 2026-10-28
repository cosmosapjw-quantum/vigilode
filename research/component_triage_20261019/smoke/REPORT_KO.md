# 컴포넌트 트리아지 보고서 (자동 생성)

> 이 문서는 `tools/triage/triage_report.py`가 `RESULTS.json`과 `CLASSIFICATION.json`만으로 생성한다. 손으로 편집하지 않는다 (PA-06). `--check`가 재생성 결과와 바이트 단위로 비교한다.

- 캠페인: `ct-20261019-smoke`
- RESULTS.json SHA-256: `ce17d7a41c72140d37f20f937987dd83c3b4cb10f4235cddfc9ae89a94e3e2b0`
- CLASSIFICATION.json SHA-256: `aaddaaf6d95bcb3a92938294a94c6791bd693a0869eb4cb2b23469003e790888`
- 생성기: `tools/triage/triage_check.py`

## 입력

| 역할 | 파일 | SHA-256 |
|---|---|---|
| allocations | ALLOCATIONS.jsonl | `a628b2cf9b087b1ceda281d639f048773d3f846987f5d2563cf3e7dcb6bad418` |
| attribution | — | — |
| components | COMPONENTS.json | `d455351c9888062c2560a722a96bca6eb547f2a96f4a7eff23c7af79a85da746` |
| corpus | CORPUS.json | `d308ab1cc5a7c9cd7bf2d1967b7f95dbf2ac00f907118ad8dc01b9ae8647a4e7` |
| list | TRIAGE_LIST.json | `5ea915d2bc4f5a77350de64a201f795a8b1cfc97831db1bb252cc01d5998dab9` |
| probes | PROBES.jsonl | `2bcb9429247d7b601488370bbb9f945dc913aee871d7bdc936cf11fe3ca8b456` |
| profile | PROFILE.jsonl | `6b0531fe896e0481b13c98601e146d75ecf46a48df9ac8452cab7261aa67f298` |
| references | REFERENCES.json | `16d0fa23832819b8d93c49d5db43984f44c551d3a31dfd56b5b79350ccb133f5` |
| runs | RUNS.jsonl.gz | `c088eca09f980c463c6d3b9eed2ff89f0834e2f12cf1394bdd92770c73cd5290` |
| thresholds | THRESHOLDS.json | `6e040dfe5f118b37cd3ddd06e20322113f28769fcb8193b891b5b40fdee19bb4` |
| timing | — | — |

## 요약

| 라벨 | 개수 |
|---|---|
| PROMOTE | 6 |
| SPECIALIZE | 1 |
| OPTIMIZE | 25 |
| PRUNE | 120 |
| 잠정 (provisional) | 61 |
| 전체 | 152 |

판정 규칙과 임계값은 `RESULTS.json`의 `thresholds`(THRESHOLDS.json 사본)를 따른다. 비율 = cost(컴포넌트 arm) / cost(상대 arm), 비용 = PROFILE `ir_warm` (h0 = 1 셀).

## 패밀리별 분류

### F1

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `bdf.order1` | bdf_order | PRUNE | 아니오 | 1094 | 1 | 7 | 7 | 0 | 0 | 401 | 15.38 | — |
| `bdf.order2` | bdf_order | PRUNE | 아니오 | 1094 | 21 | 7 | 2 | 0 | 0 | 802 | 1.327 | — |
| `drv.bdf` | driver | PRUNE | 아니오 | 1464 | 0 | 0 | 6 | 0 | 0 | 1242 | 1098 | — |
| `drv.exp_pexprb54s4` | driver | PRUNE | 예 | 4225 | 0 | 0 | 0 | 0 | 0 | 176 | — | NO_EVIDENCE |
| `drv.fast` | driver | PRUNE | 아니오 | 1520 | 2 | 14 | 4 | 0 | 0 | 1900 | 1.16 | — |
| `drv.fast_banded` | driver | OPTIMIZE | 예 | 1520 | 0 | 0 | 0 | 0 | 0 | 96 | — | NO_EVIDENCE |
| `drv.fast_small` | driver | SPECIALIZE | 아니오 | 992 | 14 | 0 | 0 | 0 | 0 | 322 | 0.6092 | — |
| `drv.fast_small_batch` | driver | OPTIMIZE | 예 | 609 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `drv.homotopy` | driver | PRUNE | 예 | 2496 | 0 | 0 | 28 | 16 | 0 | 132 | 240.1 | — |
| `drv.mf_fast` | driver | PRUNE | 아니오 | 3392 | 0 | 0 | 128 | 0 | 0 | 6592 | 27.46 | — |
| `drv.radau` | driver | PRUNE | 아니오 | 1338 | 0 | 0 | 10 | 0 | 0 | 2070 | 3264 | — |
| `drv.routed` | driver | PROMOTE | 아니오 | 676 | 30 | 0 | 0 | 13 | 0 | 725 | 0.3073 | — |
| `drv.sabr` | driver | PRUNE | 아니오 | 1157 | 0 | 0 | 8 | 0 | 0 | 888 | 6.718 | — |
| `drv.seq_direct` | driver | PRUNE | 아니오 | 2462 | 0 | 0 | 12 | 0 | 0 | 1332 | 5.945 | — |
| `drv.seq_krylov` | driver | PRUNE | 아니오 | 1179 | 0 | 0 | 10 | 0 | 0 | 1110 | 67.42 | — |
| `drv.seq_mf` | driver | PRUNE | 예 | 4155 | 0 | 0 | 0 | 0 | 0 | 1408 | — | NO_EVIDENCE |
| `drv.transactional_q1q2` | driver | PRUNE | 예 | 2001 | 0 | 0 | 0 | 0 | 0 | 352 | — | NO_EVIDENCE |
| `radau.reuse_stage_lu` | radau_error_lu | OPTIMIZE | 아니오 | 968 | 20 | 40 | 0 | 0 | 0 | 772 | 0.9959 | — |
| `radau.stages1` | radau_stages | PRUNE | 아니오 | 968 | 0 | 0 | 14 | 0 | 0 | 402 | 225.9 | — |
| `radau.stages3` | radau_stages | OPTIMIZE | 아니오 | 968 | 56 | 0 | 0 | 0 | 0 | 1608 | 0.4929 | — |
| `rt.declared_structure` | routing_declaration | PROMOTE | 아니오 | 676 | 0 | 28 | 0 | 0 | 0 | 356 | 1.002 | — |
| `blk.explicit` | sabr_block_method | PRUNE | 아니오 | 748 | 26 | 51 | 7 | 0 | 0 | 588 | 1.107 | — |
| `blk.forward` | sabr_block_method | OPTIMIZE | 아니오 | 748 | 41 | 43 | 0 | 0 | 0 | 588 | 1.001 | — |
| `blk.gmres` | sabr_block_method | PRUNE | 아니오 | 748 | 0 | 18 | 66 | 0 | 0 | 588 | 1.485 | — |
| `blk.nilpotent` | sabr_block_method | PRUNE | 아니오 | 748 | 21 | 47 | 16 | 0 | 0 | 588 | 1.107 | — |
| `sm.static_solves` | small_static | PROMOTE | 아니오 | 992 | 53 | 7 | 0 | 0 | 0 | 84 | 0.9887 | — |
| `sm.static_stages` | small_static | PROMOTE | 아니오 | 992 | 60 | 0 | 0 | 0 | 0 | 84 | 0.9537 | — |

### F10

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `scf.a1_two_arm_receipt` | scaffolding | PRUNE (relocate) | 아니오 | 400 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.audit2_bateman_real_client_research` | scaffolding | PRUNE (relocate) | 아니오 | 1314 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.audit2_matrix_free_research` | scaffolding | PRUNE (relocate) | 아니오 | 1000 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.audit2_research` | scaffolding | PRUNE (relocate) | 아니오 | 1814 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.audit2_reusable_transaction_research` | scaffolding | PRUNE (relocate) | 아니오 | 1336 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.audit2_stage_certificate_research` | scaffolding | PRUNE (relocate) | 아니오 | 968 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.candidates` | scaffolding | PRUNE (relocate) | 아니오 | 372 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.chart_transport` | scaffolding | PRUNE (relocate) | 아니오 | 490 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.common_w_gate` | scaffolding | PRUNE (relocate) | 아니오 | 915 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.comparator_fidelity` | scaffolding | PRUNE (relocate) | 아니오 | 135 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.controller_telemetry` | scaffolding | PRUNE (relocate) | 아니오 | 182 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.g1_transactional_gate` | scaffolding | PRUNE (relocate) | 아니오 | 449 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.g2_exponential_gate` | scaffolding | PRUNE (relocate) | 아니오 | 678 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.g3_fused_adaptive_gate` | scaffolding | PRUNE (relocate) | 아니오 | 887 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.g4_prefix_kernel_gate` | scaffolding | PRUNE (relocate) | 아니오 | 673 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.g4_s5b0_regime_atlas` | scaffolding | PRUNE (relocate) | 아니오 | 6067 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.g4_s5b0_trace_authority` | scaffolding | PRUNE (relocate) | 아니오 | 167 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.g4_s5b3_attempt_geometry` | scaffolding | PRUNE (relocate) | 아니오 | 1116 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.homotopy_experiments` | scaffolding | PRUNE (relocate) | 아니오 | 878 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.homotopy_order_policy` | scaffolding | PRUNE (relocate) | 아니오 | 868 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.native_gates` | scaffolding | PRUNE (relocate) | 아니오 | 215 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.path_controller` | scaffolding | PRUNE (relocate) | 아니오 | 763 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.policy_redesign_v25` | scaffolding | PRUNE (relocate) | 아니오 | 179 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.rhs_telemetry` | scaffolding | PRUNE (relocate) | 아니오 | 1148 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.scientific_validity_v2_gate` | scaffolding | PRUNE (relocate) | 아니오 | 664 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.unified_gates` | scaffolding | PRUNE (relocate) | 아니오 | 964 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.unified_screen` | scaffolding | PRUNE (relocate) | 아니오 | 955 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `scf.v38d_performance_tournament` | scaffolding | PRUNE (relocate) | 아니오 | 202 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |

### F11

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `corp.gray_scott` | corpus | PRUNE (relocate) | 아니오 | 307 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `corp.problems` | corpus | PRUNE (relocate) | 아니오 | 824 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |
| `corp.scientific_corpus_v2` | corpus | PRUNE (relocate) | 아니오 | 1426 | 0 | 0 | 0 | 0 | 0 | 0 | — | — |

### F2

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ctl.integral` | controller | PRUNE | 아니오 | 1354 | 70 | 25 | 17 | 1640 | 0 | 4280 | 2.244 | — |
| `ctl.pi` | controller | PRUNE | 아니오 | 1354 | 9 | 30 | 76 | 0 | 0 | 653 | 1.439 | — |
| `ctl.predictive` | controller | PRUNE | 아니오 | 1354 | 23 | 69 | 23 | 0 | 0 | 653 | 2.313 | — |
| `ctl.predictive_capped` | controller | PRUNE | 아니오 | 1354 | 23 | 70 | 22 | 0 | 0 | 653 | 2.331 | — |
| `ctl.predictive_capped2` | controller | PRUNE | 아니오 | 1354 | 23 | 76 | 16 | 0 | 0 | 653 | 2.319 | — |
| `ctl.fast_fused_landing` | fast_controller_path | PROMOTE | 아니오 | 1520 | 163 | 49 | 0 | 0 | 0 | 796 | 0.9903 | — |
| `ctl.fast_prevalidated` | fast_controller_path | PROMOTE | 아니오 | 2512 | 94 | 105 | 13 | 0 | 0 | 796 | 1.044 | — |
| `ctl.inner_tolerance` | inner_tolerance | OPTIMIZE | 아니오 | 307 | 0 | 0 | 0 | 287 | 0 | 1121 | — | — |
| `ctl.max_step_slack` | max_step_policy | PRUNE | 아니오 | 1570 | 70 | 31 | 22 | 0 | 0 | 757 | 3.015 | — |
| `ctl.max_step_strict` | max_step_policy | OPTIMIZE | 아니오 | 1570 | 6 | 22 | 0 | 0 | 0 | 148 | 1.011 | — |
| `nl.fixed` | newton_tolerance | PRUNE | 아니오 | 370 | 10 | 0 | 99 | 0 | 0 | 1971 | 419.8 | — |
| `nl.scaled` | newton_tolerance | PRUNE | 아니오 | 370 | 70 | 0 | 10 | 0 | 0 | 1168 | 7.995 | — |

### F3

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `out.dense_enforce` | dense_error_control | PRUNE | 예 | 2976 | 0 | 30 | 39 | 0 | 0 | 459 | 1.495 | — |
| `out.dense_off` | dense_error_control | PRUNE | 아니오 | 2976 | 54 | 15 | 0 | 0 | 0 | 459 | 0.9898 | — |
| `out.dense_report` | dense_error_control | PRUNE | 아니오 | 2976 | 5 | 30 | 34 | 0 | 0 | 459 | 1.083 | — |
| `out.dense_report_defect` | dense_error_control | PRUNE | 아니오 | 2976 | 27 | 26 | 16 | 0 | 0 | 459 | 1.048 | — |
| `out.verified_pr101` | dense_error_control | OPTIMIZE | 예 | 3843 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `out.collector` | output_collector | OPTIMIZE | 아니오 | 1570 | 0 | 0 | 0 | 583 | 0 | 985 | — | — |

### F4

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `la.banded_indexed` | banded_kernel | PRUNE | 예 | 1520 | 0 | 0 | 0 | 0 | 0 | 48 | — | NO_EVIDENCE |
| `la.banded_slices` | banded_kernel | PRUNE | 예 | 1520 | 0 | 0 | 0 | 0 | 0 | 48 | — | NO_EVIDENCE |
| `la.faer_lu` | dense_lu | PRUNE | 아니오 | 311 | 96 | 85 | 222 | 88 | 0 | 2581 | 17.06 | — |
| `la.lu_colext` | dense_lu | PRUNE | 아니오 | 1520 | 0 | 16 | 40 | 0 | 0 | 328 | 1.07 | — |
| `la.lu_colext64` | dense_lu | PRUNE | 예 | 1520 | 0 | 0 | 0 | 0 | 0 | 96 | — | NO_EVIDENCE |
| `la.small_lu` | dense_lu | PRUNE | 아니오 | 1520 | 160 | 26 | 31 | 0 | 0 | 1079 | 1.345 | — |
| `la.sparse_density_switch` | dense_lu | PRUNE | 예 | 1520 | 0 | 0 | 0 | 0 | 0 | 432 | — | NO_EVIDENCE |
| `la.sparse_direct` | sparse_lu | OPTIMIZE | 예 | 222 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `la.stage_batch` | stage_parallel | PRUNE | 예 | 850 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `la.woodbury_collective` | structured_lu | OPTIMIZE | 예 | 474 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |

### F5

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `kr.recycle_cold` | gcrodr_recycle_policy | PRUNE | 예 | 4702 | 0 | 0 | 0 | 0 | 0 | 320 | — | NO_EVIDENCE |
| `kr.recycle_legacy` | gcrodr_recycle_policy | PRUNE | 예 | 4702 | 0 | 0 | 0 | 0 | 0 | 320 | — | NO_EVIDENCE |
| `kr.recycle_refresh_after_update` | gcrodr_recycle_policy | PRUNE | 예 | 4702 | 0 | 0 | 0 | 0 | 0 | 640 | — | NO_EVIDENCE |
| `kr.x0_previous` | krylov_initial_guess | PRUNE | 아니오 | 1287 | 0 | 2 | 14 | 0 | 0 | 336 | 1.169 | — |
| `kr.x0_prevstep` | krylov_initial_guess | PRUNE | 예 | 1287 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `kr.x0_prevstep_scaled` | krylov_initial_guess | PRUNE | 예 | 1287 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `kr.x0_zero` | krylov_initial_guess | PRUNE | 아니오 | 1287 | 14 | 2 | 0 | 0 | 0 | 336 | 0.9857 | — |
| `kr.ls_workspace` | krylov_least_squares | OPTIMIZE | 아니오 | 719 | 42 | 0 | 0 | 0 | 0 | 598 | 0.7624 | — |
| `kr.block_gmres` | krylov_method | PRUNE | 예 | 635 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `kr.gcrodr` | krylov_method | OPTIMIZE | 아니오 | 1310 | 72 | 0 | 0 | 0 | 0 | 2328 | 0.7418 | — |
| `kr.gmres` | krylov_method | PRUNE | 아니오 | 887 | 28 | 0 | 76 | 0 | 0 | 1816 | 4.541 | — |
| `kr.gmres_givens` | krylov_method | PRUNE | 예 | 712 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `kr.gmres_into` | krylov_method | OPTIMIZE | 아니오 | 509 | 34 | 0 | 0 | 0 | 0 | 1886 | 0.959 | — |
| `kr.gmres_staged` | krylov_method | PRUNE | 예 | 1303 | 0 | 0 | 0 | 0 | 0 | 1600 | — | NO_EVIDENCE |
| `kr.lgmres` | krylov_method | PRUNE | 아니오 | 291 | 0 | 0 | 72 | 0 | 0 | 1368 | 4.351 | — |
| `kr.lgmres_into` | krylov_method | PRUNE | 예 | 646 | 0 | 0 | 0 | 0 | 0 | 640 | — | NO_EVIDENCE |
| `kr.accounting_recompute_final` | krylov_residual_accounting | PRUNE | 아니오 | 92 | 51 | 9 | 56 | 28 | 0 | 1840 | 4.43 | — |
| `kr.accounting_reuse_confirmed` | krylov_residual_accounting | PRUNE | 아니오 | 92 | 13 | 0 | 8 | 0 | 0 | 299 | 1.121 | — |
| `kr.mf_warm_start` | krylov_start_policy | PRUNE | 예 | 3392 | 0 | 0 | 0 | 0 | 0 | 160 | — | NO_EVIDENCE |
| `kr.start_explicit_zero` | krylov_start_policy | PRUNE | 예 | 3392 | 0 | 0 | 0 | 0 | 0 | 480 | — | NO_EVIDENCE |
| `kr.workspace_options` | krylov_workspace | OPTIMIZE | 아니오 | 3392 | 0 | 0 | 0 | 148 | 0 | 492 | — | — |
| `kr.pc_direct` | preconditioner | PRUNE | 아니오 | 736 | 40 | 0 | 16 | 0 | 0 | 392 | 1.144 | — |
| `kr.pc_jacobi` | preconditioner | PRUNE | 아니오 | 736 | 8 | 6 | 42 | 0 | 0 | 392 | 4.052 | — |
| `kr.pc_none` | preconditioner | PRUNE | 아니오 | 736 | 58 | 6 | 48 | 0 | 0 | 784 | 3.878 | — |
| `kr.pc_right` | preconditioner_side | PRUNE | 예 | 731 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `kr.recycle_scope_currentw` | recycle_scope | PRUNE | 예 | 3392 | 0 | 0 | 0 | 0 | 0 | 160 | — | NO_EVIDENCE |
| `kr.stage_target_coupled` | stage_target_policy | PRUNE | 아니오 | 3761 | 42 | 0 | 42 | 0 | 0 | 1196 | 2.92 | — |
| `kr.stage_target_coupled_guarded` | stage_target_policy | PRUNE | 아니오 | 3761 | 7 | 14 | 63 | 0 | 0 | 1196 | 3.794 | — |
| `kr.stage_target_coupled_guarded2` | stage_target_policy | PRUNE | 아니오 | 3761 | 0 | 14 | 70 | 0 | 0 | 1196 | 3.795 | — |
| `kr.stage_target_dupfix` | stage_target_policy | OPTIMIZE | 아니오 | 3761 | 21 | 0 | 0 | 0 | 0 | 299 | 0.9766 | — |
| `kr.stage_target_l2coupled` | stage_target_policy | OPTIMIZE | 아니오 | 3761 | 76 | 8 | 0 | 0 | 0 | 1196 | 1.003 | — |
| `kr.stage_target_legacy` | stage_target_policy | PRUNE | 아니오 | 3761 | 0 | 0 | 21 | 0 | 0 | 299 | 1.034 | — |
| `kr.stage_target_projl2` | stage_target_policy | PRUNE | 아니오 | 3761 | 63 | 8 | 13 | 0 | 0 | 1196 | 1.028 | — |

### F6

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `phi.binary_scaling` | phi_action | PRUNE | 예 | 161 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `phi.complex_shift_jet` | phi_action | PRUNE | 예 | 711 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `phi.fused` | phi_action | PRUNE | 예 | 3849 | 0 | 0 | 0 | 0 | 0 | 80 | — | NO_EVIDENCE |
| `phi.krylov` | phi_action | PRUNE | 예 | 3849 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `phi.laguerre` | phi_action | PRUNE | 예 | 3477 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `phi.laguerre_adjoint` | phi_action | PRUNE | 예 | 479 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `phi.leja` | phi_action | OPTIMIZE | 예 | 3774 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `phi.pade13` | phi_action | PRUNE | 예 | 473 | 0 | 0 | 0 | 0 | 0 | 80 | — | NO_EVIDENCE |
| `phi.poly_router` | phi_action | PRUNE | 예 | 4367 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `phi.shared_shift` | phi_action | PRUNE | 예 | 1262 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `phi.taylor` | phi_action | PRUNE | 예 | 298 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `phi.taylor_total` | phi_action | PRUNE | 예 | 1048 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |

### F7

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `cert.bound_pr101` | certificate | PRUNE | 예 | 867 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `cert.causal_majorant` | certificate | PRUNE | 예 | 621 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `cert.certified_budget` | certificate | PRUNE | 예 | 255 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `cert.directed` | certificate | PRUNE | 예 | 556 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `cert.fourier_path` | certificate | PRUNE | 예 | 1452 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `cert.nonnormal` | certificate | PRUNE | 예 | 886 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `cert.outward` | certificate | PRUNE | 예 | 1959 | 0 | 0 | 0 | 0 | 0 | 0 | — | NO_EVIDENCE |
| `cert.raw_stage_target` | certificate | PRUNE | 예 | 369 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `cert.residual_gain` | certificate | PRUNE | 예 | 1924 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `cert.stage_chart` | certificate | PRUNE | 예 | 329 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `cert.stage_target` | certificate | PRUNE | 예 | 257 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `cert.transform_bound` | certificate | PRUNE | 예 | 560 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |

### F8

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `pk.bound_collective` | packet | OPTIMIZE | 예 | 280 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `pk.collective_model` | packet | OPTIMIZE | 예 | 511 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `pk.curve` | packet | OPTIMIZE | 예 | 1487 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `pk.damped_moments` | packet | OPTIMIZE | 예 | 872 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `pk.trajectory` | packet | OPTIMIZE | 예 | 1937 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `pk.window` | packet | OPTIMIZE | 예 | 2110 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `pk.work` | packet | OPTIMIZE | 예 | 228 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |

### F9

| 컴포넌트 | 슬롯 | 라벨 | 잠정 | LOC | GAIN | NEUTRAL | LOSS | 역량 우위 | 역량 손실 | 제외 | 최대 비율 | 플래그 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `par.homotopy_parallel` | parallel | PRUNE | 예 | 1838 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |
| `par.pool` | parallel | PRUNE | 예 | 130 | 0 | 0 | 0 | 0 | 0 | 0 | — | C3_MISSING, NO_EVIDENCE |

## SPECIALIZE 술어

| 컴포넌트 | 특징 | 연산 | 임계값/값 | 보정 승 (포함) | 보정 승 (전체) | 홀드아웃 판정 | 홀드아웃 양성 | 홀드아웃 승 포함 |
|---|---|---|---|---|---|---|---|---|
| `drv.fast_small` | applicability | API | fast-small, rodas5p-fast-small-land, rodas5p-fast-small-ovh, rodas5p-fast-small-static, rodas5p-fast-small-static-ovh, rodas5p-fast-small-static-stages, rodas5p-fast-small-val (coverage 0.214) | — | — | deterministic | — | — |

## 기준 상태

| 컴포넌트 | C1 | C2 | C3 | 할당 | PROMOTE | SPECIALIZE | OPTIMIZE | 타이밍 |
|---|---|---|---|---|---|---|---|---|
| `bdf.order1` | pass | fail | pass | fail | fail | fail | fail | null |
| `bdf.order2` | pass | fail | pass | fail | fail | fail | fail | null |
| `drv.bdf` | pass | fail | pass | fail | fail | fail | fail | null |
| `drv.exp_pexprb54s4` | pass | fail | pass | fail | fail | fail | fail | null |
| `drv.fast` | pass | pass | pass | fail | fail | fail | fail | null |
| `drv.fast_banded` | null | null | pass | null | fail | fail | fail | null |
| `drv.fast_small` | pass | pass | pass | pass | fail | fail | fail | null |
| `drv.fast_small_batch` | null | null | null | null | fail | fail | fail | null |
| `drv.homotopy` | pass | pass | pass | fail | fail | fail | fail | null |
| `drv.mf_fast` | pass | pass | pass | fail | fail | fail | fail | null |
| `drv.radau` | pass | pass | pass | fail | fail | fail | fail | null |
| `drv.routed` | pass | pass | pass | pass | pass | fail | fail | null |
| `drv.sabr` | pass | pass | pass | fail | fail | fail | fail | null |
| `drv.seq_direct` | pass | pass | pass | fail | fail | fail | fail | null |
| `drv.seq_krylov` | pass | pass | pass | fail | fail | fail | fail | null |
| `drv.seq_mf` | pass | fail | pass | fail | fail | fail | fail | null |
| `drv.transactional_q1q2` | pass | pass | pass | fail | fail | fail | fail | null |
| `radau.reuse_stage_lu` | pass | pass | pass | fail | fail | fail | fail | null |
| `radau.stages1` | pass | fail | pass | fail | fail | fail | fail | null |
| `radau.stages3` | pass | pass | pass | fail | fail | fail | fail | null |
| `rt.declared_structure` | pass | pass | pass | pass | pass | fail | fail | null |
| `blk.explicit` | pass | pass | pass | fail | fail | fail | fail | null |
| `blk.forward` | pass | pass | pass | fail | fail | fail | fail | null |
| `blk.gmres` | pass | pass | pass | fail | fail | fail | fail | null |
| `blk.nilpotent` | pass | pass | pass | fail | fail | fail | fail | null |
| `sm.static_solves` | pass | pass | pass | pass | pass | fail | fail | null |
| `sm.static_stages` | pass | pass | pass | pass | pass | fail | fail | null |
| `scf.a1_two_arm_receipt` | null | null | null | null | fail | fail | fail | null |
| `scf.audit2_bateman_real_client_research` | null | null | null | null | fail | fail | fail | null |
| `scf.audit2_matrix_free_research` | null | null | null | null | fail | fail | fail | null |
| `scf.audit2_research` | null | null | null | null | fail | fail | fail | null |
| `scf.audit2_reusable_transaction_research` | null | null | null | null | fail | fail | fail | null |
| `scf.audit2_stage_certificate_research` | null | null | null | null | fail | fail | fail | null |
| `scf.candidates` | null | null | null | null | fail | fail | fail | null |
| `scf.chart_transport` | null | null | null | null | fail | fail | fail | null |
| `scf.common_w_gate` | null | null | null | null | fail | fail | fail | null |
| `scf.comparator_fidelity` | null | null | null | null | fail | fail | fail | null |
| `scf.controller_telemetry` | null | null | null | null | fail | fail | fail | null |
| `scf.g1_transactional_gate` | null | null | null | null | fail | fail | fail | null |
| `scf.g2_exponential_gate` | null | null | null | null | fail | fail | fail | null |
| `scf.g3_fused_adaptive_gate` | null | null | null | null | fail | fail | fail | null |
| `scf.g4_prefix_kernel_gate` | null | null | null | null | fail | fail | fail | null |
| `scf.g4_s5b0_regime_atlas` | null | null | null | null | fail | fail | fail | null |
| `scf.g4_s5b0_trace_authority` | null | null | null | null | fail | fail | fail | null |
| `scf.g4_s5b3_attempt_geometry` | null | null | null | null | fail | fail | fail | null |
| `scf.homotopy_experiments` | null | null | null | null | fail | fail | fail | null |
| `scf.homotopy_order_policy` | null | null | null | null | fail | fail | fail | null |
| `scf.native_gates` | null | null | null | null | fail | fail | fail | null |
| `scf.path_controller` | null | null | null | null | fail | fail | fail | null |
| `scf.policy_redesign_v25` | null | null | null | null | fail | fail | fail | null |
| `scf.rhs_telemetry` | null | null | null | null | fail | fail | fail | null |
| `scf.scientific_validity_v2_gate` | null | null | null | null | fail | fail | fail | null |
| `scf.unified_gates` | null | null | null | null | fail | fail | fail | null |
| `scf.unified_screen` | null | null | null | null | fail | fail | fail | null |
| `scf.v38d_performance_tournament` | null | null | null | null | fail | fail | fail | null |
| `corp.gray_scott` | null | null | null | null | fail | fail | fail | null |
| `corp.problems` | null | null | null | null | fail | fail | fail | null |
| `corp.scientific_corpus_v2` | null | null | null | null | fail | fail | fail | null |
| `ctl.integral` | pass | pass | pass | fail | fail | fail | fail | null |
| `ctl.pi` | pass | pass | pass | fail | fail | fail | fail | null |
| `ctl.predictive` | pass | pass | pass | pass | fail | fail | fail | null |
| `ctl.predictive_capped` | pass | pass | pass | pass | fail | fail | fail | null |
| `ctl.predictive_capped2` | pass | pass | pass | pass | fail | fail | fail | null |
| `ctl.fast_fused_landing` | pass | pass | pass | pass | pass | fail | fail | null |
| `ctl.fast_prevalidated` | pass | pass | pass | pass | pass | fail | fail | null |
| `ctl.inner_tolerance` | pass | fail | pass | fail | fail | fail | pass | null |
| `ctl.max_step_slack` | pass | pass | pass | fail | fail | fail | fail | null |
| `ctl.max_step_strict` | pass | pass | pass | fail | fail | fail | fail | null |
| `nl.fixed` | pass | pass | pass | fail | fail | fail | fail | null |
| `nl.scaled` | pass | pass | pass | fail | fail | fail | fail | null |
| `out.dense_enforce` | pass | pass | pass | fail | fail | fail | fail | null |
| `out.dense_off` | pass | fail | pass | fail | fail | fail | fail | null |
| `out.dense_report` | pass | fail | pass | fail | fail | fail | fail | null |
| `out.dense_report_defect` | pass | fail | pass | fail | fail | fail | fail | null |
| `out.verified_pr101` | null | null | null | null | fail | fail | fail | null |
| `out.collector` | pass | pass | pass | fail | fail | fail | pass | null |
| `la.banded_indexed` | null | null | pass | null | fail | fail | fail | null |
| `la.banded_slices` | null | null | pass | null | fail | fail | fail | null |
| `la.faer_lu` | pass | pass | pass | fail | fail | fail | fail | null |
| `la.lu_colext` | pass | pass | pass | pass | fail | fail | fail | null |
| `la.lu_colext64` | pass | pass | pass | pass | fail | fail | fail | null |
| `la.small_lu` | pass | pass | pass | fail | fail | fail | fail | null |
| `la.sparse_density_switch` | pass | pass | pass | fail | fail | fail | fail | null |
| `la.sparse_direct` | null | null | null | null | fail | fail | fail | null |
| `la.stage_batch` | null | null | null | null | fail | fail | fail | null |
| `la.woodbury_collective` | null | null | null | null | fail | fail | fail | null |
| `kr.recycle_cold` | pass | pass | pass | null | fail | fail | fail | null |
| `kr.recycle_legacy` | pass | pass | pass | null | fail | fail | fail | null |
| `kr.recycle_refresh_after_update` | pass | pass | pass | null | fail | fail | fail | null |
| `kr.x0_previous` | pass | fail | pass | fail | fail | fail | fail | null |
| `kr.x0_prevstep` | null | null | null | null | fail | fail | fail | null |
| `kr.x0_prevstep_scaled` | null | null | null | null | fail | fail | fail | null |
| `kr.x0_zero` | pass | fail | pass | fail | fail | fail | fail | null |
| `kr.ls_workspace` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.block_gmres` | null | null | null | null | fail | fail | fail | null |
| `kr.gcrodr` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.gmres` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.gmres_givens` | null | null | null | null | fail | fail | fail | null |
| `kr.gmres_into` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.gmres_staged` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.lgmres` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.lgmres_into` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.accounting_recompute_final` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.accounting_reuse_confirmed` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.mf_warm_start` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.start_explicit_zero` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.workspace_options` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.pc_direct` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.pc_jacobi` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.pc_none` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.pc_right` | null | null | null | null | fail | fail | fail | null |
| `kr.recycle_scope_currentw` | pass | pass | pass | null | fail | fail | fail | null |
| `kr.stage_target_coupled` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.stage_target_coupled_guarded` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.stage_target_coupled_guarded2` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.stage_target_dupfix` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.stage_target_l2coupled` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.stage_target_legacy` | pass | pass | pass | fail | fail | fail | fail | null |
| `kr.stage_target_projl2` | pass | pass | pass | fail | fail | fail | fail | null |
| `phi.binary_scaling` | null | null | null | null | fail | fail | fail | null |
| `phi.complex_shift_jet` | null | null | null | null | fail | fail | fail | null |
| `phi.fused` | pass | fail | pass | fail | fail | fail | fail | null |
| `phi.krylov` | null | null | null | null | fail | fail | fail | null |
| `phi.laguerre` | null | null | null | null | fail | fail | fail | null |
| `phi.laguerre_adjoint` | null | null | null | null | fail | fail | fail | null |
| `phi.leja` | null | null | null | null | fail | fail | fail | null |
| `phi.pade13` | pass | fail | pass | fail | fail | fail | fail | null |
| `phi.poly_router` | null | null | null | null | fail | fail | fail | null |
| `phi.shared_shift` | null | null | null | null | fail | fail | fail | null |
| `phi.taylor` | null | null | null | null | fail | fail | fail | null |
| `phi.taylor_total` | null | null | null | null | fail | fail | fail | null |
| `cert.bound_pr101` | null | null | null | null | fail | fail | fail | null |
| `cert.causal_majorant` | null | null | null | null | fail | fail | fail | null |
| `cert.certified_budget` | null | null | null | null | fail | fail | fail | null |
| `cert.directed` | null | null | null | null | fail | fail | fail | null |
| `cert.fourier_path` | null | null | null | null | fail | fail | fail | null |
| `cert.nonnormal` | null | null | null | null | fail | fail | fail | null |
| `cert.outward` | pass | pass | pass | fail | fail | fail | fail | null |
| `cert.raw_stage_target` | null | null | null | null | fail | fail | fail | null |
| `cert.residual_gain` | null | null | null | null | fail | fail | fail | null |
| `cert.stage_chart` | null | null | null | null | fail | fail | fail | null |
| `cert.stage_target` | null | null | null | null | fail | fail | fail | null |
| `cert.transform_bound` | null | null | null | null | fail | fail | fail | null |
| `pk.bound_collective` | null | null | null | null | fail | fail | fail | null |
| `pk.collective_model` | null | null | null | null | fail | fail | fail | null |
| `pk.curve` | null | null | null | null | fail | fail | fail | null |
| `pk.damped_moments` | null | null | null | null | fail | fail | fail | null |
| `pk.trajectory` | null | null | null | null | fail | fail | fail | null |
| `pk.window` | null | null | null | null | fail | fail | fail | null |
| `pk.work` | null | null | null | null | fail | fail | fail | null |
| `par.homotopy_parallel` | null | null | null | null | fail | fail | fail | null |
| `par.pool` | null | null | null | null | fail | fail | fail | null |

## 규칙 경로

### `bdf.order1` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, c2, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `bdf.order2` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: max_ratio, c2, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `drv.bdf` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, c2, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: insufficient_evidence: no matched cell with complete counted work on both sides

### `drv.exp_pexprb54s4` → PRUNE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A11'] -> provisional, PROMOTE forbidden
2. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)
- blocked_by: A11

### `drv.fast` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: insufficient_evidence: no matched cell with complete counted work on both sides

### `drv.fast_banded` → OPTIMIZE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 null, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)

### `drv.fast_small` → SPECIALIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `SPECIALIZE_APPLICABILITY` (regime): every other PROMOTE check passes but coverage 0.21428571428571427 < 0.5 (3 of 14 G/P problems of its Jacobian class ['analytic']): the predicate is the arms' applicability (deterministic API requirement; no holdout search)

### `drv.fast_small_batch` → OPTIMIZE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)

### `drv.homotopy` → PRUNE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A09'] -> provisional, PROMOTE forbidden
2. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
3. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
4. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
5. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS
- blocked_by: A09

### `drv.mf_fast` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: insufficient_evidence: no matched cell with complete counted work on both sides

### `drv.radau` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: insufficient_evidence: no matched cell with complete counted work on both sides

### `drv.routed` → PROMOTE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `PROMOTE` (frontier): G/P fraction 1.0 >= 0.9, max ratio 0.3073357490192659 <= 1.1, C1/C2/C3/allocation pass

### `drv.sabr` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `drv.seq_direct` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: insufficient_evidence: no matched cell with complete counted work on both sides

### `drv.seq_krylov` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `drv.seq_mf` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `drv.transactional_q1q2` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `radau.reuse_stage_lu` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 428.9138827023014 > 0.01 (radau5-tier-a prothero-robinson-nl uniform101)

### `radau.stages1` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, c2, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `radau.stages3` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 432.41301658005443 > 0.01 (radau5-scaled prothero-robinson-nl uniform101)

### `rt.declared_structure` → PROMOTE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `PROMOTE` (frontier): G/P fraction 1.0 >= 0.9, max ratio 1.001783536296789 <= 1.1, C1/C2/C3/allocation pass

### `blk.explicit` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `blk.forward` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 256.6659765080629 > 0.01 (sabr-forward van-der-pol-mu1000 uniform101)

### `blk.gmres` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `blk.nilpotent` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `sm.static_solves` → PROMOTE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `PROMOTE` (frontier): G/P fraction 1.0 >= 0.9, max ratio 0.9887167203198305 <= 1.1, C1/C2/C3/allocation pass

### `sm.static_stages` → PROMOTE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `PROMOTE` (frontier): G/P fraction 1.0 >= 0.9, max ratio 0.9537079142027123 <= 1.1, C1/C2/C3/allocation pass

### `scf.a1_two_arm_receipt` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.audit2_bateman_real_client_research` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.audit2_matrix_free_research` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.audit2_research` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.audit2_reusable_transaction_research` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.audit2_stage_certificate_research` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.candidates` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.chart_transport` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.common_w_gate` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.comparator_fidelity` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.controller_telemetry` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.g1_transactional_gate` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.g2_exponential_gate` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.g3_fused_adaptive_gate` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.g4_prefix_kernel_gate` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.g4_s5b0_regime_atlas` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.g4_s5b0_trace_authority` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.g4_s5b3_attempt_geometry` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.homotopy_experiments` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.homotopy_order_policy` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.native_gates` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.path_controller` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.policy_redesign_v25` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.rhs_telemetry` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.scientific_validity_v2_gate` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.unified_gates` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.unified_screen` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `scf.v38d_performance_tournament` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `corp.gray_scott` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `corp.problems` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `corp.scientific_corpus_v2` → PRUNE

1. `STATIC_PRUNE_RELOCATE` (static): static_class PRUNE_RELOCATE -> PRUNE(relocate)

### `ctl.integral` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `ctl.pi` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): no single-feature predicate separates calibration wins from losses
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `ctl.predictive` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio
3. `NOT_SPECIALIZE` (frontier): no single-feature predicate separates calibration wins from losses
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `ctl.predictive_capped` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio
3. `NOT_SPECIALIZE` (frontier): no single-feature predicate separates calibration wins from losses
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `ctl.predictive_capped2` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio
3. `NOT_SPECIALIZE` (frontier): no single-feature predicate separates calibration wins from losses
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `ctl.fast_fused_landing` → PROMOTE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `PROMOTE` (frontier): G/P fraction 1.0 >= 0.9, max ratio 0.9902889198194779 <= 1.1, C1/C2/C3/allocation pass

### `ctl.fast_prevalidated` → PROMOTE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `PROMOTE` (frontier): G/P fraction 0.9386792452830188 >= 0.9, max ratio 1.0442483912776552 <= 1.1, C1/C2/C3/allocation pass

### `ctl.inner_tolerance` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: c2, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 13
4. `OPTIMIZE_UNCHARACTERIZED_WINS` (frontier): unique capability, 78 win cell(s) without a validated predicate

### `ctl.max_step_slack` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `ctl.max_step_strict` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 220.0108613822764 > 0.01 (seq-direct-maxstep-strict hires uniform101)

### `nl.fixed` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): no single-feature predicate separates calibration wins from losses
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `nl.scaled` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `out.dense_enforce` → PRUNE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A05'] -> provisional, PROMOTE forbidden
2. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
3. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
4. `NOT_SPECIALIZE` (frontier): empty win set
5. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS
- blocked_by: A05

### `out.dense_off` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: c2, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: no LOSS comparison; external gap: no external_peers in COMPONENTS

### `out.dense_report` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, c2, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `out.dense_report_defect` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, c2, allocation
3. `NOT_SPECIALIZE` (frontier): no single-feature predicate separates calibration wins from losses
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `out.verified_pr101` → OPTIMIZE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A05'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)
- blocked_by: A05

### `out.collector` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 1494.6022743930766 > 0.01 (seq-lgmres hires uniform101)

### `la.banded_indexed` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 null, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `la.banded_slices` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 null, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `la.faer_lu` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `la.lu_colext` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `la.lu_colext64` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `la.small_lu` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): no single-feature predicate separates calibration wins from losses
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `la.sparse_density_switch` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `la.sparse_direct` → OPTIMIZE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)

### `la.stage_batch` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `la.woodbury_collective` → OPTIMIZE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A06', 'A07'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)
- blocked_by: A06, A07

### `kr.recycle_cold` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.recycle_legacy` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.recycle_refresh_after_update` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.x0_previous` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, c2, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.x0_prevstep` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.x0_prevstep_scaled` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.x0_zero` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: c2, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: no LOSS comparison; external gap: no external_peers in COMPONENTS

### `kr.ls_workspace` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 10.1796735018913 > 0.01 (mf-lgmres-into-ls-ws van-der-pol-mu1000 uniform101)

### `kr.block_gmres` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.gcrodr` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 684.9844290786397 > 0.01 (seq-gcrodr hires uniform101)

### `kr.gmres` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.gmres_givens` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.gmres_into` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 90.13953812462671 > 0.01 (mf-legacy-v2 van-der-pol-mu1000 uniform101)

### `kr.gmres_staged` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.lgmres` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.lgmres_into` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.accounting_recompute_final` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.accounting_reuse_confirmed` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.mf_warm_start` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.start_explicit_zero` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.workspace_options` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 98.1796735018913 > 0.01 (mf-lgmres-into van-der-pol-mu1000 uniform101)

### `kr.pc_direct` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.pc_jacobi` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.pc_none` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.pc_right` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.recycle_scope_currentw` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `kr.stage_target_coupled` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.stage_target_coupled_guarded` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.stage_target_coupled_guarded2` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, max_ratio, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.stage_target_dupfix` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 90.137965359347 > 0.01 (mf-stage-target-dupfix van-der-pol-mu1000 uniform101)

### `kr.stage_target_l2coupled` → OPTIMIZE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `OPTIMIZE_ALLOCATION` (alloc): PROMOTE fails only on allocation: worst per-attempt b 2.1379653593470036 > 0.01 (mf-stage-target-l2coupled van-der-pol-mu1000 uniform101)

### `kr.stage_target_legacy` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, allocation
3. `NOT_SPECIALIZE` (frontier): empty win set
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `kr.stage_target_projl2` → PRUNE

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NOT_PROMOTE` (frontier): failed: fraction, allocation
3. `NOT_SPECIALIZE` (frontier): holdout INCONCLUSIVE_INSUFFICIENT_POSITIVES, holdout wins recommended 0
4. `PRUNE_DEFAULT` (frontier): headroom: insufficient_evidence: no ATTRIBUTION.json; external gap: no external_peers in COMPONENTS

### `phi.binary_scaling` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `phi.complex_shift_jet` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `phi.fused` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `phi.krylov` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `phi.laguerre` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `phi.laguerre_adjoint` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `phi.leja` → OPTIMIZE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A04'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)
- blocked_by: A04

### `phi.pade13` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `phi.poly_router` → PRUNE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A08'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)
- blocked_by: A08

### `phi.shared_shift` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `phi.taylor` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `phi.taylor_total` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.bound_pr101` → PRUNE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A05'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)
- blocked_by: A05

### `cert.causal_majorant` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.certified_budget` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.directed` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.fourier_path` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.nonnormal` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.outward` → PRUNE (잠정)

1. `CORRECTNESS_OK` (correctness): C1 pass, C3 pass
2. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.raw_stage_target` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.residual_gain` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.stage_chart` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.stage_target` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `cert.transform_bound` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

### `pk.bound_collective` → OPTIMIZE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A01'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)
- blocked_by: A01

### `pk.collective_model` → OPTIMIZE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A01'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)
- blocked_by: A01

### `pk.curve` → OPTIMIZE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A01'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)
- blocked_by: A01

### `pk.damped_moments` → OPTIMIZE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A03'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)
- blocked_by: A03

### `pk.trajectory` → OPTIMIZE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A02'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)
- blocked_by: A02

### `pk.window` → OPTIMIZE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A01', 'A10'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)
- blocked_by: A01, A10

### `pk.work` → OPTIMIZE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability True -> OPTIMIZE (provisional, insufficient_evidence)

### `par.homotopy_parallel` → PRUNE (잠정)

1. `BLOCKED_BY` (contract): blocked_by ['A09'] -> provisional, PROMOTE forbidden
2. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
3. `CORRECTNESS_OK` (correctness): C1 null, C3 null
4. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)
- blocked_by: A09

### `par.pool` → PRUNE (잠정)

1. `C3_MISSING` (contract): insufficient_evidence: no PROBES rows for the component
2. `CORRECTNESS_OK` (correctness): C1 null, C3 null
3. `NO_EVIDENCE` (evidence): no usable comparison; unique_capability False -> PRUNE (provisional, insufficient_evidence)

## 증거 부족 (insufficient_evidence)

- `drv.exp_pexprb54s4`: no usable (non-excluded) comparison
- `drv.fast_banded`: C1: insufficient_evidence: no rows of the component's arms
- `drv.fast_banded`: C2: insufficient_evidence: no group with a computed R_up
- `drv.fast_banded`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `drv.fast_banded`: no usable (non-excluded) comparison
- `drv.fast_small_batch`: C1: insufficient_evidence: no rows of the component's arms
- `drv.fast_small_batch`: C2: insufficient_evidence: no group with a computed R_up
- `drv.fast_small_batch`: C3: insufficient_evidence: no PROBES rows for the component
- `drv.fast_small_batch`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `drv.fast_small_batch`: no usable (non-excluded) comparison
- `drv.seq_mf`: no usable (non-excluded) comparison
- `drv.transactional_q1q2`: no usable (non-excluded) comparison
- `scf.a1_two_arm_receipt`: C1: insufficient_evidence: no rows of the component's arms
- `scf.a1_two_arm_receipt`: C2: insufficient_evidence: no group with a computed R_up
- `scf.a1_two_arm_receipt`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.a1_two_arm_receipt`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.audit2_bateman_real_client_research`: C1: insufficient_evidence: no rows of the component's arms
- `scf.audit2_bateman_real_client_research`: C2: insufficient_evidence: no group with a computed R_up
- `scf.audit2_bateman_real_client_research`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.audit2_bateman_real_client_research`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.audit2_matrix_free_research`: C1: insufficient_evidence: no rows of the component's arms
- `scf.audit2_matrix_free_research`: C2: insufficient_evidence: no group with a computed R_up
- `scf.audit2_matrix_free_research`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.audit2_matrix_free_research`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.audit2_research`: C1: insufficient_evidence: no rows of the component's arms
- `scf.audit2_research`: C2: insufficient_evidence: no group with a computed R_up
- `scf.audit2_research`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.audit2_research`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.audit2_reusable_transaction_research`: C1: insufficient_evidence: no rows of the component's arms
- `scf.audit2_reusable_transaction_research`: C2: insufficient_evidence: no group with a computed R_up
- `scf.audit2_reusable_transaction_research`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.audit2_reusable_transaction_research`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.audit2_stage_certificate_research`: C1: insufficient_evidence: no rows of the component's arms
- `scf.audit2_stage_certificate_research`: C2: insufficient_evidence: no group with a computed R_up
- `scf.audit2_stage_certificate_research`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.audit2_stage_certificate_research`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.candidates`: C1: insufficient_evidence: no rows of the component's arms
- `scf.candidates`: C2: insufficient_evidence: no group with a computed R_up
- `scf.candidates`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.candidates`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.chart_transport`: C1: insufficient_evidence: no rows of the component's arms
- `scf.chart_transport`: C2: insufficient_evidence: no group with a computed R_up
- `scf.chart_transport`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.chart_transport`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.common_w_gate`: C1: insufficient_evidence: no rows of the component's arms
- `scf.common_w_gate`: C2: insufficient_evidence: no group with a computed R_up
- `scf.common_w_gate`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.common_w_gate`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.comparator_fidelity`: C1: insufficient_evidence: no rows of the component's arms
- `scf.comparator_fidelity`: C2: insufficient_evidence: no group with a computed R_up
- `scf.comparator_fidelity`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.comparator_fidelity`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.controller_telemetry`: C1: insufficient_evidence: no rows of the component's arms
- `scf.controller_telemetry`: C2: insufficient_evidence: no group with a computed R_up
- `scf.controller_telemetry`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.controller_telemetry`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.g1_transactional_gate`: C1: insufficient_evidence: no rows of the component's arms
- `scf.g1_transactional_gate`: C2: insufficient_evidence: no group with a computed R_up
- `scf.g1_transactional_gate`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.g1_transactional_gate`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.g2_exponential_gate`: C1: insufficient_evidence: no rows of the component's arms
- `scf.g2_exponential_gate`: C2: insufficient_evidence: no group with a computed R_up
- `scf.g2_exponential_gate`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.g2_exponential_gate`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.g3_fused_adaptive_gate`: C1: insufficient_evidence: no rows of the component's arms
- `scf.g3_fused_adaptive_gate`: C2: insufficient_evidence: no group with a computed R_up
- `scf.g3_fused_adaptive_gate`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.g3_fused_adaptive_gate`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.g4_prefix_kernel_gate`: C1: insufficient_evidence: no rows of the component's arms
- `scf.g4_prefix_kernel_gate`: C2: insufficient_evidence: no group with a computed R_up
- `scf.g4_prefix_kernel_gate`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.g4_prefix_kernel_gate`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.g4_s5b0_regime_atlas`: C1: insufficient_evidence: no rows of the component's arms
- `scf.g4_s5b0_regime_atlas`: C2: insufficient_evidence: no group with a computed R_up
- `scf.g4_s5b0_regime_atlas`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.g4_s5b0_regime_atlas`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.g4_s5b0_trace_authority`: C1: insufficient_evidence: no rows of the component's arms
- `scf.g4_s5b0_trace_authority`: C2: insufficient_evidence: no group with a computed R_up
- `scf.g4_s5b0_trace_authority`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.g4_s5b0_trace_authority`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.g4_s5b3_attempt_geometry`: C1: insufficient_evidence: no rows of the component's arms
- `scf.g4_s5b3_attempt_geometry`: C2: insufficient_evidence: no group with a computed R_up
- `scf.g4_s5b3_attempt_geometry`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.g4_s5b3_attempt_geometry`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.homotopy_experiments`: C1: insufficient_evidence: no rows of the component's arms
- `scf.homotopy_experiments`: C2: insufficient_evidence: no group with a computed R_up
- `scf.homotopy_experiments`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.homotopy_experiments`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.homotopy_order_policy`: C1: insufficient_evidence: no rows of the component's arms
- `scf.homotopy_order_policy`: C2: insufficient_evidence: no group with a computed R_up
- `scf.homotopy_order_policy`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.homotopy_order_policy`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.native_gates`: C1: insufficient_evidence: no rows of the component's arms
- `scf.native_gates`: C2: insufficient_evidence: no group with a computed R_up
- `scf.native_gates`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.native_gates`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.path_controller`: C1: insufficient_evidence: no rows of the component's arms
- `scf.path_controller`: C2: insufficient_evidence: no group with a computed R_up
- `scf.path_controller`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.path_controller`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.policy_redesign_v25`: C1: insufficient_evidence: no rows of the component's arms
- `scf.policy_redesign_v25`: C2: insufficient_evidence: no group with a computed R_up
- `scf.policy_redesign_v25`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.policy_redesign_v25`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.rhs_telemetry`: C1: insufficient_evidence: no rows of the component's arms
- `scf.rhs_telemetry`: C2: insufficient_evidence: no group with a computed R_up
- `scf.rhs_telemetry`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.rhs_telemetry`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.scientific_validity_v2_gate`: C1: insufficient_evidence: no rows of the component's arms
- `scf.scientific_validity_v2_gate`: C2: insufficient_evidence: no group with a computed R_up
- `scf.scientific_validity_v2_gate`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.scientific_validity_v2_gate`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.unified_gates`: C1: insufficient_evidence: no rows of the component's arms
- `scf.unified_gates`: C2: insufficient_evidence: no group with a computed R_up
- `scf.unified_gates`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.unified_gates`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.unified_screen`: C1: insufficient_evidence: no rows of the component's arms
- `scf.unified_screen`: C2: insufficient_evidence: no group with a computed R_up
- `scf.unified_screen`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.unified_screen`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `scf.v38d_performance_tournament`: C1: insufficient_evidence: no rows of the component's arms
- `scf.v38d_performance_tournament`: C2: insufficient_evidence: no group with a computed R_up
- `scf.v38d_performance_tournament`: C3: insufficient_evidence: no PROBES rows for the component
- `scf.v38d_performance_tournament`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `corp.gray_scott`: C1: insufficient_evidence: no rows of the component's arms
- `corp.gray_scott`: C2: insufficient_evidence: no group with a computed R_up
- `corp.gray_scott`: C3: insufficient_evidence: no PROBES rows for the component
- `corp.gray_scott`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `corp.problems`: C1: insufficient_evidence: no rows of the component's arms
- `corp.problems`: C2: insufficient_evidence: no group with a computed R_up
- `corp.problems`: C3: insufficient_evidence: no PROBES rows for the component
- `corp.problems`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `corp.scientific_corpus_v2`: C1: insufficient_evidence: no rows of the component's arms
- `corp.scientific_corpus_v2`: C2: insufficient_evidence: no group with a computed R_up
- `corp.scientific_corpus_v2`: C3: insufficient_evidence: no PROBES rows for the component
- `corp.scientific_corpus_v2`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `out.verified_pr101`: C1: insufficient_evidence: no rows of the component's arms
- `out.verified_pr101`: C2: insufficient_evidence: no group with a computed R_up
- `out.verified_pr101`: C3: insufficient_evidence: no PROBES rows for the component
- `out.verified_pr101`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `out.verified_pr101`: no usable (non-excluded) comparison
- `la.banded_indexed`: C1: insufficient_evidence: no rows of the component's arms
- `la.banded_indexed`: C2: insufficient_evidence: no group with a computed R_up
- `la.banded_indexed`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `la.banded_indexed`: no usable (non-excluded) comparison
- `la.banded_slices`: C1: insufficient_evidence: no rows of the component's arms
- `la.banded_slices`: C2: insufficient_evidence: no group with a computed R_up
- `la.banded_slices`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `la.banded_slices`: no usable (non-excluded) comparison
- `la.lu_colext64`: no usable (non-excluded) comparison
- `la.sparse_density_switch`: no usable (non-excluded) comparison
- `la.sparse_direct`: C1: insufficient_evidence: no rows of the component's arms
- `la.sparse_direct`: C2: insufficient_evidence: no group with a computed R_up
- `la.sparse_direct`: C3: insufficient_evidence: no PROBES rows for the component
- `la.sparse_direct`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `la.sparse_direct`: no usable (non-excluded) comparison
- `la.stage_batch`: C1: insufficient_evidence: no rows of the component's arms
- `la.stage_batch`: C2: insufficient_evidence: no group with a computed R_up
- `la.stage_batch`: C3: insufficient_evidence: no PROBES rows for the component
- `la.stage_batch`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `la.stage_batch`: no usable (non-excluded) comparison
- `la.woodbury_collective`: C1: insufficient_evidence: no rows of the component's arms
- `la.woodbury_collective`: C2: insufficient_evidence: no group with a computed R_up
- `la.woodbury_collective`: C3: insufficient_evidence: no PROBES rows for the component
- `la.woodbury_collective`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `la.woodbury_collective`: no usable (non-excluded) comparison
- `kr.recycle_cold`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `kr.recycle_cold`: no usable (non-excluded) comparison
- `kr.recycle_legacy`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `kr.recycle_legacy`: no usable (non-excluded) comparison
- `kr.recycle_refresh_after_update`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `kr.recycle_refresh_after_update`: no usable (non-excluded) comparison
- `kr.x0_prevstep`: C1: insufficient_evidence: no rows of the component's arms
- `kr.x0_prevstep`: C2: insufficient_evidence: no group with a computed R_up
- `kr.x0_prevstep`: C3: insufficient_evidence: no PROBES rows for the component
- `kr.x0_prevstep`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `kr.x0_prevstep`: no usable (non-excluded) comparison
- `kr.x0_prevstep_scaled`: C1: insufficient_evidence: no rows of the component's arms
- `kr.x0_prevstep_scaled`: C2: insufficient_evidence: no group with a computed R_up
- `kr.x0_prevstep_scaled`: C3: insufficient_evidence: no PROBES rows for the component
- `kr.x0_prevstep_scaled`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `kr.x0_prevstep_scaled`: no usable (non-excluded) comparison
- `kr.block_gmres`: C1: insufficient_evidence: no rows of the component's arms
- `kr.block_gmres`: C2: insufficient_evidence: no group with a computed R_up
- `kr.block_gmres`: C3: insufficient_evidence: no PROBES rows for the component
- `kr.block_gmres`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `kr.block_gmres`: no usable (non-excluded) comparison
- `kr.gmres_givens`: C1: insufficient_evidence: no rows of the component's arms
- `kr.gmres_givens`: C2: insufficient_evidence: no group with a computed R_up
- `kr.gmres_givens`: C3: insufficient_evidence: no PROBES rows for the component
- `kr.gmres_givens`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `kr.gmres_givens`: no usable (non-excluded) comparison
- `kr.gmres_staged`: no usable (non-excluded) comparison
- `kr.lgmres_into`: no usable (non-excluded) comparison
- `kr.mf_warm_start`: no usable (non-excluded) comparison
- `kr.start_explicit_zero`: no usable (non-excluded) comparison
- `kr.pc_right`: C1: insufficient_evidence: no rows of the component's arms
- `kr.pc_right`: C2: insufficient_evidence: no group with a computed R_up
- `kr.pc_right`: C3: insufficient_evidence: no PROBES rows for the component
- `kr.pc_right`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `kr.pc_right`: no usable (non-excluded) comparison
- `kr.recycle_scope_currentw`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `kr.recycle_scope_currentw`: no usable (non-excluded) comparison
- `phi.binary_scaling`: C1: insufficient_evidence: no rows of the component's arms
- `phi.binary_scaling`: C2: insufficient_evidence: no group with a computed R_up
- `phi.binary_scaling`: C3: insufficient_evidence: no PROBES rows for the component
- `phi.binary_scaling`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `phi.binary_scaling`: no usable (non-excluded) comparison
- `phi.complex_shift_jet`: C1: insufficient_evidence: no rows of the component's arms
- `phi.complex_shift_jet`: C2: insufficient_evidence: no group with a computed R_up
- `phi.complex_shift_jet`: C3: insufficient_evidence: no PROBES rows for the component
- `phi.complex_shift_jet`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `phi.complex_shift_jet`: no usable (non-excluded) comparison
- `phi.fused`: no usable (non-excluded) comparison
- `phi.krylov`: C1: insufficient_evidence: no rows of the component's arms
- `phi.krylov`: C2: insufficient_evidence: no group with a computed R_up
- `phi.krylov`: C3: insufficient_evidence: no PROBES rows for the component
- `phi.krylov`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `phi.krylov`: no usable (non-excluded) comparison
- `phi.laguerre`: C1: insufficient_evidence: no rows of the component's arms
- `phi.laguerre`: C2: insufficient_evidence: no group with a computed R_up
- `phi.laguerre`: C3: insufficient_evidence: no PROBES rows for the component
- `phi.laguerre`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `phi.laguerre`: no usable (non-excluded) comparison
- `phi.laguerre_adjoint`: C1: insufficient_evidence: no rows of the component's arms
- `phi.laguerre_adjoint`: C2: insufficient_evidence: no group with a computed R_up
- `phi.laguerre_adjoint`: C3: insufficient_evidence: no PROBES rows for the component
- `phi.laguerre_adjoint`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `phi.laguerre_adjoint`: no usable (non-excluded) comparison
- `phi.leja`: C1: insufficient_evidence: no rows of the component's arms
- `phi.leja`: C2: insufficient_evidence: no group with a computed R_up
- `phi.leja`: C3: insufficient_evidence: no PROBES rows for the component
- `phi.leja`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `phi.leja`: no usable (non-excluded) comparison
- `phi.pade13`: no usable (non-excluded) comparison
- `phi.poly_router`: C1: insufficient_evidence: no rows of the component's arms
- `phi.poly_router`: C2: insufficient_evidence: no group with a computed R_up
- `phi.poly_router`: C3: insufficient_evidence: no PROBES rows for the component
- `phi.poly_router`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `phi.poly_router`: no usable (non-excluded) comparison
- `phi.shared_shift`: C1: insufficient_evidence: no rows of the component's arms
- `phi.shared_shift`: C2: insufficient_evidence: no group with a computed R_up
- `phi.shared_shift`: C3: insufficient_evidence: no PROBES rows for the component
- `phi.shared_shift`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `phi.shared_shift`: no usable (non-excluded) comparison
- `phi.taylor`: C1: insufficient_evidence: no rows of the component's arms
- `phi.taylor`: C2: insufficient_evidence: no group with a computed R_up
- `phi.taylor`: C3: insufficient_evidence: no PROBES rows for the component
- `phi.taylor`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `phi.taylor`: no usable (non-excluded) comparison
- `phi.taylor_total`: C1: insufficient_evidence: no rows of the component's arms
- `phi.taylor_total`: C2: insufficient_evidence: no group with a computed R_up
- `phi.taylor_total`: C3: insufficient_evidence: no PROBES rows for the component
- `phi.taylor_total`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `phi.taylor_total`: no usable (non-excluded) comparison
- `cert.bound_pr101`: C1: insufficient_evidence: no rows of the component's arms
- `cert.bound_pr101`: C2: insufficient_evidence: no group with a computed R_up
- `cert.bound_pr101`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.bound_pr101`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.bound_pr101`: no usable (non-excluded) comparison
- `cert.causal_majorant`: C1: insufficient_evidence: no rows of the component's arms
- `cert.causal_majorant`: C2: insufficient_evidence: no group with a computed R_up
- `cert.causal_majorant`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.causal_majorant`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.causal_majorant`: no usable (non-excluded) comparison
- `cert.certified_budget`: C1: insufficient_evidence: no rows of the component's arms
- `cert.certified_budget`: C2: insufficient_evidence: no group with a computed R_up
- `cert.certified_budget`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.certified_budget`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.certified_budget`: no usable (non-excluded) comparison
- `cert.directed`: C1: insufficient_evidence: no rows of the component's arms
- `cert.directed`: C2: insufficient_evidence: no group with a computed R_up
- `cert.directed`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.directed`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.directed`: no usable (non-excluded) comparison
- `cert.fourier_path`: C1: insufficient_evidence: no rows of the component's arms
- `cert.fourier_path`: C2: insufficient_evidence: no group with a computed R_up
- `cert.fourier_path`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.fourier_path`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.fourier_path`: no usable (non-excluded) comparison
- `cert.nonnormal`: C1: insufficient_evidence: no rows of the component's arms
- `cert.nonnormal`: C2: insufficient_evidence: no group with a computed R_up
- `cert.nonnormal`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.nonnormal`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.nonnormal`: no usable (non-excluded) comparison
- `cert.outward`: no usable (non-excluded) comparison
- `cert.raw_stage_target`: C1: insufficient_evidence: no rows of the component's arms
- `cert.raw_stage_target`: C2: insufficient_evidence: no group with a computed R_up
- `cert.raw_stage_target`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.raw_stage_target`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.raw_stage_target`: no usable (non-excluded) comparison
- `cert.residual_gain`: C1: insufficient_evidence: no rows of the component's arms
- `cert.residual_gain`: C2: insufficient_evidence: no group with a computed R_up
- `cert.residual_gain`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.residual_gain`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.residual_gain`: no usable (non-excluded) comparison
- `cert.stage_chart`: C1: insufficient_evidence: no rows of the component's arms
- `cert.stage_chart`: C2: insufficient_evidence: no group with a computed R_up
- `cert.stage_chart`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.stage_chart`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.stage_chart`: no usable (non-excluded) comparison
- `cert.stage_target`: C1: insufficient_evidence: no rows of the component's arms
- `cert.stage_target`: C2: insufficient_evidence: no group with a computed R_up
- `cert.stage_target`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.stage_target`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.stage_target`: no usable (non-excluded) comparison
- `cert.transform_bound`: C1: insufficient_evidence: no rows of the component's arms
- `cert.transform_bound`: C2: insufficient_evidence: no group with a computed R_up
- `cert.transform_bound`: C3: insufficient_evidence: no PROBES rows for the component
- `cert.transform_bound`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `cert.transform_bound`: no usable (non-excluded) comparison
- `pk.bound_collective`: C1: insufficient_evidence: no rows of the component's arms
- `pk.bound_collective`: C2: insufficient_evidence: no group with a computed R_up
- `pk.bound_collective`: C3: insufficient_evidence: no PROBES rows for the component
- `pk.bound_collective`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `pk.bound_collective`: no usable (non-excluded) comparison
- `pk.collective_model`: C1: insufficient_evidence: no rows of the component's arms
- `pk.collective_model`: C2: insufficient_evidence: no group with a computed R_up
- `pk.collective_model`: C3: insufficient_evidence: no PROBES rows for the component
- `pk.collective_model`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `pk.collective_model`: no usable (non-excluded) comparison
- `pk.curve`: C1: insufficient_evidence: no rows of the component's arms
- `pk.curve`: C2: insufficient_evidence: no group with a computed R_up
- `pk.curve`: C3: insufficient_evidence: no PROBES rows for the component
- `pk.curve`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `pk.curve`: no usable (non-excluded) comparison
- `pk.damped_moments`: C1: insufficient_evidence: no rows of the component's arms
- `pk.damped_moments`: C2: insufficient_evidence: no group with a computed R_up
- `pk.damped_moments`: C3: insufficient_evidence: no PROBES rows for the component
- `pk.damped_moments`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `pk.damped_moments`: no usable (non-excluded) comparison
- `pk.trajectory`: C1: insufficient_evidence: no rows of the component's arms
- `pk.trajectory`: C2: insufficient_evidence: no group with a computed R_up
- `pk.trajectory`: C3: insufficient_evidence: no PROBES rows for the component
- `pk.trajectory`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `pk.trajectory`: no usable (non-excluded) comparison
- `pk.window`: C1: insufficient_evidence: no rows of the component's arms
- `pk.window`: C2: insufficient_evidence: no group with a computed R_up
- `pk.window`: C3: insufficient_evidence: no PROBES rows for the component
- `pk.window`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `pk.window`: no usable (non-excluded) comparison
- `pk.work`: C1: insufficient_evidence: no rows of the component's arms
- `pk.work`: C2: insufficient_evidence: no group with a computed R_up
- `pk.work`: C3: insufficient_evidence: no PROBES rows for the component
- `pk.work`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `pk.work`: no usable (non-excluded) comparison
- `par.homotopy_parallel`: C1: insufficient_evidence: no rows of the component's arms
- `par.homotopy_parallel`: C2: insufficient_evidence: no group with a computed R_up
- `par.homotopy_parallel`: C3: insufficient_evidence: no PROBES rows for the component
- `par.homotopy_parallel`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `par.homotopy_parallel`: no usable (non-excluded) comparison
- `par.pool`: C1: insufficient_evidence: no rows of the component's arms
- `par.pool`: C2: insufficient_evidence: no group with a computed R_up
- `par.pool`: C3: insufficient_evidence: no PROBES rows for the component
- `par.pool`: allocation: insufficient_evidence: no h0 = 1 ALLOCATIONS rows for the component's arms
- `par.pool`: no usable (non-excluded) comparison

## 외부 피어 counted-work 격차 (OPTIMIZE b)

| 컴포넌트 | 외부 피어 | 일치 셀 | 전체 셀 | 중앙 격차 | 사유 |
|---|---|---|---|---|---|
| `drv.bdf` | ext.cvode, ext.scipy-bdf | 0 | 0 | — | insufficient_evidence: no matched cell with complete counted work on both sides |
| `drv.fast` | ext.hairer-rodas | 0 | 0 | — | insufficient_evidence: no matched cell with complete counted work on both sides |
| `drv.fast_small` | ext.hairer-rodas | 0 | 0 | — | insufficient_evidence: no matched cell with complete counted work on both sides |
| `drv.mf_fast` | ext.cvode | 0 | 0 | — | insufficient_evidence: no matched cell with complete counted work on both sides |
| `drv.radau` | ext.hairer-radau5, ext.scipy-radau | 0 | 0 | — | insufficient_evidence: no matched cell with complete counted work on both sides |
| `drv.seq_direct` | ext.hairer-rodas | 0 | 0 | — | insufficient_evidence: no matched cell with complete counted work on both sides |
| `drv.seq_mf` | ext.cvode | 0 | 0 | — | insufficient_evidence: no matched cell with complete counted work on both sides |

일치한 외부 격차 셀 0개 / 전체 0개.

## 제외된 비교의 사유

| 사유 | 개수 |
|---|---|
| insufficient_evidence | 16074 |
| reference | 53712 |
| reference_limited | 427 |

## 비교 집계

| 결과 | 개수 |
|---|---|
| GAIN | 1817 |
| NEUTRAL | 1036 |
| LOSS | 1539 |
| capability_gain | 2803 |
| capability_loss | 0 |
| excluded | 70213 |

행 6201개, 그룹 2067개, 비교 77408개, 다른 캠페인 행 0개 무시.
