# 달성 오차 정합·banded 대조 확인 결과

측정 source: `9c6a6d9145795e35b978652a567b806ddc84a6bd`

모든 시간은 이 새 runner에서 실행했다. 이전 호스트의 시간 표본을 합치지 않았다. 모든 h0 배율의 오차를 anchor 아래로 제한하고 nominal h0에서 실제 오차를 [anchor/2, anchor] 안에 맞춘 구성만 시간 비교했다.

## 실행 요약

```json
{
  "calibration_groups": 90,
  "calibration_solves": 270,
  "complete_pairs": 42,
  "old_timing_pooled": false,
  "paired_comparisons": 42,
  "production_promotion": false,
  "raw_state_compressed_bytes": 7540513,
  "raw_state_objects": 189,
  "reference_source_run": "39257390707",
  "selection_groups": 10,
  "selection_status": {
    "MATCHED_ALL_ARMS": 10
  },
  "source_commit": "9c6a6d9145795e35b978652a567b806ddc84a6bd",
  "timed_integrations": 4896,
  "timing_processes": 408,
  "valid_timing_processes": 408,
  "wall_clock_authority": "diagnostic_shared_runner"
}
```

## 공통 달성 오차에서의 시간

|문제|출력|Arm|입력 rtol|실제 E_up|중앙시간 ms|
|---|---|---|---:|---:|---:|
|robertson|endpoint|diffsol-bdf|4.216965e-07|2.2387e-07|0.108376|
|robertson|endpoint|netlib-dvode|1e-07|2.7347e-07|0.100469|
|robertson|endpoint|vigilode-fast|3.162278e-07|2.143e-07|0.037920|
|robertson|uniform101|diffsol-bdf|1e-07|2.0143e-07|0.116443|
|robertson|uniform101|netlib-dvode|3.162278e-08|1.8387e-07|0.117109|
|robertson|uniform101|vigilode-fast|3.162278e-06|1.0751e-07|0.089863|
|hires|endpoint|diffsol-bdf|3.162278e-08|1.8113e-07|0.536127|
|hires|endpoint|netlib-dvode|8.058422e-09|9.3418e-08|0.479670|
|hires|endpoint|vigilode-fast|1e-06|1.3161e-07|0.281925|
|hires|uniform101|diffsol-bdf|1e-08|8.1828e-08|0.590071|
|hires|uniform101|netlib-dvode|5.623413e-09|9.8846e-08|0.533075|
|hires|uniform101|vigilode-fast|1e-06|1.4067e-07|0.373237|
|van-der-pol-mu1000|endpoint|diffsol-bdf|3.162278e-09|2.4146e-07|0.983506|
|van-der-pol-mu1000|endpoint|netlib-dvode|2.371374e-09|2.6389e-07|0.753888|
|van-der-pol-mu1000|endpoint|vigilode-fast|1e-06|3.3857e-07|0.265004|
|van-der-pol-mu1000|uniform101|diffsol-bdf|1e-09|3.156e-07|1.152068|
|van-der-pol-mu1000|uniform101|netlib-dvode|3.162278e-10|3.1176e-07|0.933415|
|van-der-pol-mu1000|uniform101|vigilode-fast|3.162278e-07|2.1512e-07|0.331527|
|brusselator-1d-50|endpoint|diffsol-bdf|3.162278e-08|2.2706e-07|5.156916|
|brusselator-1d-50|endpoint|netlib-dvode|1e-07|4.0498e-07|0.943856|
|brusselator-1d-50|endpoint|vigilode-banded|3.162278e-07|2.8595e-07|1.999168|
|brusselator-1d-50|endpoint|vigilode-fast|3.162278e-07|2.8595e-07|2.819666|
|brusselator-1d-50|uniform101|diffsol-bdf|1e-08|4.6622e-07|5.424569|
|brusselator-1d-50|uniform101|netlib-dvode|3.162278e-09|3.3395e-07|1.562516|
|brusselator-1d-50|uniform101|vigilode-banded|3.162278e-07|2.6574e-07|2.759208|
|brusselator-1d-50|uniform101|vigilode-fast|3.162278e-07|2.6574e-07|3.883919|
|brusselator-1d-200|endpoint|diffsol-bdf|3.162278e-08|2.3152e-07|141.289589|
|brusselator-1d-200|endpoint|netlib-dvode|1e-07|4.2712e-07|3.610742|
|brusselator-1d-200|endpoint|vigilode-banded|3.162278e-07|3.0586e-07|7.900871|
|brusselator-1d-200|endpoint|vigilode-fast|3.162278e-07|3.0586e-07|27.411388|
|brusselator-1d-200|uniform101|diffsol-bdf|1e-08|4.476e-07|131.609302|
|brusselator-1d-200|uniform101|netlib-dvode|3.162278e-09|2.9042e-07|6.154194|
|brusselator-1d-200|uniform101|vigilode-banded|1e-07|2.3033e-07|12.830879|
|brusselator-1d-200|uniform101|vigilode-fast|1e-07|2.3033e-07|44.820667|

