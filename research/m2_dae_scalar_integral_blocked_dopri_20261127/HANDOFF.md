# M2-03d.5b.19 scalar adjoint integral and blocked DOPRI

Base 1bd729e9f89fc941db4ed90c63dd6260daa51d8a; exact executed code d56bec2da066af48d478b3a6ccd2ede81c11bfac; RED ab0414a875b839789f319040c93a64f056c620be, guide a1441417b0bd110348e45915a3f737cbfdf33ba2. One bounded slice, original distinctive algorithm design re-read. Native subagents absent, serial implementation and author review. No defaults changed; adding public enum variants can affect downstream exhaustive Rust matches.

## Contract and mathematics

Original qualified regular reduced nx=nz=1, no mass/warp/conservation/reset, full horizon and zero incoming differential radius. Original physical_cell_box extracts existing slab interval time, certified native/reconstruction power plus true whole_x/z radii intersect original tube/root box; old bound_cell arithmetic retained. IntegralGuard16 same source/guard cells as Repeated16, sum h*A then one interval exp (one logical interval exp, generally two endpoint evaluations). Direct16 two original interval Newton root narrowing passes and -g_x/g_z, DirectBare16 no narrowing; denominator excludes zero. Adaptive local contribution width <=1e-4*h_lower/T_upper or dyadic depth6, intersects parent integral with sum of children, cap leaves reported. Threshold is integral width allocation, not absolute final gradient width. FlowRatio uses proof of no time factor in f,g, F(xT)/F(x0) times true terminal covector; initial exact differential value and certified initial root intersect first cell. F0 interval containing zero and time-dependent source refused. No adaptive discrete/parameter/reset sensitivity. Same original primal certification required. QuadraticSeed16 uses previously unused quadratic_homotopy_root chart for center only, structurally degree2 in z, midpoint coefficients, regular positive linear term, clamp to certified box, same original interval Newton inclusion. No numerical root proposal promoted as proof.

## Measured scalar portfolio

Seven actual qualified primal epochs; 49 candidates,41 valid zeroheap positive-goal enclosure calls,8 expected refusals (two FlowRatio, six quadratic seed). Negative goals independently checked in41 cases; nonzero incoming refusal and unchanged output. Three prospective sources: clock x'=x+t, g=z-x,x0=0, gradient e^T; equilibrium x'=-2x,x0=0, gradient e^-2T; shifted g=z²+z-x-3/4, f=(2z+1)/16,z0=.5, terminal z gradient1/2. Existing quadratic,quartic,logistic,affine. Independent Fraction checks53864:82 positive/82 negative gradient corners,456 primal,53244 DOPRI baseline sample corners. All gradient widths<=.1. Timings caller CPU0, nonexclusive shared host, seven rotated candidate order samples each8 gradient solves, primal solve excluded. Refusal logs may allocate error strings.

case | candidate | width | source rows | cells | interval exps | cap leaves | median ns
---|---|---:|---:|---:|---:|---:|---:

quadratic | Repeated16 | 9.09627382772e-06 | 390 | 64 | 64 | 0 | 629697
quadratic | IntegralGuard16 | 9.09627351431e-06 | 390 | 64 | 1 | 0 | 304168
quadratic | Direct16 | 9.09627344448e-06 | 390 | 64 | 1 | 0 | 257806
quadratic | DirectBare16 | 9.09627440682e-06 | 130 | 64 | 1 | 0 | 104491
quadratic | Adaptive | 7.7833423543e-05 | 42 | 5 | 1 | 0 | 31051
quadratic | FlowRatio | 1.18610676836e-11 | 12 | 0 | 0 | 0 | 7305
quartic | Repeated16 | 0.000163307808995 | 582 | 96 | 96 | 0 | 1352011
quartic | IntegralGuard16 | 0.000163307808222 | 582 | 96 | 1 | 0 | 1098612
quartic | Direct16 | 0.000160868573695 | 582 | 96 | 1 | 0 | 1127386
quartic | DirectBare16 | 0.000165140764272 | 194 | 96 | 1 | 0 | 335105
quartic | Adaptive | 0.000100889706488 | 1290 | 110 | 1 | 0 | 2306312
quartic | FlowRatio | 3.60808884992e-08 | 12 | 0 | 0 | 0 | 13747
logistic | Repeated16 | 6.86966669693e-05 | 390 | 64 | 64 | 0 | 842170
logistic | IntegralGuard16 | 6.8696666819e-05 | 390 | 64 | 1 | 0 | 369791
logistic | Direct16 | 6.8696666819e-05 | 390 | 64 | 1 | 0 | 296458
logistic | DirectBare16 | 6.869666682e-05 | 130 | 64 | 1 | 0 | 157007
logistic | Adaptive | 8.52933438672e-05 | 246 | 22 | 1 | 0 | 186750
logistic | FlowRatio | 7.60877805206e-09 | 12 | 0 | 0 | 0 | 9218
affine | Repeated16 | 8.05074323179e-16 | 5574 | 928 | 928 | 0 | 10390163
affine | IntegralGuard16 | 1.83555427802e-16 | 5574 | 928 | 1 | 0 | 4401482
affine | Direct16 | 1.83555427802e-16 | 5574 | 928 | 1 | 0 | 3689315
affine | DirectBare16 | 1.83555427802e-16 | 1858 | 928 | 1 | 0 | 1783332
affine | Adaptive | 8.07730618502e-18 | 354 | 58 | 1 | 0 | 235070
affine | FlowRatio | 8.3395924759e-09 | 12 | 0 | 0 | 0 | 7593
clock | Repeated16 | 5.52891066263e-13 | 582 | 96 | 96 | 0 | 1096109
clock | IntegralGuard16 | 6.2172489379e-15 | 582 | 96 | 1 | 0 | 473304
clock | Direct16 | 6.2172489379e-15 | 582 | 96 | 1 | 0 | 409395
clock | DirectBare16 | 6.2172489379e-15 | 194 | 96 | 1 | 0 | 206914
clock | Adaptive | 5.3290705182e-15 | 42 | 6 | 1 | 0 | 34933
equilibrium | Repeated16 | 1.29118937764e-13 | 390 | 64 | 64 | 0 | 643599
equilibrium | IntegralGuard16 | 3.5527136788e-15 | 390 | 64 | 1 | 0 | 237280
equilibrium | Direct16 | 3.5527136788e-15 | 390 | 64 | 1 | 0 | 190504
equilibrium | DirectBare16 | 3.5527136788e-15 | 130 | 64 | 1 | 0 | 94554
equilibrium | Adaptive | 2.44249065418e-15 | 30 | 4 | 1 | 0 | 21012
shifted | Repeated16 | 1.14747294178e-06 | 390 | 64 | 64 | 0 | 973472
shifted | IntegralGuard16 | 1.14747278485e-06 | 390 | 64 | 1 | 0 | 523997
shifted | Direct16 | 1.14747278407e-06 | 390 | 64 | 1 | 0 | 445372
shifted | DirectBare16 | 1.14747279684e-06 | 130 | 64 | 1 | 0 | 161613
shifted | Adaptive | 1.83595637288e-05 | 30 | 4 | 1 | 0 | 40325
shifted | FlowRatio | 1.85351733961e-13 | 12 | 0 | 0 | 0 | 12821
shifted | QuadraticSeed16 | 1.14747278407e-06 | 520 | 64 | 1 | 0 | 561270

FlowRatio fastest in all five admitted cases and narrower than repeated except stiff affine (endpoint state uncertainty amplifies). Adaptive fastest among clock/equilibrium. DirectBare16 reduces source rows; QuadSeed16 used65 times on shifted, exact same width as Direct16,520 vs390 source rows, slower, not selected. Quartic adaptive final width1.008897e-4 despite zero cap leaves, because local target is coefficient integral not final gradient. Scalar zeroheap after workspace preparation, not total primal solve. Constant exact .16 route replay only, no same-campaign timing.

## DOPRI native recurrence

New fixed tiles4/8/16/32 with local bounded array and contiguous derivative slices, identical represented coefficient order and tiny h multiplication; no FMA change, controller/FSAL/observations unchanged. Logical destination writes33n->7n, scratch writes still exist; not actual hardware stores or guaranteed vectorization. Actual13 cases,78 candidate fixed and whole adaptive runs, all exact full serialized identity per case, 78 warmed fixed noheap. Same allocation and RHS/controller counters. Pure linear n1,4,8,16,31,64,127,256,1024, logistic31 and actual DAE quadratic/coupled/affine. Common 17 samples, independent max differential error7.668936721159382e-10. Baseline sample corners counted once per group since all candidates exact. Seven interleaved timing samples fixed reps2048 pure/64 DAE, full8 solves each, shared host.

case | candidate | fixed ns | full dense solve ns
---|---|---:|---:

linear1 | Rows | 104 | 3024
linear1 | Components | 77 | 2869
linear1 | Block4 | 140 | 3206
linear1 | Block8 | 144 | 3193
linear1 | Block16 | 155 | 3248
linear1 | Block32 | 159 | 3337
linear4 | Rows | 145 | 3657
linear4 | Components | 148 | 3813
linear4 | Block4 | 169 | 3867
linear4 | Block8 | 169 | 3818
linear4 | Block16 | 190 | 3898
linear4 | Block32 | 205 | 3863
linear8 | Rows | 177 | 4315
linear8 | Components | 231 | 4841
linear8 | Block4 | 265 | 4740
linear8 | Block8 | 223 | 4458
linear8 | Block16 | 218 | 4506
linear8 | Block32 | 229 | 4503
linear16 | Rows | 287 | 6045
linear16 | Components | 417 | 7159
linear16 | Block4 | 468 | 6967
linear16 | Block8 | 357 | 6276
linear16 | Block16 | 315 | 6030
linear16 | Block32 | 316 | 6087
linear31 | Rows | 486 | 8686
linear31 | Components | 756 | 10940
linear31 | Block4 | 912 | 10999
linear31 | Block8 | 657 | 9579
linear31 | Block16 | 597 | 9203
linear31 | Block32 | 524 | 8833
linear64 | Rows | 811 | 13726
linear64 | Components | 1417 | 18783
linear64 | Block4 | 1681 | 18634
linear64 | Block8 | 1274 | 16104
linear64 | Block16 | 1080 | 15036
linear64 | Block32 | 914 | 14128
linear127 | Rows | 1514 | 23919
linear127 | Components | 2669 | 34360
linear127 | Block4 | 3175 | 34243
linear127 | Block8 | 2351 | 28786
linear127 | Block16 | 1989 | 26270
linear127 | Block32 | 1712 | 25046
linear256 | Rows | 5272 | 85655
linear256 | Components | 10428 | 135820
linear256 | Block4 | 10449 | 119077
linear256 | Block8 | 8596 | 93169
linear256 | Block16 | 6625 | 50574
linear256 | Block32 | 3290 | 44948
linear1024 | Rows | 11821 | 167141
linear1024 | Components | 21099 | 248335
linear1024 | Block4 | 27967 | 243645
linear1024 | Block8 | 18649 | 205551
linear1024 | Block16 | 19034 | 217535
linear1024 | Block32 | 15526 | 179691
logistic31 | Rows | 470 | 6580
logistic31 | Components | 763 | 8352
logistic31 | Block4 | 924 | 8322
logistic31 | Block8 | 677 | 7430
logistic31 | Block16 | 595 | 6972
logistic31 | Block32 | 513 | 6601
quadratic | Rows | 67886 | 249024
quadratic | Components | 67153 | 246507
quadratic | Block4 | 67635 | 245676
quadratic | Block8 | 68110 | 246976
quadratic | Block16 | 67516 | 247239
quadratic | Block32 | 67892 | 250697
coupled | Rows | 336467 | 1415474
coupled | Components | 280927 | 1190606
coupled | Block4 | 292574 | 1141834
coupled | Block8 | 277635 | 983850
coupled | Block16 | 355227 | 1452620
coupled | Block32 | 342230 | 1458356
affine | Rows | 39773 | 2880701
affine | Components | 39483 | 2891025
affine | Block4 | 40606 | 2876240
affine | Block8 | 39633 | 2873581
affine | Block16 | 39607 | 2873210
affine | Block32 | 39597 | 2871203

Rows mostly fastest pure ODE, Components n1, Block32 n256 (fixed5272->3290 ns, full85655->44948 ns in this campaign), n1024 Block32 still slower than Rows. DAE projection dominates and shared-host timing differences are not isolated SIMD evidence. Block4/8/16/32 preserve tails, tiny step; retry/fatal core test extended. No general speed claim or promotion.

## MathHarness and evidence

Original General3.1.0 ZIP+154 files verified,64 abstract routes;127 measured rows (49 scalar+78 DOPRI),2048 proposals/domain,582 mutations,32/32 finite Pareto coverage (8+24). Selection.json includes scalar width thresholds .1,1e-4,1e-6 and separate fastest fixed/full DOPRI. Width qualification precedes cost ranking, no automatic runtime selector or default. First concurrent connector call timed out; Exa/SciSpace/Parallel separately successful, Wolfram evaluator exact identities with unused-symbol warning. First GREEN compile failed because directed Interval not Serialize; removed unnecessary derive and reran. Both behavior RED failures observed; scalar repeated64->integral1, DOPRI block target806->186 writes on n31. Existing warnings residual_radius,mixed,unused Result retained. implement.py is the historical patch script (already applied, not a reproduction command). Not full workspace/Clippy or independent blinded holdout.

Regression: prior four actual portfolios119 logical equal,68 exact,51 timing-only changes; facade10 bins58 pass6 ignored, core5 bins29 pass0 ignored, all final exit0. Actual DOPRI retry 129 attempts/126 accepted/3 rejected/771 RHS/128 FSAL; fatal 1 attempt/0 accepted/4 RHS, all six kernels identical. No nontrivial committed partial fatal case. RED scalar and block101 expected assertions; GREEN both at EXEC_SHA.

## Next single node

M2-03d.5b.20-CERTIFIED-VECTOR-ADJOINT-AND-GOAL-ERROR. Generalize certified time-varying noncommuting vector adjoint to nx2/4/8 with independent manufactured flow (including nonnormal and clock-dependent source). Use source-certified Schur Jacobian and defect-based integration/enclosure, compare backward adjoint against forward variational columns and block actions. For goal error, use a rigorous secant/averaged reduced Jacobian between physical and represented solution, not merely true-flow sensitivity, and original source residual. Autonomous tangent identity S F0=FT is one directional check, not recovery of entire vector adjoint. Profile native ODE stage and source work, choose one real efficient candidate with actual accuracy/time and zeroheap, not endless small layout tweaks. Formal M2/full VG1 OPEN.
