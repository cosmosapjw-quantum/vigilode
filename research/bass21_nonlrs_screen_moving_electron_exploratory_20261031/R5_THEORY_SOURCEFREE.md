# VigilODE BASS21 exploratory source-free non-LRS screen and moving-electron Thomson | R5

**EXPLORATORY, post-hoc, NOT original owner or canonical DAG node.** This branch was based on the exact untouched PR #159 head `0c996dfb18cec764c457ed9d9ff667d1642e3ed6`. PR159 prereg `4e2e976dce0c01ce6092b32eb1588826a61383bc` does not preregister this new exploratory experiment.

Metric (-,+,+,+): ds2=-c2 dt2+a2 sum_i exp(2 b_i) (dX_i)2, trace(b)=0; x=ln(a/a_ref), experimental b_i=s_i*x, sum s_i=0. Exact diagonal Bianchi I photon ray:
- n_i=p_i exp(-b_i)/sqrt(sum_j p_j2 exp(-2b_j)); p_i Killing-conserved.
- n_i'=(sum_j s_j n_j2-s_i)n_i; d ln(aE)/dx=-sum_j s_j n_j2.
- screen frame (e_theta,e_phi) has omega_x=n_z(s_x-s_y)n_x n_y/(n_x2+n_y2), excluding n_z poles of this chart.
- collisionless bolometric parallel transport: (d/dx+2i omega_x)(P/E4)=0 for P=Q+iU. Connection itself depends on screen gauge.

Thomson incident unpolarized radiation in moving-electron local rest frame:
- D=gamma(1-beta dot n_lab), E'=D E, I'_bol=D4 I_bol.
- J'ij=(I'/2)(delta_ij-n'_i n'_j), M_ij=integral J'ij dOmega'.
- electron-rest gain G'AB=3/(8pi) e'_A^i M_ij e'_B^j.
- normal-frame instantaneous collision per c n_e' sigma_T dt_normal is D*(R G' R^T/D4-I_lab*1_2/2); R is the boost-screen SO(2) matrix with null gauge restoration.
- For *isotropic lab unpolarized bolometric* I0 specifically, the finite-velocity exact closed form is
  (Q+iU)'_gain=-I0 gamma2 (beta_screen_1+i beta_screen_2)2
  with gamma2=1/(1-beta2), |beta|<1.
- Exact quadrupole integral: integrate_{-1}^1 P2(mu)/(1+beta mu)^4 dmu=8 beta2/[3(1-beta2)^3]. SymPy and independent Wolfram both evaluated the identity.

This is **not** generic non-LRS Bianchi class A/B, polarized incoming collision, multiscattering, Einstein-Boltzmann time IVP, original BASS owner computed result, observed E/B or new primordial tensors. No claim of novelty or independent author referee.

**Official project state (unchanged)**: VigilODE 7/23, REI central 28/88, working 33/88, BASS CE1 9/15, owner IVPs 0; `PRODUCTION=HOLD`, no merge, force push, owner source replacement or auto-next-node.

Executable sources, controlled numeric samples, all figures, tests and proof search are preserved in source-free binary ZIP on Google Drive:
https://drive.google.com/file/d/1sjUrqWfyOv4NzHwStc7MyC9l8eO2xrAH/view
Archive exactly **422174 bytes**, SHA256 `8797aac0207095a8e8aa2a004c487a42d18922aee8c6e4f55b1a8472fd908641`. A remote raw Drive readback reproduced this SHA byte-exact; full cleanroom restore and local replay verified separately. Not original source archive.

Primary related papers: Fleury–Pitrou–Uzan PRD91 043511 (2015); Challinor PRD62 043004 (2000); Pontzen & Challinor MNRAS380 (2007); Pitrou CQG26 065006 (2009).
