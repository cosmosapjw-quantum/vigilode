# Original 26-event REI photon-source → VigilODE native 15-state time forcing

## Source and equations

For the 46,080-cell original REI photoionization suboperator, at each source instant u the three global neutral sums S_s and four photon-group currents J_g and external weights E_g satisfy

F_{isg}(u) = J_g(u) C_{sg}(u) N_{is}(u) / [ E_g(u) + sum_a C_{ag}(u) S_a(u) ].

When C, J, E depend on u but not on the individual cell i except through N_{is}, the fraction f_{is}=N_{is}/S_s is invariant under **photoionization-only** depletion, since d(log N_{is})/dt = d(log S_s)/dt (for HI, HeI). Thus the 46,080 original cell photoelectron histories are reconstructed exactly **within the reduced physical model** from the original source-identified f_{is}(0), and global integrated events. This does not say the full 26-event chemistry has the same invariant.

The PR332 source point gives p^0_{sg}, p^0_{ext,g}, J^0_g, S^0_s. PR342 provides 5 clock values p^0_{sg}(u_i), external p^0_{ext,g}(u_i) and source photon-current *upper bounds* J^{hi}_g(u_i). In this work J^{model}_g(u_i) := J^0_g J^{hi}_g(u_i)/J^{hi}_g(u_0). Piecewise-linear interpolation of J^{model}_g and the positive normalized four-channel p^0_g across u_i gives a reproducible approximate continuous source. It is exact at the original source snapshots for p and matches original initial J; at other times this is an **external prescribed interpolation**, not proven owner forcing.

w_s=S_s/S_s^0, q_{sg}(u,w) = p^0_{sg}(u) w_s / (p^0_{ext,g}(u)+sum_a p^0_{ag}(u)w_a), R_s(u,w)=sum_g J_g^{model}(u) q_{sg}(u,w).

With proper one-tick time h and u=(t-t160)/h:

dwHI/du=-h RHI/S0HI; dwHeI/du=-h RHeI/S0HeI; dwHeII/du=h(RHeI-RHeII)/S0HeII.

da_s/du=h R_s/S0_s; de_g/du=h J_g q_ext/S0_HI; dm_g/du=h J_g/S0_HI; db/du=h sum_s I_s R_s/S0_HI [eV per initial neutral HI].

**Exact reduced invariants:** wHI+aHI=1, wHeI+aHeI=1, wHeII-1-(S0HeI/S0HeII)aHeI+aHeII=0; sum_g(m_g-e_g)-sum_s S0_s a_s/S0_HI=0; b-sum_s I_s S0_s a_s/S0_HI=0. These include external photon escape and explicitly distinguish ionization binding energy from thermal gas heating.

**Exact analytic JVP:** for a shared state tangent v_s, δq_{sg}=q_{sg}[v_s/w_s - sum_a q_{ag}v_a/w_a], δq_ext,g = -q_ext,g sum_a q_{ag}v_a/w_a. This is passed directly to the existing native RODAS5P, without assembling a 46,080x46,080 matrix. It retains local numerator versus shared global denominator response. Wolfram PR332 PR342 source identities, not a Taylor certificate of original MPRK full/half.

## Adversarial R6/C9

- **High impact, known incomplete:** 5-knot time interpolation has no certified continuum forcing remainder or actual original 26-event four-site stage transport. Result stays source-conditional.
- **Potential attribution error, fixed by explicit definition:** PR342 original current field is an outward upper float; only its ratio sets J_model(u_i), while PR332 original initial midpoint J0 pins absolute current. Do not say all forcing currents are original exact midpoints.
- **Physics exclusion:** no recombination/HeII double ionization (original group 3 subnormal), no temperature/OTS coupling or source response to feedback from recombination. Chemical binding b is not thermal photoheating or full radiation+escape energy ledger.
- **Nonunique parent:** PR332 used nuclei-conserving representative in the accepted parent box; PR320 xHeII/xHeIII used to obtain marginal outer electron/H intervals. These may not correspond to a common original dynamical parent; do not claim a rigorous all-X observer.
- **Validation tier:** source original ZIP checks and local real-data 46k-run, Radau/DOP853 are one author's independent algorithms; native RODAS5P must execute and read back separately. Native RODAS5P does not guarantee positivity for arbitrary stiff reaction RHS; observed positivity only for this source case.
- **Progress watchdog:** actual new source-time IVP + spatial electron history yes; official first canonical interval or VigilODE DAG node advance no. Stop additional photo-only sidebranches after this and bridge source to original 26-event thermal owner or a true same-parent defect when available.
