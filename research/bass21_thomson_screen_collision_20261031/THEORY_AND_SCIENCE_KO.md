# Original BASS21 Thomson tensor and passive observer collision E/B

Source-owned electron-normal rest frame, metric (-,+,+,+), x=ln(a/a_ref), g=c sigma_T n_e/H. Exact parent PR155 `5c2ec59a489073d67e60021d957d03ed3373edac`; prospective prereg `027b9c91a82b37de4b6d61f0d11cb0dcb996a06e`. No new IVPs.

At each source state J0,J2 and Q24(mu)=Σ_{even l≤24}(E_l/k_l)P_l²(mu), c2=E2/k2, the full 3D screen-coherency gain is `G_ij(n)/g=(3/(8pi))S_i^k M_kl S_j^l`, M=∫P_ij dΩ. Exact LRS tensor M is diag(Mperp,Mperp,Mparallel), with `Mparallel-Mperp=-2pi J2+(24pi/5)c2` and `tr M=4pi J0`.

Hence `I_gain/g=J0+(J2/2-6c2/5)P2(mu)`, `Q_gain/g=(-J2/4+3c2/5)P2²(mu)`; source-frozen polarized collision `C_Q=Q_gain-g Q24`, `C_E2=-g(k2 J2/4+2E2/5)`, `C_El=-g E_l (even l>=4)`. The exact intensity low moment source is `C_J0=0`, `C_J1=0`, `C_J2=-g[(9/10)J2+(6/25)c2]`, with `C_(Pparallel-Pperp)=4pi C_J2`. Eigenvalues of the normalized (J2,c2) collision generator are -1 and -3/10.

Original PR141 integrated source M and 5-angle I/Q gains match this exact tensor; PR145 saved collision l2..22 matches exactly (bitwise), independently reconstructed 3D tensor integral at four source states agrees within 2e-13. Source physical data: original 50×193 tensor rows, 5 passive original observer frames×77 l,m outputs, 3,715,250 E and 3,715,250 B *collision-contribution coefficients*. Rest B max ~1.74e-15, transverse x.05 B max 0.002738751; control d1 B max ~1.74e-15. Max original collision intensity quadrupole rate 0.020765784; maximum pressure anisotropy rate 0.260950538.

**Important scope:** The original full unpolarized I(mu) is not fitted; only exact Thomson gain and low-rank intensity collision, plus full reconstructed Q24 polarization collision, are admitted. Passive observer transformation of the collision field is not the complete moving-observer Liouville-Thomson source per observer proper time. The physical source was already solved, no new owner IVP. This is not an observational B-mode nor generic full 1+3 non-FLRW PSTF hierarchy. Original s00 higher multipoles have inherited angular accuracy floor. Independent-author R6/C9 and native subagent NOT_RUN.

Full scientific source (original nested PR155, actual NPZ, code and 40 tests): ZIP 268548194 bytes, SHA256 `afa36332c223ac1c6c2495a5262822d66250b38390d698a8f254d8069ba13030`. Empty-directory clean replay: 54/54 payload, 40/40 tests, 12/12 source-derived result files byte-exact, ZIP CRC PASS. The Git numeric witness is intentionally only 10 representative source endpoints; Git scoped CI does not rerun original private NPZ. Official VigilODE DAG 7/23; production HOLD; NO_MORE_IVPS, no merge, no force push, no auto-next.

Primary literature: Challinor PRD62 043004 (2000), exact Thomson scattering Eq40; Challinor & van Leeuwen PRD65 103001 (2002); Dai & Chluba PRD89 123504 (2014); Yasini & Pierpaoli PRD96 103502 (2017).
