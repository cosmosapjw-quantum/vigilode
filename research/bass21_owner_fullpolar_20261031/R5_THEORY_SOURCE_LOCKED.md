# BASS21 owner-frozen full polarized Thomson collision | R5 derivation

**Scientific status:** DERIVED, SOURCE_CONDITIONAL, NONPRODUCTION. Parent Draft PR #162 fixed at `02f16fac23090cd4b8bfc5981e5b196c9e47c791`. No parent HEAD, original owner source, source-owned CMB history or original IVP is changed.

## Source identity and exact scope
Original BASS owner 50 histories x 193 stored states source ZIP SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`, original E24 NPZ SHA256 `b087a54bcd3bdbfaef5d91165634bf5c7590e6c43417112e4de0168327b3366b`. Replayed pre-existing source-frozen coupled Volterra I(x,mu),Q(x,mu) on 96 Gauss–Legendre nodes; resulting I/Q NPZ SHA256 `59190f5ce6ffc7b7927c41c6fba91889d4332ebfab420ad721b790b91e1504ab`. This is a saved-source replay, **not a new Einstein–Boltzmann IVP**. Originally sourced I and Q have U=V=0 in their LRS normal frame. A separate general scattering kernel supports synthetic nonzero incoming U,V, but does NOT imply source-owned U,V dynamics.

Signature (-,+,+,+), beta=v_e/c, |beta|<1, x=ln(a/a_ref), c and sigma_T retained, original `g(x)=c sigma_T n_e/H` with normal-frame electron density n_e. A cold, single electron bulk velocity β is a controlled counterfactual, not an observed BASS owner velocity. Bolometric, Thomson elastic, no recoil, no thermal velocity, no induced scattering.

For incoming screen coherency, K_AB=(1/2)[[I+Q,U-iV],[U+iV,I-Q]], let the transverse spatial 3x3 tensor be K_ij(n)=e_Ai K_AB e_Bj. The Lorentz electron-rest Doppler is D=gamma(1-beta·n); transform the photon direction and screen dyads by null-gauge restoration. Bolometric coherency boosts as K' = D^4 L(K) and dΩ'=D^-2 dΩ. Define the exact rest-frame moment

`M'_{ij} = ∫ dΩ' K'_{ij}(n') = ∫ dΩ D² L(K(n))`.

The Thomson gain for outgoing electron-frame dyad e'_A is

`G'_AB(n') = (3/(8π)) e'_A{}^i M'_{ij} e'_B{}^j`.

At fixed original normal-frame n_e, electron proper density is n'_e=n_e/gamma. The local normal-frame collision source per x is

`C_AB(n) = g(x)/gamma * [ D^-3 G'_AB(n') - D K_AB(n) ]`.

Both gain and loss are components in the same restored outgoing screen. The D^-3 arises from D times D^-4 bolometric frame conversion, the loss from encounter flux D. This is NOT a complete Liouville–Einstein–Boltzmann evolution, nor an all-sky harmonic B mode.

For an arbitrary complex Hermitian 3x3 electron-frame angular moment M, with P_n=1-n n^T, exact angular gain map A(M)=∫3/(8π) P_n M P_n dΩ equals

`A(M) = M/2 + [tr(M) I_3 + M + M^T]/10`.

Using ∫n_i n_j=4πδ_ij/3 and ∫n_i n_j n_k n_l=4π(δijδkl+δikδjl+δilδjk)/15, one obtains trace eigenvalue 1, symmetric-traceless eigenvalue 7/10, imaginary-antisymmetric Hermitian eigenvalue 1/2; local collision-only damping rates 0, 3/10 and 1/2, respectively. These are not streaming eigenmodes.

## Executed original-source conditional physics
50 original source cases x193 saved epochs x4 prescribed electron velocities x5 outgoing directions = **193000 newly evaluated local Thomson I/Q/U/V collision rows**, and **38600 source-conditioned electron-rest coherency moments**. Prior PR141 original M comparison maximum 2.930988785e-14; PR145 parent collision E_l parity maximum 2.359223927e-16; independent direct 3D M integration vs factorized Lorentz source max 4.440892099e-15. Original Volterra fixed-point max 4.633012723e-15, min(I-|Q|)=0.218185535. The source-frozen field has NO added maximum-entropy intensity closure.

Frame maximum |C_U|: rest 2.61482505e-17, beta_z=+0.05 3.92176190e-17 (axis null), beta_x=+0.05 0.000735161124, beta tilted 45 deg in xz at |beta|=0.05 0.000570169737. Source-sampled local U, not global B. 32/32 fresh tests PASS, clean separate ZIP restore and **14/14 byte-identical science result replay**. This Git note is not a full backup of binary owner science.

Primary source papers: Portsmouth & Bertschinger, astro-ph/0412094 (coherency tensor Thomson); Challinor, Phys Rev D 62 043004 (2000) (covariant polarized transfer); Fleury, Pitrou & Uzan Phys Rev D 91 043511 (2015) (Bianchi I light propagation).

## Protected status
VIGILODE 7/23, REI central 28/88 working 33/88, BASS CE1 9/15, NEW_OWNER_IVPS=0. NO_MERGE, NO_FORCE_PUSH, PRODUCTION=HOLD, AUTO_NEXT_NODE=false. Native subagent not available; distinct-author C9, HOL kernel proof, project-global authority and self-consistent moving-electron Boltzmann hierarchy remain NOT_RUN/UNRESOLVED.