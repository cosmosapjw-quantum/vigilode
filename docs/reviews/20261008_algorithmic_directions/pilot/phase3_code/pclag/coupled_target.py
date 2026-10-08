"""Outer-coupled stage target for the matrix-free U-form RODAS5P driver (probe A1; EXPLORATORY pilot,
not ledger authority).

RULE. For stage i = 1..8 of an attempt (t_n, y_n, h_n), accept the Krylov iterate U_i when the true
residual r_i = b_i - W U_i (W = I - h g J) satisfies, in the outer error test's own WRMS metric
(weights D = 1/(atol + rtol*|y_n|), ||v||_w = ||D v||_2 / sqrt(n), i.e. tolerance units):

    ||r_i||_w <= eps_i,

    theta_n = Theta * (h_n / T) * min(1, e_hat_n / e_ref)^p            per-step budget (tol units)
    eps_i   = theta_n / (8 * max(tau_y,i, tau_e,i))                    equal-share allocation
    eps_8   = min(eps_8, kappa8 * e_sat)                               U8 fidelity cap

  T        = t_end - t_0, the integration span (EPUS: "error per unit step" budget)
  e_hat_n  = embedded error estimate of the last accepted step; e0 before the first acceptance
  tau      = sup_{Re z<=0} |T_y,i(z)|, |T_e,i(z)| of the U-form residual-to-output / -to-estimate
             transfer functions, L(z) = (1 - g z) I - g (z A + C), T_y = b_code^T L^-1, T_e = (L^-1)_{8,:}
  e_sat    = (0.9/5)^5 = 1.89e-4 (error at which the I controller's growth factor saturates at 5)
  defaults Theta = 0.2, e_ref = 0.5, p = 6/5, e0 = 1e-6, kappa8 = 0.1      (probe A1 calibration)

Krylov realisation ('scaled' form, recommended): GMRES on D W D^-1 with right-hand side D b, so the
Givens-projected residual equals sqrt(n)*||r||_w exactly; in-cycle stop at
    thr_i = max(eps_i * sqrt(n), RHO_FL * ||D b_i||_2),         RHO_FL = 16 eps_mach (3.6e-15)
confirmed by one true residual; and the attainable-accuracy STALL rule: after a restart cycle that did
not reduce the true residual 4x, accept if ||r|| <= RHO_STALL * (||D b|| + ||D U|| + ||D(U - W U)||),
RHO_STALL = 1024 eps_mach (2.3e-13). No L2 absolute floor and no relative 1e-10 target remain.
Cost of the scaled form: 2n extra multiplies per operator application (D^-1 v, D w).
(The L2-GMRES alternative tests ||r||_2 <= eps*sqrt(n)/max(D) in-cycle and the WRMS true residual;
it is valid but over-solves on badly scaled problems: +3-8% JVP on HIRES/Bruss in probe A1.)

WHY (first principles; first-order, frozen J; von Neumann bound valid when mu_D(hJ) <= 0):
  1. one step: dy_n = sum_i T_y,i(hJ) r_i, d err_n = sum_i T_e,i(hJ) r_i, so
     ||dy_n||_w <= sum_i tau_y,i ||r_i||_w <= theta_n and |d err_n| <= theta_n with the allocation.
  2. many steps: global contamination ||sum_n R(t_N, t_n+1) dy_n||_w <= M * sum_n theta_n <= M * Theta,
     because sum_n h_n / T = 1: independent of rtol and of the number of steps -> tolerance
     proportional, like the exact-solve global error (which is ~kappa*tol, kappa ~ 0.05-4 measured).
     A per-step budget theta gives M*N*theta, N ~ rtol^(-1/5): grows at tight rtol (measured: HIRES
     1e-10/1e-11 fail at theta = 1e-3 per step). A relative target eta*||b|| gives residuals
     eta*||b||_w with ||b||_w ~ rtol^(-0.7) tol units, and the fixed L2 floor g*1e-14 is ~1/rtol tol
     units: both grow as rtol shrinks (the KRY-PROJ-STOP failure mode).
  3. fixed tolerance, h -> 0 (contract ladders): EPUS alone leaves a floor M*Theta; the factor
     min(1, e_hat/e_ref)^(6/5) ~ h^6 makes the summed contamination O(h^6), below the O(h^5)
     truncation, so the direct-LU order is kept (measured: without it diagpr128 is 8-149x LU).
  4. err = ||U_8||_w decides acceptance/growth; the cap keeps it resolved to 10% down to e_sat and
     prevents a zero-iteration U_8 (err = 0, growth x5) (measured: without it HIRES 1.6-1.8x base).
"""
import math
import numpy as np

