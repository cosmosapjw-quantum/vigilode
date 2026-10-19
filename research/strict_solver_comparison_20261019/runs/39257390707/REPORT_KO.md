# 동일 호스트 VigilODE / Diffsol / Netlib DVODE 비교

이 보고서는 저장된 새 원자료에서 생성했다. 공유 GitHub runner의 시간은 diagnostic이며 생산 성능 승격 근거로 사용하지 않는다.

## 실행 및 정확도

```json
{
  "status": "COMPLETED_BOUNDED_CAMPAIGN",
  "planned_accuracy_cells": 756,
  "retained_accuracy_cells": 756,
  "status_counts": {
    "ok": 756
  },
  "structurally_admitted": 756,
  "R_up_le_1": 262,
  "R_up_le_10": 543,
  "arm_accuracy": {
    "vigilode-fast": {
      "cells": 252,
      "admitted": 252,
      "R_up_le_1": 207
    },
    "diffsol-bdf": {
      "cells": 252,
      "admitted": 252,
      "R_up_le_1": 39
    },
    "netlib-dvode": {
      "cells": 252,
      "admitted": 252,
      "R_up_le_1": 16
    }
  },
  "selection_status": {
    "READY": 32,
    "HOLD_MISSING_ACCURACY_ARM": 4
  },
  "timing_processes": 1152,
  "valid_timing_processes": 1152,
  "complete_paired_comparisons": 96,
  "timing_authority": "diagnostic_shared_runner",
  "global_error_guarantee": false,
  "production_promotion": false
}
```

## 정확도 목표별 paired 시간비

값은 baseline/candidate이며 1보다 크면 candidate의 측정 시간이 짧다. 실제 오차비가 2를 초과하는 경우 same-target 조건일 뿐 matched-achieved 비교가 아니다. 95% 구간은 pointwise이며 다중비교 보정되지 않았다.

