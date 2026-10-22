# Independent review: full Li I absorption with a declared charge closure

Date: 2026-10-21. Reviewer: `/root/next_backup_context`, separate from root and the atomic-network implementation author. Status: **ACCEPTABLE_CONDITIONAL_NUMBER_MODEL**. Final code and output review remain pending.

## Source actually inspected

REI PR151 commit `8a4f5df46f6e3058f5e685b85cc1cf3c20926368` supplies Unit7 `LI_PHOTO_CONTRACT.json`, `li_photo_driver.py`, `provider_reconstruction.json`, and the Verner source manifest. These files were read directly from the local Git-source intake; the intake manifest records matching Git blob identities. The driver SHA256 is `2851dabdffb35c6912e8f7de20f2db01390690caa2fa8cab6031d413d5fa09b7`; the contract SHA256 is `2428928d7e5f2d95254a97e0332492c03d13ac7dc3c22c1208e97ae257a253bf`.

The inherited provider binds the original `phfit2.f` SHA256 `8df982535a4a2b952e33ca2ffb6d81600e156d7373fbf547a0c0170c1cecc296` and independently matches its selected Li rows to `table1.dat` and `photo.dat`. The primary source URL recorded there is https://www.pa.uky.edu/~verner/dima/photo/phfit2.f .

The exact LiI branch is:

| Energy | 2s partial | 1s partial |
|---|---|---|
| below5.392eV | 0 | 0 |
| 5.392≤E<64.39eV | PH2 | 0 |
| E≥64.39eV | PH1 | PH1 |

Partials are summed once, without another shell-occupancy multiplier. Thus the previous outer-only `photo.dat` Emax64.39eV limitation is resolved by this additional shell provider. The earlier intake's outer-only restriction must not be promoted into a permanent obstruction after this source becomes available.

The old Unit7 saved-response validation covers source energies through91.83842857919419eV. A new200eV evaluation is a new finite evaluation scope. It must remain within the primary provider's declared domain and should be checked at the new energies; the old validation is not silently relabeled as a200eV trajectory validation.

## Strongest admissible interpretation

For each selected isotope A and charge q, define a finite number generator with positive photo rates and RR/DR rates. LiI photo rate includes both shell partials. Each absorbed photon is mapped by this model to one q→q+1 transition and one additional free electron. This fixes a coarse-grained charge yield as a **model assumption**, rather than deriving it from the cross section.

The resulting ODE is mathematically legitimate. Its atomic event stoichiometry preserves every isotope count and A-weighted baryon number, and its ionic charge increase matches its electron increase. With finite rates, positive H and prescribed T, the vector field points inward on every zero-population face. The conserved nonnegative abundance simplex bounds the solution. Missing branching data therefore do not imply that no conditional number model or preliminary computation can exist.

The physical interpretation is narrower: the q+1 compartment uses the adopted downstream ground-parent rate model after unresolved internal relaxation. The calculation does not claim the actual final charge distribution, Auger branching, fluorescence yield, direct double-photoionization, secondary ionization or state-resolved cascade. Isotope equality remains a separate explicit approximation. No CI-omission error bound follows from the process selection.

## Minimum accounting conditions

1. Freeze the photon support, normalization, shell branching and units. For an isotropic field, the scalar encounter flux is `F_E=c*n_gamma,E`; it is not the vanishing vector net flux. The externally maintained UV field supplies its absorbed photons and energy. It is not the CMB hierarchy's number array.
2. For LiI1s keep `K_primary=E−64.39eV`, positive valence cost `χ_val=5.392eV`, and unresolved core-energy allocation `X_core=64.39−5.392eV` per1s event. The source identity is

   `W_abs = K_primary + χ_val*Gamma_total + X_core*Gamma_1s`.

   For a spectrum these are absorption-weighted integrated rates. `E−χ_val` for an inner-shell event includes core excitation; it must not be named immediate thermal heat.
3. A ground-charge-state ledger may store only the cumulative charge ionization costs if the unresolved core term is retained separately and included in the external/reservoir balance. A claimed closed thermal solution would require further source physics; the selected prescribed-T model does not claim one.
4. Preserve the required net gas input and the bath-minus-escape residual labels. RR/DR number coefficients alone do not determine their kinetic/emitted-energy moments. The computed residual cannot certify that the thermostat or radiation reservoir has the actual required behavior.
5. Electron density must use all active ionic charges once; it damps the photon hierarchy through Thomson scattering only. Neutrino scattering is absent. Matter and radiation signed stresses are summed before their total norm.

## Outcome and remaining verification

The UV5–200eV conditional model is admissible for this finite research pilot when these conditions are reflected in the implementation and report. It can populate LiIII/LiIV through their source-supported electronic transitions while preserving the full21-state inventory. Physical HOLD, continuous/global convergence NOT_RUN and production NOT_APPROVED remain unchanged. This scoped acceptance is neither a full19-reaction primordial-network completion nor a bound on neglected processes.

Next review: actual selected-rate wrappers, finite event map, source binding, coupled geometry, saved scientific arrays, and the produced plots. No new ODE, NCP job, benchmark or previous campaign replay was executed for this independent scope review.