## Pairwise 비율

비율은 baseline/candidate. 1보다 크면 candidate가 짧다. 구간은 42개 사전 지정 비교를 고려한 Bonferroni bootstrap 구간이며 공유호스트·12블록이라는 제한은 여전히 남는다.

|문제|출력|Baseline|Candidate|비율|family 구간|
|---|---|---|---|---:|---|
|robertson|endpoint|diffsol-bdf|vigilode-fast|2.8575|2.8460, 2.8740|
|robertson|endpoint|netlib-dvode|vigilode-fast|2.6487|2.6327, 2.6571|
|robertson|endpoint|netlib-dvode|diffsol-bdf|0.9262|0.9206, 0.9296|
|robertson|uniform101|diffsol-bdf|vigilode-fast|1.2997|1.2863, 1.3049|
|robertson|uniform101|netlib-dvode|vigilode-fast|1.3023|1.2996, 1.3122|
|robertson|uniform101|netlib-dvode|diffsol-bdf|1.0065|0.9959, 1.0147|
|hires|endpoint|diffsol-bdf|vigilode-fast|1.9005|1.8929, 1.9195|
|hires|endpoint|netlib-dvode|vigilode-fast|1.7007|1.6929, 1.7112|
|hires|endpoint|netlib-dvode|diffsol-bdf|0.8973|0.8814, 0.8997|
|hires|uniform101|diffsol-bdf|vigilode-fast|1.5814|1.5726, 1.5955|
|hires|uniform101|netlib-dvode|vigilode-fast|1.4280|1.4246, 1.4405|
|hires|uniform101|netlib-dvode|diffsol-bdf|0.9048|0.8929, 0.9091|
|van-der-pol-mu1000|endpoint|diffsol-bdf|vigilode-fast|3.7138|3.6845, 3.7244|
|van-der-pol-mu1000|endpoint|netlib-dvode|vigilode-fast|2.8462|2.8342, 2.8576|
|van-der-pol-mu1000|endpoint|netlib-dvode|diffsol-bdf|0.7669|0.7609, 0.7721|
|van-der-pol-mu1000|uniform101|diffsol-bdf|vigilode-fast|3.4799|3.4554, 3.5156|
|van-der-pol-mu1000|uniform101|netlib-dvode|vigilode-fast|2.8172|2.8010, 2.8240|
|van-der-pol-mu1000|uniform101|netlib-dvode|diffsol-bdf|0.8094|0.8048, 0.8133|
|brusselator-1d-50|endpoint|diffsol-bdf|vigilode-fast|1.8158|1.7967, 1.8524|
|brusselator-1d-50|endpoint|netlib-dvode|vigilode-fast|0.3327|0.3285, 0.3389|
|brusselator-1d-50|endpoint|vigilode-banded|vigilode-fast|0.7072|0.6955, 0.7169|
|brusselator-1d-50|endpoint|netlib-dvode|diffsol-bdf|0.1834|0.1803, 0.1848|
|brusselator-1d-50|endpoint|vigilode-banded|diffsol-bdf|0.3884|0.3838, 0.3925|
|brusselator-1d-50|endpoint|vigilode-banded|netlib-dvode|2.1278|2.1014, 2.1445|
|brusselator-1d-50|uniform101|diffsol-bdf|vigilode-fast|1.3983|1.3552, 1.4068|
|brusselator-1d-50|uniform101|netlib-dvode|vigilode-fast|0.4012|0.3890, 0.4054|
|brusselator-1d-50|uniform101|vigilode-banded|vigilode-fast|0.7087|0.7036, 0.7172|
|brusselator-1d-50|uniform101|netlib-dvode|diffsol-bdf|0.2876|0.2860, 0.2896|
|brusselator-1d-50|uniform101|vigilode-banded|diffsol-bdf|0.5092|0.5039, 0.5161|
|brusselator-1d-50|uniform101|vigilode-banded|netlib-dvode|1.7676|1.7525, 1.7880|
|brusselator-1d-200|endpoint|diffsol-bdf|vigilode-fast|5.1494|5.0509, 5.1778|
|brusselator-1d-200|endpoint|netlib-dvode|vigilode-fast|0.1316|0.1297, 0.1322|
|brusselator-1d-200|endpoint|vigilode-banded|vigilode-fast|0.2879|0.2843, 0.2894|
|brusselator-1d-200|endpoint|netlib-dvode|diffsol-bdf|0.0255|0.0255, 0.0259|
|brusselator-1d-200|endpoint|vigilode-banded|diffsol-bdf|0.0559|0.0556, 0.0567|
|brusselator-1d-200|endpoint|vigilode-banded|netlib-dvode|2.1886|2.1821, 2.1956|
|brusselator-1d-200|uniform101|diffsol-bdf|vigilode-fast|2.9364|2.9105, 2.9589|
|brusselator-1d-200|uniform101|netlib-dvode|vigilode-fast|0.1373|0.1362, 0.1385|
|brusselator-1d-200|uniform101|vigilode-banded|vigilode-fast|0.2867|0.2838, 0.2877|
|brusselator-1d-200|uniform101|netlib-dvode|diffsol-bdf|0.0467|0.0466, 0.0469|
|brusselator-1d-200|uniform101|vigilode-banded|diffsol-bdf|0.0975|0.0970, 0.0980|
|brusselator-1d-200|uniform101|vigilode-banded|netlib-dvode|2.0831|2.0768, 2.0931|

