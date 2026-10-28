# A01-LCERT-01 독립 검토

**판정: `PASS_SCOPED`.** A02가 재사용할 수 있는 κ=0 rotating-frame frozen represented-window local inclusion subnode를 닫는다. 전체 A01, legacy `compute_certificate`, trajectory, production 승격은 여전히 `HOLD`다.

## 검토 binding과 실행

- 구현/test checkpoints: `1fe7a1ba9b9cc9144ff3e7304b21dc94b9c6dbce`, `c97912d219c7cceb468ea67e21f5d0715d93bdc5`, `6b722a918deb0ab178b8e77090cac1280dbb3051`.
- 세 실행 receipt의 tree/source/log hashes를 확인했고 현재 소스가 `6b722a9`와 일치함을 확인했다.
- 4-arm/80-disk Fraction oracle은 기존 독립 검증 payload와 byte-identical. Exact one-mode 6 disk checks, compile-fail mutation probes 4개, 최소 subnormal input에 대한 추가 Fraction 24 checks가 통과했다.
- allocator positive control은 같은 allocator에서 3개 clone allocation을 검출했고 warmed successful query는 0 allocation.
- 독립 reviewer probe에서 입력 radii/candidate 변경 후 source owner가 보존되고, NaN refusal 후 재-query 결과 중심/반경 bit identity를 확인했다.
- exact subnormal initial amplitude, time, uncertainty 입력은 포함되었고 typed refusal은 발생하지 않았다.

## 수식과 경계

구현의 \(\lambda=S|\chi|\max(SE_+,M_Y^2)\), \(B_w=e^{\lambda t}(e_{0,w}+t\rho_w)\)는 frozen contract와 맞는다. 연속 residual은 실제 저장된 represented curve에서 계산되고, returned mode/mean radii에는 exact polynomial evaluation과 binary64 returned center 사이 displacement가 추가된다. 첫 성공의 N=3 certificate work는 110 ops, N=1은 52 ops, overflow refusal은 7 attempted ops이며 refusal/attempt counters가 증가함을 raw logs에서 재계산했다. 이 계측은 명시된 항목만 포함하며 전체 FLOP 또는 전체 trajectory 비용 주장은 아니다.

`weighted_radius()`는 정확한 frozen polynomial `Y(t)` 주변의 base `B_w`다. 반환 중심 주변 weighted ball은 아니다. Mode radius에는 evaluation displacement가 포함되어 있다. A02는 outward `delta_w >= sqrt(sum_j w_j delta_j^2)`를 별도로 더해 반환 중심 기준 radius를 만들어야 하며 이 accessor 의미를 source docs에도 명시하기 전에는 해당 값을 A02가 직접 사용하면 안 된다.

보조 검토의 첫 `CompiledPacket.free` mutation probe는 Rust E0616으로 거절됐다. 이는 성공한 privacy check이며, 실패 로그와 원본 probe를 보존했다. 수정한 별도 probe와 Fraction 검사는 exit 0이다. 전체 세부 근거는 `reviewer_readonly_20261018/`와 세 execution identity 파일이다.
