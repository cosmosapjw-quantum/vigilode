# 21-state photorecombination contract: independent source/physics review

Reviewer: `/root/next_backup_context`, distinct from root implementation and theory/source-author agent. Date: 2026-10-21. This review reads the Stage A `README_KO.md`, the pinned REI150 Li source, current REI210 atomic report and REI211 RUN19 source/report. It is a contract review; new solver code/results were not yet available and no ODE was executed by this reviewer.

## Scope decision

The selected prescribed-temperature, prescribed-UV, test-fluid number network is mathematically concrete. It can generate a preliminary 21-state isotope/charge history coupled through its actual free-electron density to Thomson scattering in the canonical LRS radiation hierarchy. That result belongs to this finite process model. Neither isotope-precise rates, an internally closed thermal/radiation evolution, nor the complete 19-reaction primordial target network follows from it.

The species count is consistent: n(1), H1/D/T(2 each), He3/He4(3 each), Li6/Li7(4 each), total21. Each atomic transition changes only electronic charge, preserving its isotope number and A-weighted baryon count. If `Y=n/nB` and `nB'=−3nB` in x=ln a, the atomic contribution is `dY/dx = S_number/(H*nB)`. Charge-neutral `ne=nB ΣqY` must include all21 states once. All states may remain in the registry even when chosen initial abundances are exactly zero.

The existing case-independent Li numeric coefficient files have actual Dropbox byte identities matched to Git PR150 `fd9a21c4d3886a495540b51d780a83b608e72ea2`. These rows are elemental electronic fits; the mandatory isotope equality approximation must stay visible. Source preservation does not prove empirical fit accuracy outside the original support.

## Source-domain conditions

1. Use per-channel Li RR/DR evaluation, not the existing full CI+RR+DR bundle. The full bundle requires T≥5/(8.617385e−5) K because of CI. Selected RR q1/2/3 has conservative supported Tmin10/40/90K. T≥100K is sufficient for these lower bounds; its upper bound must also respect the chosen H/He owner (the IGM subset operational guard is1…10^6K) and Li rows. Keep the IGM operational guard distinct from a universal empirical-error claim.
2. LiI Verner outer-shell photo fit ends at64.39eV. A prescribed spectrum entirely below that energy may be evaluated within this source. Then LiII/III photo transitions are below their higher thresholds and correctly zero. A tail above64.39eV requires another admitted LiI/inner-shell provider, not silent cutoff, extrapolation or treating missing opacity as0. If a blackbody/power-law is truncated, the truncation is part of the source contract.
3. Low-temperature CI omission is an explicit process reduction. It is not an error bound. The same applies to CX, molecules, negative ions, nuclear captures, photodisintegration and radioactive evolution unless explicitly implemented.
4. Neutron and T positive initial populations cannot be described as a full late-universe isotope evolution when their decay is omitted. Zero initial populations remain identically zero in this atomic-only model and should be described as inventory-only rows. No fresh BBN donor is implied by assigned abundance values.
5. The user PNG's full target obligations and RUN19 `TARGET_NETWORK_19.json` must remain distinct from the selected atomic slice. Electronic Li recombination is not Li photodisintegration or T(α,γ)Li capture.

## Energy accounting that is computable without invented RR moments

Let `npart = Σheavy ni + ne`, `S_e` be the net electron-number reaction source per proper volume per second, and `Uth=(3/2)kB*T*npart`, `p=kB*T*npart`. Pure atomic ionization/recombination preserves total heavy-particle number, so

\[
\dot n_{\rm part}+3Hn_{\rm part}=S_e.
\]

The energy needed to maintain the externally prescribed temperature satisfies the exact EOS identity

\[
Q_{\rm th,required}
=\dot U_{\rm th}+3H(U_{\rm th}+p)
=\tfrac32 k_B n_{\rm part}\dot T
 +\tfrac32 k_BT S_e+3Hk_BT n_{\rm part}.
\]

Choose each ionic state's stored ionization energy relative to the neutral ground state as the cumulative positive binding cost. Then `B=Σni εi`, and

\[
Q_{B,\rm reaction}=\dot B+3HB
=\sum_{A,q}\chi_{A,q}\,(R_{\rm PI,A,q}-R_{\rm rec,A,q+1}).
\]

Both PI and recombination events must use the same `χ` in this ledger. A fit's rounded threshold and a separately chosen physical binding energy are different conventions; if both occur, record their difference and preserve nonnegative photoelectron energy.

The quantity

\[
Q_{\rm gas,required}=Q_{\rm th,required}+Q_{B,\rm reaction}
\]

is the required **net gas energy input**. For a known prescribed spectrum, absorbed photo energy `Pphoto=Σ ∫ E dRPI` and direct excess `Pphoto−ΣχRPI` are also computable. Without an RR/DR rate-weighted electron kinetic/emitted-energy moment, neither actual recombination radiative loss nor the thermostat's standalone heating power is known. The computable residual

\[
Q_{\rm gas,required}-P_{\rm photo}
=P_{\rm bath}-P_{\rm escaped,rec}
\]

is a **combined bath-minus-escape requirement**, conditional on the declared omitted processes. Labeling it as actual RR cooling or as a closed thermal solution would be incorrect. In particular, assigning3kBT/2 per recombined electron just from the number rate invents a rate-weighted moment.

An energy-density ratio against the inherited Einstein matter budget quantifies the test-fluid assumption for the selected histories. It does not prove an accumulated trajectory-error bound. A baryon population already included in the inherited dust density must not be added to the Friedmann source a second time.

## Matter/radiation and observable bookkeeping

For the non-tilted common-temperature ideal-gas baryon closure, isotropic pressure gives `π_b=0`; an independently modeled collisionless matter component supplies `π_m`. The radiation stress must retain photon and massless-neutrino signed contributions separately, then sum them. The computed `ne` applies to photon Thomson scattering only. Prescribed isotropic UV has zero anisotropic stress but can have nonzero energy density; its omission from Einstein evolution belongs in the test-fluid energy ratio.

For orthonormal `diag(−p,−p,2p)`, the Frobenius norm is `sqrt(6)|p|`, while the half-contraction norm is `sqrt(3)|p|`. Use `κ_t=8πG/c²` when p is SI energy density, and combine signed matter/radiation stresses before taking total norm. Cancellation makes `Pi_total` different from `Pi_m+Pi_r`.

## Review status and next evidence

Status: `CONTRACT_CONDITIONAL_SCOPED`, with the explicit source-domain, reservoir, and energy-label conditions above. This is not an implementation PASS. The useful next check is a read of the actual selected-rate wrapper, network/temperature/UV contract, and one saved result receipt to confirm that their quantities match these equations. No detailed benchmark, old-campaign rerun or production-approval change is needed for this review.