## 읽기 검산

```json
{
  "status": "PASS_SCOPED_NUMERICAL_READBACK",
  "checked": {
    "calibration": 270,
    "finite_complete_ok": 678,
    "timing": 408,
    "paired_median_recomputed": 42
  },
  "unique_state_objects_verified": 189,
  "max_conservation_residual_robertson_or_hires": 4.218847493575595e-15,
  "new_timing_points_only": true,
  "independence": "separate vectorized computation; same agent authorship, not independent personnel review",
  "unverified": "bootstrap coverage under correlated host noise; continuous-time certified enclosure; full workspace regression suite",
  "legacy_archival_gap": "Run 39257390707 omitted two >20MiB raw gzip files; its CSV and block medians remain. This run retains content-addressed full states and raw per-repeat times."
}
```

Netlib DVODE banded와 VigilODE banded의 비교가 구조 정합 비교다. Diffsol은 FaerLU dense 구성이므로 해당 차이를 방법 자체의 순위로 확대하지 않는다. local-error tolerance와 실제 전방/global error의 구분, 경험적 참조 폭, 실패·미완료의 독립 보존 원칙을 유지한다.

## 초기값 비트 검사에 대한 정정

원래 auditor의 초기값 bit equality 검사는 실패한 기록 그대로 보존한다. 이 후처리에서는 초기 출력 오차를 숨기지 않고 E_up/R_up에 그대로 포함한 채, 별도 16 epsilon floating consistency 검사를 추가했다. 실제 차이와 영향은 INITIAL_VALUE_SUMMARY.json에 기록했다. 원자료·선택·측정 시간을 바꾸거나 적분을 재실행하지 않았다. 최종 readback 상태는 AUDIT.json이 우선한다.
