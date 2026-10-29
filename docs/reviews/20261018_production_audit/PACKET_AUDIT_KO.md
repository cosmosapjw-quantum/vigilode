# Native collective packet 독립 적대적 감사

기준 commit: `2fc61b944fe431d40ba455c373579bce00b0c39a`. 조사자는 packet/window/trajectory/damped scalar 원자료를 독립적으로 읽고 수식을 다시 유도했다. 기존 published campaign은 재실행하지 않았다. 아래 새 counterexample과 변경분 회귀만 실행한다.

## 판정

현재 Track A를 검증된 production solver로 승격한 LP10 판정을 철회해야 한다. source의 `PacketCertificate`는 f64 residual estimate이며, roundoff enclosure가 구현되어 있지 않다. 원래 조건부 Volterra 정리는 유지된다. 현 구현과 정리의 연결, 모형 admission, 시간 restriction, inherited error transfer를 교정해야 한다. 새 좁은 교정 후에도 production certificate는 HOLD다.

## Concrete findings

- **PK-01 / critical** `packet_window.rs:231–275`: κ>0를 받아들이지만 compiler/candidate/reconstruction은 κ=0 방정식이다. n=1, z0=0.5, width=chi=0, κ=1, h=0.1이면 output0.5, certificate0; exact0.5exp(-0.1), error≈0.04758129. 최소 교정은 κ0 API의 감쇠 입력 거절; 실제 damped integration은 별도 구현.
- **PK-02 / critical** `packet_window.rs:363–428`: 문서는 tail, rounding,reconstruction 모두를 enclosing한다고 쓰지만 실제 residual, tail, sums, norms는 모두 일반 f64이며 roundoff 항이 없다. LP10 outward Interval 검증 주장은 source와 모순. NaN candidate는 `bound>1` 비교를 통과해 NaN bound를 포함한 Ok 반환. 최소 교정은 nonfinite refusal와 machine-readable arithmetic authority 미승격; 후속 interval full residual 필요.
- **PK-03 / high** `packet_window.rs:520–523`: dense output에서 h→hτ로 바꾸지만 original normalized-time candidate coefficient c_k를 τ^k c_k로 restriction하지 않는다. n1,width0,chi1,z0.5,h.1의 exact a(t)=.5exp(i.25t)로 독립 확인 가능. 이전 tests는 τ0만 확인.
- **PK-04 / high** `packet_trajectory.rs:270–286`: cert.kernel_model은 τ(A0+hG/(p+2)) 전체인데 G 위치에 다시 넣어 nested τ tail을 만든다. 올바른 total mean=(D+kernel_model+E0)/(1-hL), mode=single_window_mode+hLE0/(1-hL)+previous_radius. 실제현재 code는 source bound를 축소한다.
- **PK-05 / high** `packet_trajectory.rs:143–161`: arbitrary initial radii를 받아들이지만 L=3|chi|,G=|chi|가 필요한 true |m|≤1 domain을 검사하지 않는다. n1,center.5,r100,chi1,width0,h=π/100.5²에서 allowed true start100.5는 endpoint-100.5이고 output≈.5; actual error≈101인데 propagated radius≈100.0934. 최소 교정은 initial uncertainty disk product의 weighted energy bound≤1 확인.
- **PK-06 / medium** `packet_trajectory.rs:134–140,199`: config fields 모두 public인데 constructor 이후 재검증하지 않는다. t_end=NaN이면 while을 건너뛰고 성공. 종료 condition `t<t_end-1e-14`는 아주 짧은 interval을 처리하지 않고 성공. 모든 runtime 입력의 유한성/기본 domain을 API 경계에서 검증해야 한다.
- **PK-07 / high** `damped_moments.rs:174`: beta start를 interval division이 아닌 point(fl(1/(j+1)))로 만든다. κ0,j2,k0의 normalized moment는 exact1/3을 포함하지 않는 point interval. 이전 `exact_beta` test는 동일 rounded recurrence라 독립 reference가 아님. Large-z recurrence는 forward damped moment와 대비해 width가 매우 커지거나 overflow할 수 있으며 exp enclosure는 검증된 elementary scalar kernel 대신 임의1e-15 inflation.
- **PK-08 / high** `collective_model.rs:423–448`: declared model과 arbitrary OdeProblem의 관계는 dimension/mass만 검사. 같은dimension의 다른RHS도 admitted. packet compiler 자체가 model에서 RHS를 생성하는 안전 경로와 이 verifier의 위조가능성을 분리해야 한다.
- **PK-09 / medium** `packet_window.rs:113–124,132–140,309–321,449–469`: 매 sweep Vec allocation이 여러 차례 존재. LP10 hot_loop_allocations0과 source가 일치하지 않는다. work counters는 table setup, reconstruction gamma accumulation, interval/error work, allocation을 계수하지 않는다. counted terms를 total scalar operations로 부를 수 없다.
- **PK-10 / high, retained open** `collective_model.rs:134–161`: |xi|≤1과 normalizedweights가 tolerance로 softened되지만 certificate가 그차이를 charge하지 않는다. 공개mutable CompiledPacket fields도 certificate의 origin/model identity를 깨뜨릴 수 있다. exact descriptor/binding 또는 outward model error accounting이 필요.

## 실행 및 수정 기록

새 focused test와 전후 로그는 별도 evidence에 보존한다. baseline source SHA256도 기록했다. 예전 LP01–LP10 캠페인의 성공 출력은 입력 identity가 유지된 역사적 사실로 남기되, 아래 counterexample이 invalidated한 production enclosure/complete-domain 주장의 authority를 해제한다.

## 좁은 교정 완료

수정 전 새 6개 계약 회귀는 0PASS/6FAIL이었다. 수정 후 8개 새 회귀는 모두 PASS다. κ>0를 κ0 compiler에서 명시적으로 거절했고, dense restriction은 P_h(τs)를 구현했다. trajectory는 single-window kernel-model term을 한번만 더하고 inherited contribution을 분리했다. 초기 uncertainty disk의 weighted energy E와 weight 합계 S를 directed arithmetic으로 계산하여 S*E≤1을 확인한다. NaN candidate와 public config 변조를 실행 경계에서 거절하며, 1e−15 길이의 비어 있지 않은 interval도 실제 진전한다. 정상 작은 초기 uncertainty가 계속 지원되는 회귀도 확인했다.

`PacketCertificate`와 `PacketTrajectoryResult`의 새 직렬화 필드 `roundoff_enclosed=false`가 현재 authority를 명시한다. f64 residual 계산을 interval certificate로 승격하지 않았다. PK-02의 nonfinite handling만 닫혔고 전체 outward coefficient/residual/reconstruction 구현 의무는 남는다. PK-07은 별도 이론/구현 agent가 교정한다. PK-08–10과 weights/ξ contract는 root 보고서의 후속 DAG로 연결한다.

실제 명령: `cargo test --locked --offline -p rodas5p-integrators --test production_audit_packet -- --nocapture` (Rust1.94.1, project offline vendor). 첫 post-fix compile은 동시 작업 중 `leja_action.rs` ambiguous float형 때문에 중단되었고, 해당 owner가 형을 교정한 뒤 동일 packet test8PASS를 확보했다. packet source가 원인인 실패와 구별한다. 초기 rustfmt component 부재는 설치 후 해당3파일에 rustfmt를 적용하여 닫았다.

독립 검토자는 dense restriction과 inherited transfer 식을 확인했다. weight normalization 허용오차를 고려하여 초기 tube 검사만 S*E≤1로 교정했다. 정확한 kernel mass/ξ 전제의 전체 연결은 PK-10에서 열린 상태를 유지한다.
