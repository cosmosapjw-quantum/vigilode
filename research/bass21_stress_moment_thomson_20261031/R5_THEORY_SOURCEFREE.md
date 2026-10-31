# Source-free BASS21 stress-moment reduction | R5 derived

**Status:** EXPLORATORY/POST-HOC, not owner-source, not a canonical DAG node. Depends on PR161 HEAD \`1219495f3e89fbeacee3f825f845687827070a16\` and immutable PR159 HEAD \`0c996dfb18cec764c457ed9d9ff667d1642e3ed6\`. The PR159 prospective preregistration is **not** applicable to this new cohort.

Signature (-,+,+,+), x⁰=ct; diagonal Bianchi I ds²=-c²dt²+a² Σ exp(2b_i)dX_i²; b_i=s_i x, x=ln(a/a_emit), Σs_i=0. For initially normal-frame isotropic, **unpolarized**, collisionless radiation the bolometric angular brightness at a prescribed x is F(n,s,x)=[Σ n_i²exp(2s_i x)]^-2, relative to I_iso(x)=I_emit exp(-4x). This is an analytic ray model, **not** original BASS owner data/IVP.

Define R^{ab}=c T^{ab}=∫ I(n) ℓ^aℓ^b dΩ (ℓ=(1,n)), with intensity units. Electron cold monovelocity β=v/c, |β|<1, Lorentz transformation Λ from normal to instantaneous electron-rest tetrad, R'=ΛRΛᵀ. In that rest tetrad, the Thomson coherency gain for unpolarized incoming intensity is

G'_{AB} = (3/(16π)) [ R'^{00} δ_{AB} - e_A'^i R'_{ij} e_B'^j ].

With Q'=G'11-G'22, U'=2G'12 and m=e1'+i e2', the exact local gain identity is

(Q'+iU')_gain = -(3/(16π)) m_i m_j R'^{ij}.

The collision integral depends **only on incoming intensity moments ℓ≤2** at that instant because ∫ I(n)ℓ^aℓ^b dΩ contains no higher harmonics. This does **not** truncate photon streaming, nor hold for arbitrary polarized incoming radiation.

Axis witness: I/I0=1+d P1(μ)+q P2(μ)+o P3(μ), β=β ẑ, outgoing electron-rest direction x̂, screen (e1,e2)=(ẑ,-ŷ). Angular tensor moments are R00/(4πI0)=1, R0z/(4πI0)=d/3, Rzz/(4πI0)=1/3+2q/15, Rxx/(4πI0)=1/3-q/15. Therefore

Q'_gain/I0 = [-3q/20+βd/2-β²(1-q/20)]/(1-β²),  U'_gain=0.

For d=q=0, Q'=-I0 γ²β², consistent with the restricted all-β moving-electron prior in PR161. The coefficient o cancels exactly, even for finite β. Symbolic equivalence checked by SymPy and a separate Wolfram evaluation with exact zero residual. For small anisotropy and β=0, (Q+iU)'/I_iso = (2/5) x m_i m_j diag(s_i)_{ij}+O(x²).

**Scope omissions:** The electron-normal full collision requires frequency dependence, Lorentz screen rotation, optical-depth normalization, gain/loss, plasma evolution and original owner source. No observed E/B, primordial tensor, original 50×193 BASS-owned IVP or full polarized incoming collision has been generated. No claim of scientific novelty.

Primary related literature: Portsmouth & Bertschinger astro-ph/0412094, Challinor PRD 62 043004 (2000), Fleury/Pitrou/Uzan PRD 91 043511 (2015).

**Progress unchanged:** VIGILODE=7/23; REI central=28/88 working=33/88; BASS CE1=9/15; NEW_OWNER_IVPS=0; PRODUCTION=HOLD, NO_MERGE, AUTO_NEXT_NODE=false.