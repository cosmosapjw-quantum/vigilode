# Dense angular gain and periodic wraps through the existing border API

Let the phase block be A+UV^T with banded invertible A and U,V in R^(n×r). For an additional matter block B,C,D, introduce z=V^T u. The enlarged system is

    [ A   B   U ] [u]   [f]
    [ C   D   0 ] [v] = [g]
    [-V^T 0   I ] [z]   [0].

Eliminating its last row gives exactly the original phase/matter system. Hence the delivered `BorderedBandedPreconditioner` represents this system with phase n and border m+r, input B_ext=[B,U], C_ext=[C;-V^T], D_ext=diag(D,I_r). No change to the factorization API is required. Zero auxiliary RHS is part of the contract; an approximate U,V requires the original operator residual.

The construction is an implementation mapping of the classical low-rank/Schur identity, not a novelty claim. Its cost follows `SCHUR_THEORY.md` with m replaced by m+r. Fixed r,m,b gives linear n dependence. A periodic nearest-neighbour one-dimensional stencil has two wrap entries representable with r≤2; test the actual geometry and signs before binding it to a stage matrix.

For the LRS unpolarized Thomson kernel in `BASS_REI_PORT_RESEARCH.md`, rank is two **per energy group**. Across N_E independent groups the full gain rank can be 2N_E. Do not put every such group into a supposedly fixed-size global dense Schur block. Better use exact/local angular low-rank inverses per energy group, combine them with energy transport block solves, and leave only the few matter integrals in the global border. The low-rank benefit is O(N_mu) local scattering application rather than O(N_mu²), under the stated cold elastic kernel. Energy redistribution or polarization requires its own factorization.

This extension is algebraically derived and independently checked by substitution in this review. A dedicated native augmented-border/periodic client test is pending; the existing native tests cover general B,C,D block solves and an off-band perturbation through actual GMRES.
