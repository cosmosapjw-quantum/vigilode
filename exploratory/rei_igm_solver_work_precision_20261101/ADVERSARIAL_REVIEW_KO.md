# REI original-source VigilODE: 단일 적대적 검증

원본 REI owner PR348 fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120, frozen kernel SHA256 579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644, frozen source archive SHA256 a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b, original accepted array SHA256 cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01. 원본 repo는 읽기 전용, worktree clean.

A0 PASS SCOPED: PR175의 15-state photo-only reduced IVP, 기존 네 RODAS5P 경로 x 두 forcing. Frozen Radau reference budget 5e-10 모두 통과. Timevarying sequential JVP 10051, RHS 1751, accepted 215; fast workspace JVP 10516, RHS 1789, accepted 221. 결정론적 작업량은 fast가 JVP +465 증가. 한 번의 elapsed time 개선은 diagnostic only, generic speedup 주장 불가.

A1 FIRST FAIL PRESERVED: 46,080-cell 원본 source의 수학적 재배열을 순수 float64로 수행하면 H I interval inclusion 0/46080, heating 3659/46080. 초기 실패 ORIGINAL26_INITIAL_INTERVAL_FAIL.json 및 실제 debug diagnostics 유지. 기준 gate 수정 없음.

A2 PASS SCOPED: 세 global moment 및 네 group scalar coefficient만 numpy.longdouble, 셀별 arithmetic은 float64. 원본 frozen tick160의 H I/He I/He II 각각 46080/46080 및 heating 46080/46080 original interval inclusion. JVP 중앙차분 상대 차이 약 7.95e-13, photon ledger residual 0.0. 이것은 source 조건부 부모 midpoint 한 점의 enclosure 비교이지 all-X 증명이 아님.

A3 BLOCK FOR FULL ADOPTION: longdouble의 He II photon source는 46,080셀 모두 양수이지만 모든 셀에서 float64 zero underflow. Extended cumulative rate ~4.9266637335576802504e-324 particles/s. Interval contains zero라는 사실은 physical trace source 보존 증명이 아님. Full 26-event parity 전에 scaling/log-domain/extended-precision 또는 정당화된 bound 처리 필요.

A4 NOT_RUN: Full MPRK22(1)/Alexander SDIRK2 difference D_h, 26-event thermal root Jacobian, four-site source derivative, HS event-partition differentiability, original all-X uniform remainder. 기존 owner PR348 first-interval common-parent increment candidate는 원본 owner 승인 대기. Historical original failed cross-output logT 2.1245050576368385e-4 >2e-4 기록은 그대로 유지. Tick161 admission 없음.

A5 UNRESOLVED: x86 numpy.longdouble is platform-dependent and not outward directed MPFI proof. Python array temporary allocations are not VigilODE no-hot-loop-allocation Rust proof. Owner interval inclusion of midpoint does not certify globally uniform source. Existing RODAS5P positivity theorem is not inferred here.

Science progress is a reusable original owner photon partial-source low-rank structure, NOT whole REI acceleration. Nonlocal photon Jacobian rank upper <=3 because three global neutral moments, plus local diagonal. Production HOLD, no merge, no force push, no tolerance relax, no auto-next. REI central 28/88, working 33/88, fast 4/7, VigilODE core 7/23, successor 0/12: no canonical DAG delta.

Drift watchdog: stop more point-parent atlases, repeated benchmark tuning and governance-only PRs. The next productive work is to integrate an opt-in rank-three source Jacobian into an original full 26-event solver work-precision test with original 4 sites, energy/photons/HS/thermal/positivity gates and true residual. One same-author review was done; distinct-author reviewer NOT_RUN; native subagent NOT_RUN.