|문제|격자|목표|Baseline|Candidate|시간비|95% 구간|정확도 정합성|
|---|---|---:|---|---|---:|---|---|
|robertson|endpoint|1e-04|netlib-dvode|vigilode-fast|2.775|2.771, 2.785|same_target_only|
|robertson|endpoint|1e-04|diffsol-bdf|vigilode-fast|3.118|3.111, 3.123|same_target_only|
|robertson|endpoint|1e-04|netlib-dvode|diffsol-bdf|0.893|0.890, 0.895|same_target_only|
|robertson|endpoint|1e-06|netlib-dvode|vigilode-fast|2.931|2.915, 2.955|same_target_only|
|robertson|endpoint|1e-06|diffsol-bdf|vigilode-fast|2.148|2.144, 2.153|same_target_only|
|robertson|endpoint|1e-06|netlib-dvode|diffsol-bdf|1.365|1.360, 1.371|same_target_only|
|robertson|endpoint|1e-08|netlib-dvode|vigilode-fast|2.354|2.351, 2.360|matched_achieved_factor2|
|robertson|endpoint|1e-08|diffsol-bdf|vigilode-fast|1.535|1.532, 1.541|matched_achieved_factor2|
|robertson|endpoint|1e-08|netlib-dvode|diffsol-bdf|1.533|1.527, 1.540|matched_achieved_factor2|
|robertson|uniform101|1e-04|netlib-dvode|vigilode-fast|0.706|0.704, 0.708|same_target_only|
|robertson|uniform101|1e-04|diffsol-bdf|vigilode-fast|0.811|0.809, 0.811|same_target_only|
|robertson|uniform101|1e-04|netlib-dvode|diffsol-bdf|0.870|0.869, 0.874|same_target_only|
|robertson|uniform101|1e-06|netlib-dvode|vigilode-fast|1.202|1.197, 1.204|same_target_only|
|robertson|uniform101|1e-06|diffsol-bdf|vigilode-fast|1.414|1.412, 1.419|same_target_only|
|robertson|uniform101|1e-06|netlib-dvode|diffsol-bdf|0.848|0.846, 0.851|same_target_only|
|robertson|uniform101|1e-08|netlib-dvode|vigilode-fast|1.935|1.933, 1.939|same_target_only|
|robertson|uniform101|1e-08|diffsol-bdf|vigilode-fast|1.699|1.694, 1.703|same_target_only|
|robertson|uniform101|1e-08|netlib-dvode|diffsol-bdf|1.139|1.136, 1.144|same_target_only|
|hires|endpoint|1e-04|netlib-dvode|vigilode-fast|1.468|1.465, 1.471|same_target_only|
|hires|endpoint|1e-04|diffsol-bdf|vigilode-fast|2.408|2.400, 2.414|same_target_only|
|hires|endpoint|1e-04|netlib-dvode|diffsol-bdf|0.610|0.608, 0.612|same_target_only|
|hires|endpoint|1e-06|netlib-dvode|vigilode-fast|1.789|1.784, 1.791|same_target_only|
|hires|endpoint|1e-06|diffsol-bdf|vigilode-fast|1.626|1.622, 1.629|same_target_only|
|hires|endpoint|1e-06|netlib-dvode|diffsol-bdf|1.099|1.092, 1.104|same_target_only|
|hires|endpoint|1e-08|netlib-dvode|vigilode-fast|1.410|1.407, 1.415|same_target_only|
|hires|endpoint|1e-08|diffsol-bdf|vigilode-fast|1.305|1.302, 1.312|same_target_only|
|hires|endpoint|1e-08|netlib-dvode|diffsol-bdf|1.080|1.078, 1.082|same_target_only|
|hires|uniform101|1e-04|netlib-dvode|vigilode-fast|0.838|0.836, 0.839|same_target_only|
|hires|uniform101|1e-04|diffsol-bdf|vigilode-fast|1.380|1.373, 1.384|same_target_only|
|hires|uniform101|1e-04|netlib-dvode|diffsol-bdf|0.608|0.606, 0.610|same_target_only|
|hires|uniform101|1e-06|netlib-dvode|vigilode-fast|1.375|1.370, 1.378|same_target_only|
|hires|uniform101|1e-06|diffsol-bdf|vigilode-fast|1.255|1.252, 1.263|same_target_only|
|hires|uniform101|1e-06|netlib-dvode|diffsol-bdf|1.095|1.093, 1.096|same_target_only|
|hires|uniform101|1e-08|netlib-dvode|vigilode-fast|0.691|0.688, 0.696|same_target_only|
|hires|uniform101|1e-08|diffsol-bdf|vigilode-fast|0.809|0.807, 0.814|same_target_only|
|hires|uniform101|1e-08|netlib-dvode|diffsol-bdf|0.853|0.851, 0.855|same_target_only|
|van-der-pol-mu1000|endpoint|1e-04|netlib-dvode|vigilode-fast|1.846|1.833, 1.855|same_target_only|
|van-der-pol-mu1000|endpoint|1e-04|diffsol-bdf|vigilode-fast|2.242|2.224, 2.246|same_target_only|
|van-der-pol-mu1000|endpoint|1e-04|netlib-dvode|diffsol-bdf|0.825|0.822, 0.829|same_target_only|
|van-der-pol-mu1000|endpoint|1e-06|netlib-dvode|vigilode-fast|2.114|2.107, 2.118|same_target_only|
|van-der-pol-mu1000|endpoint|1e-06|diffsol-bdf|vigilode-fast|3.275|3.262, 3.285|same_target_only|
|van-der-pol-mu1000|endpoint|1e-06|netlib-dvode|diffsol-bdf|0.646|0.643, 0.649|same_target_only|
|van-der-pol-mu1000|uniform101|1e-04|netlib-dvode|vigilode-fast|1.968|1.964, 1.969|same_target_only|
|van-der-pol-mu1000|uniform101|1e-04|diffsol-bdf|vigilode-fast|2.709|2.705, 2.717|same_target_only|
|van-der-pol-mu1000|uniform101|1e-04|netlib-dvode|diffsol-bdf|0.726|0.721, 0.727|same_target_only|
|van-der-pol-mu1000|uniform101|1e-06|netlib-dvode|vigilode-fast|2.682|2.670, 2.687|same_target_only|
|van-der-pol-mu1000|uniform101|1e-06|diffsol-bdf|vigilode-fast|3.890|3.885, 3.904|same_target_only|
|van-der-pol-mu1000|uniform101|1e-06|netlib-dvode|diffsol-bdf|0.688|0.685, 0.690|same_target_only|
|brusselator-1d-50|endpoint|1e-04|netlib-dvode|vigilode-fast|0.387|0.381, 0.388|same_target_only|
|brusselator-1d-50|endpoint|1e-04|diffsol-bdf|vigilode-fast|2.489|2.479, 2.507|same_target_only|
|brusselator-1d-50|endpoint|1e-04|netlib-dvode|diffsol-bdf|0.155|0.154, 0.156|same_target_only|
|brusselator-1d-50|endpoint|1e-06|netlib-dvode|vigilode-fast|0.395|0.393, 0.398|same_target_only|
|brusselator-1d-50|endpoint|1e-06|diffsol-bdf|vigilode-fast|1.832|1.824, 1.838|same_target_only|
|brusselator-1d-50|endpoint|1e-06|netlib-dvode|diffsol-bdf|0.215|0.214, 0.217|same_target_only|
|brusselator-1d-50|endpoint|1e-08|netlib-dvode|vigilode-fast|0.520|0.518, 0.520|same_target_only|
|brusselator-1d-50|endpoint|1e-08|diffsol-bdf|vigilode-fast|1.560|1.554, 1.563|same_target_only|
|brusselator-1d-50|endpoint|1e-08|netlib-dvode|diffsol-bdf|0.333|0.332, 0.334|same_target_only|
|brusselator-1d-50|uniform101|1e-04|netlib-dvode|vigilode-fast|0.187|0.186, 0.196|same_target_only|
|brusselator-1d-50|uniform101|1e-04|diffsol-bdf|vigilode-fast|1.358|1.351, 1.364|same_target_only|
|brusselator-1d-50|uniform101|1e-04|netlib-dvode|diffsol-bdf|0.138|0.137, 0.143|same_target_only|
|brusselator-1d-50|uniform101|1e-06|netlib-dvode|vigilode-fast|0.362|0.361, 0.365|matched_achieved_factor2|
|brusselator-1d-50|uniform101|1e-06|diffsol-bdf|vigilode-fast|1.397|1.396, 1.410|matched_achieved_factor2|
|brusselator-1d-50|uniform101|1e-06|netlib-dvode|diffsol-bdf|0.259|0.257, 0.261|matched_achieved_factor2|
|brusselator-1d-200|endpoint|1e-04|netlib-dvode|vigilode-fast|0.149|0.146, 0.150|matched_achieved_factor2|
|brusselator-1d-200|endpoint|1e-04|diffsol-bdf|vigilode-fast|7.838|7.677, 7.941|matched_achieved_factor2|
|brusselator-1d-200|endpoint|1e-04|netlib-dvode|diffsol-bdf|0.019|0.019, 0.019|matched_achieved_factor2|
|brusselator-1d-200|endpoint|1e-06|netlib-dvode|vigilode-fast|0.156|0.153, 0.157|same_target_only|
|brusselator-1d-200|endpoint|1e-06|diffsol-bdf|vigilode-fast|5.377|5.307, 5.389|same_target_only|
|brusselator-1d-200|endpoint|1e-06|netlib-dvode|diffsol-bdf|0.029|0.029, 0.029|same_target_only|
|brusselator-1d-200|endpoint|1e-08|netlib-dvode|vigilode-fast|0.206|0.205, 0.208|same_target_only|
|brusselator-1d-200|endpoint|1e-08|diffsol-bdf|vigilode-fast|3.512|3.487, 3.539|same_target_only|
|brusselator-1d-200|endpoint|1e-08|netlib-dvode|diffsol-bdf|0.059|0.059, 0.059|same_target_only|
|brusselator-1d-200|uniform101|1e-04|netlib-dvode|vigilode-fast|0.077|0.076, 0.077|same_target_only|
|brusselator-1d-200|uniform101|1e-04|diffsol-bdf|vigilode-fast|4.006|3.973, 4.029|same_target_only|
|brusselator-1d-200|uniform101|1e-04|netlib-dvode|diffsol-bdf|0.019|0.019, 0.019|same_target_only|
|brusselator-1d-200|uniform101|1e-06|netlib-dvode|vigilode-fast|0.152|0.150, 0.154|matched_achieved_factor2|
|brusselator-1d-200|uniform101|1e-06|diffsol-bdf|vigilode-fast|3.476|3.439, 3.519|matched_achieved_factor2|
|brusselator-1d-200|uniform101|1e-06|netlib-dvode|diffsol-bdf|0.044|0.044, 0.044|matched_achieved_factor2|
|pivot-linear|endpoint|1e-04|netlib-dvode|vigilode-fast|2.578|2.569, 2.587|same_target_only|
|pivot-linear|endpoint|1e-04|diffsol-bdf|vigilode-fast|5.092|5.082, 5.124|same_target_only|
|pivot-linear|endpoint|1e-04|netlib-dvode|diffsol-bdf|0.506|0.505, 0.507|same_target_only|
|pivot-linear|endpoint|1e-06|netlib-dvode|vigilode-fast|4.608|4.587, 4.623|same_target_only|
|pivot-linear|endpoint|1e-06|diffsol-bdf|vigilode-fast|6.925|6.904, 6.943|same_target_only|
|pivot-linear|endpoint|1e-06|netlib-dvode|diffsol-bdf|0.664|0.663, 0.668|same_target_only|
|pivot-linear|endpoint|1e-08|netlib-dvode|vigilode-fast|5.173|5.160, 5.190|same_target_only|
|pivot-linear|endpoint|1e-08|diffsol-bdf|vigilode-fast|8.134|8.105, 8.174|same_target_only|
|pivot-linear|endpoint|1e-08|netlib-dvode|diffsol-bdf|0.637|0.633, 0.638|same_target_only|
|pivot-linear|uniform101|1e-04|netlib-dvode|vigilode-fast|0.448|0.446, 0.449|same_target_only|
|pivot-linear|uniform101|1e-04|diffsol-bdf|vigilode-fast|0.873|0.871, 0.874|same_target_only|
|pivot-linear|uniform101|1e-04|netlib-dvode|diffsol-bdf|0.513|0.511, 0.515|same_target_only|
|pivot-linear|uniform101|1e-06|netlib-dvode|vigilode-fast|0.762|0.756, 0.763|same_target_only|
|pivot-linear|uniform101|1e-06|diffsol-bdf|vigilode-fast|1.156|1.151, 1.158|same_target_only|
|pivot-linear|uniform101|1e-06|netlib-dvode|diffsol-bdf|0.658|0.655, 0.661|same_target_only|
|pivot-linear|uniform101|1e-08|netlib-dvode|vigilode-fast|1.162|1.158, 1.164|same_target_only|
|pivot-linear|uniform101|1e-08|diffsol-bdf|vigilode-fast|1.826|1.823, 1.834|same_target_only|
|pivot-linear|uniform101|1e-08|netlib-dvode|diffsol-bdf|0.635|0.632, 0.637|same_target_only|

