# A01-UFREF-01 독립 검토

판정: `PASS_SCOPED`. 전체 A01 판정은 `HOLD`다.

독립 검토자는 κ=0의 보존 에너지에서 개별 모드가 단위 원판 안에 있음을 확인했고, exact coefficient-sum 검사가 Taylor 후보의 영역을 보존한다고 확인했다. 검토된 block-Euclidean cubic Lipschitz 상계 `3|χ|S³`에 대해 구현의 `L=max|width·ξ_j|+6|χ|S³`는 보수적이다. degree `3K` 잔차 전개, 지수 Taylor 기하 꼬리, 초기 불확실성 disk 및 반환 binary64 중심을 기준으로 한 반경 포함성도 독립 재실행 oracle에서 일치했다.

재실행 oracle: Python exit 0; 80 disk checks; 1,056 exact-zero residual coefficients; degree 40/48; 음성 endpoint 중심/반경 대조군 모두 거절. Allocator 양성 대조군은 `CollectiveModel::clone()`에서 3 allocations를 관측했고 해당 warmed query는 0 allocations였다.

리뷰어가 확인한 source files와 packet implementation은 `d087dab9a8dacff0b195566c4a649273acdb476c`와 일치한다. 실행 시 공유 worktree의 A05 dirty overlays는 별도 `EXECUTION.json`에 SHA-256으로 결속했다.

이 pass는 독립 unequal-frequency reference subunit에만 적용된다. subnormal/overflow/NaN/Inf 및 모든 지원 영역의 산술 완결을 증명하지 않는다. legacy `compute_certificate`의 ordinary-f64 force/convolution/residual/norm/tail 산술과 `roundoff_enclosed=false` authority 정리가 남아 full A01은 HOLD다. A02의 다중 window 수송과 rejection transaction도 별도 의무다.
