# Web source map for coding loop 2

Primary-source anchors used to set comparison boundaries:

1. G. Steinebach, *Construction of Rosenbrock–Wanner method Rodas5P and numerical benchmarks within the Julia Differential Equations package*, BIT Numer. Math. 63, 27 (2023), DOI `10.1007/s10543-023-00967-x` — protected RODAS5P order, stiff and index-1 DAE test context.
2. P. J. van der Houwen and B. P. Sommeijer, *Iterated Runge–Kutta Methods on Parallel Computers*, SIAM J. Sci. Stat. Comput. 12 (1991), DOI `10.1137/0912054` — finite parallel iteration depth and convergence to corrector stability/accuracy.
3. G. Čaklović et al., *Improving Efficiency of Parallel Across the Method Spectral Deferred Corrections*, SIAM J. Sci. Comput. 47 (2025), DOI `10.1137/24M1649800` — diagonal parallel sweeps, coefficient-sensitive convergence, nilpotent/nonstationary error propagation.
4. P. Munch et al., *Stage-Parallel Fully Implicit Runge–Kutta Implementations with Optimal Multilevel Preconditioners at the Scaling Limit*, SIAM J. Sci. Comput. 46 (2024), DOI `10.1137/22M1503270` — stage-parallel block solves, matrix-free PDE context, batching alternative, speedup bounded by stage count.
5. D. A. Brown and D. W. Zingg, *Matrix-free monolithic homotopy continuation with application to computational aerodynamics*, Numer. Algorithms 78 (2018), DOI `10.1007/s11075-017-0424-8` — matrix-free homotopy as nonlinear globalization prior art.

These sources motivate the next branch but do not validate the present affine oracle; its claims rest on code tests and independent numerical reconstruction.