## 제한

- Netlib 원본 Fortran DVODE와 Rust 두 solver는 동일 실행파일 및 공통 RHS/Jacobian 수식을 사용한다. Python wrapper callback 비용은 timing에 포함되지 않는다.
- VigilODE fast는 요청 시각에 step을 맞추고 Diffsol/DVODE는 dense output을 사용할 수 있다. 출력 시각 계약은 같으나 내부 출력 알고리즘은 다르다.
- E_up 및 R_up의 reference radius는 Radau ladder와 독립 LSODA의 경험적 차이이며 엄밀한 전방오차 인증이 아니다.
- R_up>1은 관측된 격자/global tolerance miss이며 local tolerance만 제어하는 solver의 명세 위반으로 자동 분류하지 않는다.
- 각 arm에서 초기 step 배율 세 가지를 모두 통과한 가장 큰 입력 tolerance를 accuracy만으로 선택했다. timing 최솟값을 골라 승자를 만들지 않았다.
- Diffsol의 FaerLU dense matrix 경로와 DVODE의 Brusselator banded 경로를 명시적으로 구분한다. 이 결과는 특정 solver+backend 구성 비교이며 방법론 일반의 순위가 아니다.
- 새 allocation/Callgrind 측정은 이 캠페인에 포함되지 않는다. 기존 자료의 수치를 새 호스트 결과와 섞지 않는다.
