# REI F05 native interval MPRK corrector | R5 constructive theorem and C5 contract

## Identity / selected scope (2026-11-02)

- Immutable predecessor VigilODE PR #183 `df3bc5752970e0c0840b596c76175b4143a8edd8`.
- Original REI owner `fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120`; source tar `a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b`; 26-event kernel `579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644`; original accepted parent `cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01`.
- REI PR #368 separately versioned source worker and PR #369 next-parent use are NONOFFICIAL V2 source-scoped; original V1 failed logT `2.1245050576368385e-4 > 2e-4` remains untouched. Official accepted tick only 160/131072. N=46,080 genuine H/He cells.
- This cycle R5 -> C5/C6/C7 scoped: independent **native Rust interval enclosure for the ORIGINAL 2x2 and 3x3 MPRK22(1) final population corrector** on the 3 original full/half1/half2 maps. Source owner supplies true 46,080×3 interval parent, 25 flux slots and 5 predictor denominators, no surrogate model; Ruby/Python source-owner only read-only transport. The genuine full time-map interval thermal roots, branch box, 14 signed ledgers and official tick161 are NOT inferred from this linear substep.

## Theorem (exact arithmetic, directly derived)

For a conservative directed production-destruction flux \(F_{ij}\ge0\) from column j to row i, strict positive denominator \(\sigma_j>0\), and timestep \(h>0\), define
\[ A_{ij}=\begin{cases}1+h\sum_{k\ne j}F_{kj}/\sigma_j,&i=j,\\-hF_{ij}/\sigma_j,&i\ne j.\end{cases}\]
Then \(\mathbf1^TA=\mathbf1^T\), every column is strictly diagonally dominant, and \(A\) is a nonsingular M-matrix. In particular \(A^{-1}\ge0\), \(\mathbf1^T A^{-1}=\mathbf1^T\), hence \(\lVert A^{-1}\rVert_1=1\). This applies separately to H(2) and He(3) blocks, even though flux coefficients depend on global photon moments, provided they are held at the original source evaluation stage. The original Patankar result \(y=A^{-1}p\) conserves H and He nuclei and remains strictly positive for \(p>0\).

For the INCREMENT \(\delta=y-p\), write \(A\delta=-(A-I)p=:b\). Let \(\boldsymbol{z}\) be ANY finite f64 trial increment, and let interval arithmetic with documented outward rounding enclose every admissible \(A\), \(b\) and component residual \(r=b-Az\). Define \(\rho=\sum_i\max(|\underline r_i|,|\overline r_i|)\), with an outward upper sum. Since \(\delta-z=A^{-1}r\), every exact admissible \(\delta_i\) lies in \([z_i-\rho,z_i+\rho]\), with outward endpoint rounding. This is valid without Krawczyk contraction and without trusting the trial solve, but may be wider than original owner Krawczyk. Exact source-family positivity allows optional intersection with \(y_i\ge p_i/A_{ii}\) and nucleus-sum bounds; if intersection empty, fail closed rather than widen a user acceptance tolerance.

## Computational/negative contract

Read original sealed 3-map final-corrector source boxes using unchanged `original_map` wrappers. Export one byte-checked binary with 70 f64 per cell per map: \(p_{lo/hi}\) 5+5, \(F_{lo/hi}\) 25+25, \(\sigma_{lo/hi}\) 5+5. No source code or original F05 worker mutation. Rust imports `rodas5p_core::directed` exact outward primitives, rejects negative physical flux beyond original outward-zero \(-2^{-1074}\), singular/negative denominators, overflow, nonconservative block transfer or source-byte drift; runs on original 46,080×3 states. Compare independent physically relevant (HII, HeII, HeIII) increment enclosures against ORIGINAL owner common-parent increment reference, preserving full-vs-twohalf correlation distinction. Record widths and any misses. Strictly do not call this a certified full H/He/OTS thermal IVP, global continuum error certificate, HeII f64 underflow repair, or native speedup.

## Research and termination gates

- R5 exact M-matrix argument + small exact 2x2/3x3 test; C5 actual input-to-output 46k ×3 Rust code.
- Native interval accounting PASS requires explicit original source hashes, positive strict M-matrix physical rates, actual residual enclosure, strict 3-map source clocks, and original owner interval reference comparison.
- If ρ too broad for useful physical tubes, classify sharpness LIMITATION rather than false physics FAIL; preserve the computed (possibly conservative) intervals and smallest alternative certified tightening.
- Earlier point-native full/half states are retained as independent diagnostic only. Original 14 FINAL interval ledgers are **owner-tested** and NOT yet natively recomputed. HeII positive subnormal remains UNRESOLVED.
- No default change, merge, force-push, production promotion, threshold relaxation, source-owner modification or official tick161.
- DAG before/after without new owner admission: VigilODE 7/23, successor 0/12, REI central 28/88, working 33/88, physical-fast 4/7, canonical delta 0. Native subagent unavailable: self-hostile referee only. One bounded productive implementation/review episode.
