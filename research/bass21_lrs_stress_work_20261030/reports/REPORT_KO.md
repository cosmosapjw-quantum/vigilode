# VigilODE × BASS21 | source-owned radiation stress and anisotropic work, 2026-10-30

**New independent scoped physical dataset, frozen parent PR132.** Source owner exact 50×193 original BASS21 62-state trajectories, 0 new IVPs. First prospectively recorded new full-atlas execution on branch `research/bass21-lrs-source-owned-stress-work-20261030`, prereg commit `3eb9bd535e3aaaf6b63b4048ced60a8c8ba91342`. PR132's earlier three-case preflight was not prospective.

## Scientific result

- 50 × 193 × 6 = **57,900** source-owned Legendre correction entries δJ_{0,2,4,6,8,10}; 25 × 193 × 6 = **28,950** matched pair difference entries, plus **9,650** time-ordered energy/pressure/work budget records. Separate CSVs contain all records; full NPZ contains stress, B_Q, local quadrupole collision/shear budgets, Q equatorial and accumulated work.
- At the terminal source epoch (z=2 model): mean across 50 δJ0 = +4.0251654634559156e-7; mean δJ2 = −1.2351230138784083e-5. Matched 25-pair means ΔδJ0 = +8.065507552213502e-7, ΔδJ2 = −2.4696334720767026e-5; all 25 pair ΔδJ0 positive and ΔδJ2 negative. Mean pair accumulated geometric work = +8.065818082026581e-7.
- Exact continuum photon-energy work dδJ0/dx = −2b'δJ2, Thomson collision monopole exactly zero. Across all 50 original cases saved-grid maximum accumulated work closure residual = 4.402918531762318e-11 absolute, 5.7659799220187526e-5 relative to each case's max |δJ0|. This is NOT a continuum/BDF error bound.
- GL64 vs GL96 highest absolute δJ_even moment difference = 1.9893362859655294e-19. Direct angular stress tensor vs J0/J2 representation highest absolute discrepancy = 7.623296525288703e-21, trace zero to <= 3.3881317890172014e-21. Max local time-derivative quadrupole budget defect = 2.7466652386423947e-8 (finite-difference and time sample).
- Parent coupled Volterra operator maximum residual = 4.6330127234650575e-15, max |Q/I| over new GL directions = 0.012023690610186153, min source-normalized I_full = 0.21921230837473224.

## Claimed status and important limits

This is `derived + numerically_checked + implementation_verified` *within source-conditional frozen-background Thomson response*, not a new Einstein/chemistry/photon baseline IVP, not actual observed CMB E/B, and not full BASS L8/J_l+E2 co-evolution with free-electron/metric feedback. E2 source normalization crosswalk unbound, independent distinct-author R6/C9 NOT_RUN. No tomography of actual sky/no z0 opacity.

The collision operator itself conserves energy in the electron frame; the small nonzero δJ0 originates from shear work and has a verified transport identity. Q is a polarization difference, not separately added to T_{ab}. The high-l δJ10 is a **response** only, not a total l10 certified baseline moment.

## Boundaries and next step

Official VigilODE DAG 7/23, REI central 28/88, REI working 33/88, BASS CE1 9/15, successor 0/12; official delta0; new auxiliary scoped physical stress node 1/1 complete only within model, production HOLD. Preserve PR132 parent's fixed HEAD and original sources. No original chemistry/Einstein IVPs, force push, merge, tolerance relaxation, production promotion or automatic next stage.

A next separately authorized node should map `B_Q` to the *source-owner* normalized polarization E2 in `BASS lrs_boltzmann.py`/`CoupledSystem` and implement genuine self-consistent J_l/E_l expansion on a common event before upgrading Einstein stress feedback. It cannot import BASS KM2E or CE1 different-event data without an explicit map. No additional literature is needed for this scoped moment calculation; for full source-physics promotion require BASS frozen collision source, E2 convention and unit tests, available in owner Git; if the user has an authoritative unpublished full polarized hierarchy or electron collision table, provide exact source/edition and SHA-256.