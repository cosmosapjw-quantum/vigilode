# BASS21 source-owned axis-boost observer polarization crosswalk

Parent PR151 HEAD `81aeea38e8a7b856286e3738de1c3611fafa6467`, prospective registration `9173e8c6ebd6a113336b24b934f77015312e8071`. Frozen original PR145 E_l0(+) 50 cases ×193 epochs, reconstructed Q24 with even l2..24; not the original infinite-l sky.

For a co-located observer moving +βc parallel to the original LRS axis (μ is photon *propagation* cosine), μ'=(μ−β)/(1−βμ), D=ν'/ν=γ(1−βμ), Q'_bol(μ')=D⁴Q24(μ). The original transported axial screen has U=B=0; arbitrary non-axial observer and generic Wigner/E-B are NOT covered. Project for all integer l2..32 including odd using E'_l=k_l/[2N_l]∫_{−1}^{1}Q'(μ')P_l²(μ')dμ', k_l²=4πN_l, N_l=(l+2)!/[(2l+1)(l−2)!].

Exact first-order generator T P_l²=−(l+4)(l−1)/(2l+1)P_{l+1}²+(l+2)(l−3)/(2l+1)P_{l−1}². Pure original E2 gives E'3/E2=−6β/√7+O(β³). Reverse β changes odd-l sign and preserves even-l. Independent norm identity: ∫dμ' Q'^2=∫dμ D⁶ Q24². Wolfram exact polynomial checks l2..12 PASS.

Actual source experiment beta=[−0.05,−0.001,0,+0.001,+0.05], 50×193×5×31 = **1,495,750** source-conditional E_l values, 0 new original IVPs. Max |E'3| 4.024525818457852e−3 at |β|=.05, 8.04697206887487e−5 at |β|=.001; beta0 ≤5.863651214e−18. s04 final mean E'3(+.05)=−0.0029395842303073906. GL96/128 gap ≤3.7470027081099033e−16; independent μ-source transform ≤7.979727989493313e−17, spin basis ≤1.734723475976807e−17; D⁶ norm ≤1.8973538018496328e−19. Same-environment clean archive replay: source payload SHA 49/49, physics tests 32/32, numeric/CSV/PNG/SVG output hashes 9/9 exact.

Full executable source ZIP SHA256 `e8faa022fb6e567a97f17f73cca4571f071ffa37de5ea8e9597b0c4d98b088bc`, 77,035,898 bytes. **Not present in Git** (binary owner archives/NPZ omitted); available in current conversation as a sandbox artifact. Library and Google Drive upload of this new ZIP FAILED with explicit `container_session_expired`, so do not claim two-provider backup. All old backups untouched.

This is an observer transformation of source Q24 and **not** independent cosmological physics trajectories, observed CMB E/B, original high-l convergence, generic 1+3 PSTF hierarchy, or full Einstein/electron/thermal/coupled J_l/E_l IVP. Self-hostile pass only; distinct-author R6/C9 and native child NOT_RUN. Official VigilODE 7/23, REI central 28/88, working 33/88, BASS CE1 9/15, admitted successor 0/12, official delta0, production HOLD; no merge/force push/new owner IVPs/auto next.

References: Challinor, Phys Rev D 62 043004 (2000), doi:10.1103/PhysRevD.62.043004; Challinor & van Leeuwen, Phys Rev D 65 103001 (2002), doi:10.1103/PhysRevD.65.103001, Eq.(1-5), Sec.III; Dai & Chluba, Phys Rev D 89 123504 (2014), doi:10.1103/PhysRevD.89.123504.
