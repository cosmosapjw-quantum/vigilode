# BASS21 original-source frozen LRS E₂ normalization, spin-2 multipoles, and shear transfer

**Sources**: Original BASS `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`; 50×193 archived original same-62-state histories, original archive SHA-256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`. Parent VigilODE PR133 HEAD `24a0c534adca54c700bd050cd8358ce3638e3756`; parent source-inclusive ZIP SHA-256 `ffd7623ba2e2bd99233a1244ac7945470f6eeed348a63a4fd02f5967ad449b14`. New preregistration `691180299a043393a42efdad473f4914d1108772` was published before full 50-case run. Earlier preliminary derivation and prior parent results were not retrospectively preregistered.

Metric \((- + + +)\), normal-comoving cold Thomson electrons, source frozen `b(x), H(x), ne(x), tau(x), Jℓ(x)` and x=ln(a/a_ref), c preserved through g=c σ_T ne/H. Parent PR132 solved source-frozen I=I_base+δI and Q=I_meridional−I_azimuthal; no IVPs, no replacement source/event, U=V=0 by axial LRS parity.

## 1. Source-native *collision* normalization

Unpolarized BASS baseline `lrs_boltzmann.py` blob `e15e1875fba7fcc4a42e257e291c54d9b540a3f0` explicitly records the intended full collision quadrupole `K₂=−Γ(9F₂/10+√6 E₂/10)`, while its actual original evolution sets E₂=0. Parent PR132 exact polarization-corrected intensity source is

\[δS_I=j_0+[j_2/2−3B_Q/8]P_2(μ),\quad B_Q=2\int_0^1(1−μ^2)Q(μ)dμ.\]

Since `J₂=<I P₂>`, collisional angular projection gives

\[ C_{J₂} = -g\left(\frac9{10}j_2+\frac3{40}B_Q\right). \]

Define \(E_2^{\rm cand}=3B_Q/(4\sqrt6)\), thereby obtaining exactly

\[C_{J₂}=-g\left(\frac9{10}j_2+\frac{\sqrt6}{10}E_2^{\rm cand}\right).\]

This is a **same-source collision-operator coefficient crosswalk**, not yet an equivalence of the frozen baseline model to a fully evolved BASS owner E hierarchy.

## 2. Spin-2 even LRS sky and higher moments

For \(\ell=2,4,6,8,10\), define \(P_\ell^2(μ)=(1−μ^2)\partial_μ^2P_\ell(μ)\), the positive associated Legendre convention for m=2, and

\[c_\ell(x)=\frac{\int_0^1Q(x,μ)P_\ell^2(μ)dμ}{N_\ell},\quad N_\ell=\frac{(\ell+2)!}{(2\ell+1)(\ell−2)!}.\]

The \(c_\ell\) are coefficients of the spin-2 angular basis in this screen convention. Only the \(E_2^{\rm cand}\) collision normalization above is asserted to correspond to BASS's indicated quadrupole coefficient. The higher \(c_\ell\) are not called BASS owner Eℓ or CMB temperature-mode harmonics.

\[c_2=\frac5{16}B_Q,\qquad E_2^{\rm cand}=\frac{12}{5\sqrt6}c_2.\]

Using \(P_4^2=(15/2)(7μ^2−1)(1−μ^2)\) gives the exact identity

\[C_Q=2\int_0^1μ^2(1−μ^2)Qdμ=\frac17 B_Q+\frac{32}{21}c_4.\]

## 3. Geometric-shear/Thomson coupled E2 law

PR132 normal-screen Liouville transfer is \(∂_xQ=3b'μ(1−μ^2)∂_μ Q+4b'(1−3μ^2)Q+g(S_Q−Q)\), with \(S_Q=[−3(J₂^{(0)}+j₂)/4+9B_Q/16](1−μ^2)\). Integrating \(2\int_0^1(1−μ^2)(...)dμ\) by parts, with no boundary term, yields

\[\frac{dB_Q}{dx}=b'\left(\frac{10}7B_Q+\frac{32}7c_4\right)-g\left[\frac45(J₂^{(0)}+j₂)+\frac25B_Q\right].\]

Therefore

\[\boxed{\frac{dE_2^{\rm cand}}{dx}=b'\left(\frac{10}7E_2^{\rm cand}+\frac{4\sqrt6}7c_4\right)-g\left[\frac{\sqrt6}{10}(J₂^{(0)}+j₂)+\frac25E_2^{\rm cand}\right].}\]

One cannot truncate at E2 under nonzero anisotropic shear without explicitly approximating c4. Homogeneous b'=0 collision matrix for \((j₂,E₂)\) has eigenvalues `−1`, `−3/10`, both negative (Wolfram exact). LRS Type I screen has no B-mode mixing for parity-even axisymmetric source under adopted transported basis; no physical observed CMB E/B posterior follows.

## 4. Scientific errors, approximations and independent tests

Original BASS stored 193 samples of a frozen **unpolarized L≤8** Boltzmann solution. New Volterra field uses the same source and exact 62-state b/g/tau/J2; full source/metric and chemistry remain frozen; no Einstein/shear or electron/thermal response is fed back. E2 transport is a *model-conditional* derived equation. Numerical derivatives use `np.gradient` on nonuniform saved epochs and `cumulative_simpson`, so nonzero local budget residual and integrated deficit must be preserved, not portrayed as global BDF precision. Primary comparisons: exact analytic B_Q crosswalk and Wolfram, GL64/96 angular projection; original PR133 B_Q and j2 arrays independent readback; positivity Q≤I; 9 local tests, negative parent SHA tamper.

## References

- Challinor (2000), *Microwave background polarization in cosmological models*, Phys Rev D **62**, 043004, DOI `10.1103/PhysRevD.62.043004`; arXiv:astro-ph/9911481. Paper uses metric (+−−−) so translate conventions.
- Pontzen & Challinor (2007), *Bianchi model CMB polarization and its implications for CMB anomalies*, MNRAS **380**, 1387–1398, DOI `10.1111/j.1365-2966.2007.12221.x`, arXiv:0706.2075. Supports special Type I no E/B mixing under their near-FRW assumptions; not a proof of our finite-anisotropy numerical result.
- Anile (1974), *Anisotropic expansion of the universe and the anisotropy and linear polarisation of the cosmic microwave background*, Ap&SS **29**, 415 onward, DOI `10.1007/BF02639073`.
