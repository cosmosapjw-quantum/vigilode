"""Longitude-line preconditioning of one shifted spherical diffusion stage."""
from pathlib import Path
import json
import numpy as np
import scipy
from scipy.fft import irfft, rfft
from scipy.linalg import eigvalsh_tridiagonal, solve_banded
from scipy.sparse import eye
from scipy.sparse.linalg import LinearOperator, cg, gmres
from reference import operator


def compare(nt, sigma, kappa=0.5, radius=1.0):
    np_ = 2*nt
    b, p = np.pi/nt, 2*np.pi/np_
    lat = (np.arange(nt)+0.5)*b
    area = 2*radius**2*p*np.sin(lat)*np.sin(b/2)
    longitude = kappa*b/(area*np.sin(lat)*p)
    interface = kappa*np.sin(b*np.arange(1, nt))*p/b
    diagonal = kappa / radius**2 / b / np.tan(b/2)
    off = -sigma*interface/np.sqrt(area[:-1]*area[1:])
    fourier = 4*np.sin(np.pi*np.arange(np_//2+1)/np_)**2
    den = 1 + sigma*longitude[:,None]*fourier[None,:]
    extremal, extrema_pc = [], []
    for col in den.T:
        d = col + sigma*diagonal
        for dest, diag, offdiag in ((extremal, d, off),
                (extrema_pc, d/col, off/np.sqrt(col[:-1]*col[1:]))):
            w = eigvalsh_tridiagonal(diag, offdiag)
            dest.extend([w[0], w[-1]])
    cond_pc = max(extrema_pc)/min(extrema_pc)
    prediction = 1 + 2*sigma*diagonal
    assert abs(cond_pc / prediction - 1) < 1e-12
    # Independent incidence matrix on the physical grid and a nonsmooth RHS.
    n = nt*np_
    system = eye(n,format="csr") - sigma*operator((nt,np_), None, kappa, radius)
    rhs = np.random.default_rng(331+nt).normal(size=n)
    def apply(v):
        mode = rfft(v.reshape(nt,np_), axis=1)
        return irfft(mode/den,n=np_,axis=1).ravel()
    pre = LinearOperator((n,n),matvec=apply,dtype=np.float64)
    work = {}
    for name, prec in (("identity", None), ("longitude_lines", pre)):
        hist = []
        x, info = gmres(system, rhs, M=prec, atol=0, rtol=1e-9, restart=30,
                        maxiter=500, callback=hist.append, callback_type="pr_norm")
        residual = np.linalg.norm(rhs-system@x)/np.linalg.norm(rhs)
        assert info == 0 and residual <= 1.01e-9
        work[name] = dict(arnoldi_columns=len(hist), true_relative_residual=residual)
    # A separate low-storage SPD Krylov alternative in area-scaled coordinates.
    weights = np.repeat(np.sqrt(area), np_)
    symmetric = LinearOperator((n,n), matvec=lambda v: weights*(system@(v/weights)),
                               dtype=np.float64)
    budget = []
    # Tighten the transformed tolerance using norm equivalence so that the
    # physical Euclidean residual is also at most 1e-9 times ||rhs||_2.
    tau = 1e-9*np.min(weights)*np.linalg.norm(rhs)/np.linalg.norm(weights*rhs)
    z, status = cg(symmetric, weights*rhs, M=pre, atol=0, rtol=tau,
                   maxiter=500,callback=lambda _z: budget.append(1))
    residual = np.linalg.norm(rhs-system@(z/weights))/np.linalg.norm(rhs)
    assert status == 0 and residual <= 1.01e-9
    work["weighted_pcg_longitude_lines"] = dict(iterations=sum(budget),
                                               true_relative_residual=residual)
    # With scalar constant diffusivity this geometry allows the complete shifted
    # system to be solved by independent meridional tridiagonal systems.
    root_w = np.sqrt(area)
    modes = rfft(rhs.reshape(nt,np_),axis=1) * root_w[:,None]
    band = np.zeros((3,nt))
    band[0,1:] = band[2,:-1] = off
    for m in range(modes.shape[1]):
        band[1] = den[:,m] + sigma*diagonal
        modes[:,m] = solve_banded((1,1),band,modes[:,m])
    direct = irfft(modes/root_w[:,None],n=np_,axis=1).ravel()
    residual_direct = np.linalg.norm(rhs-system@direct)/np.linalg.norm(rhs)
    assert residual_direct < 1e-11
    work["fourier_tridiagonal_direct"] = dict(meridional_solves=np_//2+1,
                                              true_relative_residual=residual_direct)
    return dict(n_theta=nt,n_longitude=np_,kappa=kappa,radius=radius,sigma=sigma,
                symmetric_condition=max(extremal)/min(extremal),
                preconditioned_condition=cond_pc,predicted_condition=prediction,
                single_stage=work)


if __name__ == "__main__":
    rows = [compare(nt, 0.1) for nt in (8,16,32)]
    out = dict(numpy=np.__version__,scipy=scipy.__version__,rows=rows)
    Path(__file__).with_name("polar_lines.json").write_text(json.dumps(out,indent=2)+"\n")