# sup over Re z <= 0 (a = 0 row of hyp/tau_table.npz; re-verified to 2.8e-5 by verify/math_prior/tau_vn.py)
TAU_Y = np.array([1.729, 0.511, 6.246, 6.081, 3.82, 4.771, 1.011, 1.0])
TAU_E = np.array([1.393, 2.057, 2.266, 2.16, 1.51, 1.792, 2.011, 1.0])
S = 8
E_SAT = (0.9/5.0)**5          # 1.89e-4
EPS_MACH = np.finfo(float).eps
RHO_FL = 16*EPS_MACH          # 3.6e-15, attainable-accuracy guard relative to ||D b||_2
RHO_STALL = 1024*EPS_MACH     # 2.3e-13, stall acceptance relative to the backward-error scale

DEFAULTS = dict(Theta=0.2, epus=True, alloc='equal', p=1.2, e_ref=0.5, e0=1e-6, kappa8=0.1, rho_fl=RHO_FL)


class Target:
    """Coupled WRMS absolute stage target (see module docstring). Parameters:
    Theta  : global contamination budget, tol units (epus=True) or per-step budget (epus=False, INO form)
    alloc  : 'equal' -> theta/(8 max(tau_y, tau_e)); 'uniform' -> theta/sum(tau_y)
    p, e_ref, e0 : order factor min(1, e_hat/e_ref)^p; e0 is e_hat before the first acceptance
    kappa8 : U8 cap eps_8 <= kappa8*E_SAT (None disables);  rho_fl : guard relative to ||D b||_2 (None disables)
    """

    def __init__(self, Theta=0.2, epus=True, alloc='equal', p=1.2, e_ref=0.5, e0=1e-6, kappa8=0.1,
                 rho_fl=RHO_FL):
        self.Theta, self.epus, self.alloc, self.p, self.e_ref, self.e0 = Theta, epus, alloc, p, e_ref, e0
        self.kappa8, self.rho_fl = kappa8, rho_fl
        if alloc == 'equal':
            self.w = 1.0/(S*np.maximum(TAU_Y, TAU_E))
        elif alloc == 'uniform':
            self.w = np.full(S, 1.0/TAU_Y.sum())
        else:
            raise ValueError(alloc)

    def step_budget(self, h, span, err_hat):
        """theta_n (tol units)."""
        e = self.e0 if err_hat is None else err_hat
        phi = min(1.0, max(e, 0.0)/self.e_ref)**self.p if self.p else 1.0
        psi = (h/span) if self.epus else 1.0
        return self.Theta*psi*phi

    def stage_eps(self, h, span, err_hat, n=None):
        """The 8 per-stage WRMS residual targets eps_i (tol units)."""
        eps = self.step_budget(h, span, err_hat)*self.w
        if self.kappa8 is not None:
            eps[-1] = min(eps[-1], self.kappa8*E_SAT)
        return eps

    def l2_threshold(self, eps, Db_norm, n):
        """Threshold on ||D r||_2 (= sqrt(n) * WRMS) for the scaled GMRES, and whether the guard binds."""
        a = eps*math.sqrt(n)
        fl = self.rho_fl*Db_norm if self.rho_fl else 0.0
        return (max(a, fl), fl > a)

    @staticmethod
    def stall_accept(r_norm, r_prev_norm, Db_norm, DU_norm, DU_minus_DWU_norm):
        """Attainable-accuracy stall rule, all norms L2 in the D-scaled system."""
        return (r_prev_norm is not None and r_norm > 0.25*r_prev_norm
                and r_norm <= RHO_STALL*(Db_norm + DU_norm + DU_minus_DWU_norm))


def default_target():
    """The recommended configuration of probe A1."""
    return Target(**DEFAULTS)


def stage_wrms_targets(h, span, err_hat, **kw):
    """Functional form for the integrated-stack probe: the 8 WRMS targets (tol units) of one attempt.
    h: trial step; span: t_end - t_0; err_hat: last accepted embedded error (None before the first)."""
    cfg = dict(DEFAULTS); cfg.update(kw)
    return Target(**cfg).stage_eps(h, span, err_hat)


def scaled_gmres_threshold(eps_i, Db, **kw):
    """L2 threshold for GMRES on D W D^-1 (rhs D b): max(eps_i sqrt(n), RHO_FL ||D b||_2)."""
    n = len(Db)
    return max(eps_i*math.sqrt(n), kw.get('rho_fl', RHO_FL)*float(np.linalg.norm(Db)))


if __name__ == '__main__':
    t = default_target()
    print('weights 1/(8 max tau):', np.round(t.w, 4))
    for h, T, e in [(1e-6, 10.0, None), (0.1, 10.0, 0.5), (0.1, 10.0, 0.02), (1.0, 10.0, 0.6)]:
        print(f'h={h:g} T={T:g} e_hat={e}: theta_n={t.step_budget(h, T, e):.3e} eps={np.array2string(t.stage_eps(h, T, e), precision=2)}')
