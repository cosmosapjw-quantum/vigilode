"""Problem registry for PROBE B2 (EXPLORATORY). Every problem is a dict with
  f(t,y), J(t,y) -> scipy.sparse.csr (exact; the Krylov 'JVP' is J@v, counted as one analytic JVP), ft or None,
  y0, span, ascale (atol = ascale*rtol), n, nnzJ, colors (CPR column colouring of J), h0,
  F_rhs, F_jvp, F_ft (state flops per call: the JVP COST MODEL, explicit per problem, see COST below),
  ref (endpoint reference), err(y) -> endpoint error, err_name, family tag.
COST MODEL (flops per call, n = dimension; TRIG = 20 flops per sin/cos/exp):
  corpus-v2 families: F_jvp = corpus_v2.JVP_COST_MODEL (robertson 8.33n, hires 6n, vdp 5.5n, rotating 14n + 1 trig/comp,
      forcing 8n + 1 trig/comp, semilinear 22n); F_rhs = F_jvp plus the manufactured terms named in corpus_v2
      (rotating + 4 trig/comp, semilinear + n + (nx+ny) trig); F_ft = F_rhs.
  brusselator-1d-N (harness): F_rhs 11n, F_jvp 12n (probs2d.bruss1d); bruss2d-N: F_rhs 23N^2, F_jvp 24N^2 (n = 2N^2).
  advdiff-128 (nonnormal stress, non-autonomous, exact solution): F_rhs 14n, F_jvp 9n, F_ft 14n.
Endpoint error rules: corpus -> tight-basis WRMS (weights 1e-10 + 1e-8|ref|, corpus_v2 rule) of the final state;
  harness Brusselators / bruss2d / advdiff -> componentwise max relative error with 1e-10 floor."""
import json
import math
import os

import numpy as np
import scipy.sparse as sp

import corpus_v2 as cv
import probs2d

HERE = os.path.dirname(os.path.abspath(__file__))
TRIG = 20.0
CORPUS_SHORT = {'rob': 'robertson-ramped', 'hires': 'hires-ramped', 'vdp': 'van-der-pol-ramped',
                'rot': 'rotating-nonnormal', 'forc': 'nonautonomous-stiff-forcing',
                'semi': 'semilinear-advection-diffusion-ramped'}


def maxrel(y, ref):
    return float(np.max(np.abs(y - ref) / np.maximum(np.abs(ref), 1e-10)))


def tight_wrms(y, ref):
    return float(math.sqrt(np.mean(((y - ref) / (cv.TIGHT_WRMS_ABS + cv.TIGHT_WRMS_REL * np.abs(ref))) ** 2)))


def corpus(short, n=96):
    fam = CORPUS_SHORT[short]
    q = cv.build(fam, n)
    Js = q.J_sparse
    J0 = Js(q.span[0], q.y0)
    # structural CPR colouring: union of the patterns along the reference (explicit zeros at y0 are not structural
    # zeros: Robertson colours as 1 at y0 but 3 structurally); semilinear uses the classical 5-colouring if valid
    rr = q.exact_reference() if q.exact is not None else q.reference()
    pat = None
    for k in (0, 25, 50, 75, 100):
        P = (abs(Js(rr['times'][k], rr['states'][k])) > 0).astype(float)
        pat = P if pat is None else ((pat + P) > 0).astype(float)
    nc, _ = probs2d.greedy_colors(pat)
    if short == 'semi':
        nx, ny = q.grid_shape if q.grid_shape else cv.SEMILINEAR_GRIDS[n]
        ii, jj = np.meshgrid(np.arange(nx), np.arange(ny))
        col5 = ((ii + 2 * jj) % 5).ravel()
        if probs2d.check_coloring(pat, col5):
            nc = 5
    m = cv.JVP_COST_MODEL[fam]
    Fj = (m['flops_per_component'] + TRIG * m['transcendentals_per_component']) * n
    Fr = Fj
    if short == 'rot':
        Fr += 4 * TRIG * n
    if short == 'semi':
        nx, ny = q.grid_shape if q.grid_shape else cv.SEMILINEAR_GRIDS[n]
        Fr += n + (nx + ny) * TRIG
    if q.exact is not None:
        ref = np.array(q.exact(q.span[1])); ref_src = 'exact'
    else:
        r = q.reference(); ref = np.array(r['states'][-1]); ref_src = r.get('source', 'stored')
        assert abs(r['times'][-1] - q.span[1]) < 1e-12
    span = q.span
    return dict(name=f'{short}-{n}', family=fam, f=q.f, J=lambda t, y: Js(t, y).tocsr(), ft=q.ft, y0=q.y0, span=span,
                ascale=cv.ATOL_FACTOR, n=n, nnzJ=int(J0.nnz), colors=nc, h0=(span[1] - span[0]) / 100.0,
                F_rhs=float(Fr), F_jvp=float(Fj), F_ft=float(Fr), ref=ref, ref_src=ref_src,
                err=lambda y, ref=ref: tight_wrms(y, ref), err_name='tight-WRMS endpoint', kind='corpus',
                jvp_fn=q.jvp)


def bruss1d(cells):
    p = probs2d.bruss1d(cells)
    n = p['n']
    if cells == 50:
        ref = np.array(json.load(open('/home/user/wt-speed/research/stiff_native_benchmark_20261001/NATIVE.json'))
                       ['references']['brusselator-1d-50']['final_state'])
        src = 'NATIVE.json'
    else:
        fn = os.path.join(HERE, 'refs', f'bruss1d-{cells}.npz')
        if os.path.exists(fn):
            z = np.load(fn); ref = z['y_r13']; src = 'direct RODAS5P rtol=atol=1e-13 (refs/)'
        else:
            ref = None; src = None
    return dict(name=f'bruss1d-{cells}', family='brusselator-1d', f=p['f'], J=p['J'], ft=None, y0=p['y0'],
                span=p['span'], ascale=1.0, n=n, nnzJ=p['nnzJ'], colors=p['colors'], h0=1e-6,
                F_rhs=11.0 * n, F_jvp=12.0 * n, F_ft=0.0, ref=ref, ref_src=src,
                err=(lambda y, ref=ref: maxrel(y, ref)), err_name='max rel (1e-10 floor) endpoint', kind='harness')


def bruss2d(N):
    p = probs2d.bruss2d(N)
    z = np.load(os.path.join(HERE, 'refs', f'bruss2d-{N}.npz'))
    ref = z['y_r13']
    return dict(name=f'bruss2d-{N}', family='brusselator-2d-offcontract', f=p['f'], J=p['J'], ft=None, y0=p['y0'],
                span=p['span'], ascale=1.0, n=p['n'], nnzJ=p['nnzJ'], colors=p['colors'], h0=1e-6,
                F_rhs=float(p['frhs']), F_jvp=float(p['fjvp_state']), F_ft=0.0, ref=ref,
                ref_src='direct RODAS5P 1e-13 (pclag refs; Radau agrees 1.3e-13)',
                err=(lambda y, ref=ref: maxrel(y, ref)), err_name='max rel (1e-10 floor) endpoint', kind='2d')


def advdiff(n=128, D=0.01, a=5.0, r=-1.0, nl=10.0):
    dx = 1.0 / (n + 1); x = np.arange(1, n + 1) * dx
    A = sp.diags([(D / dx ** 2 + a / dx) * np.ones(n - 1), (-2 * D / dx ** 2 + r - a / dx) * np.ones(n),
                  (D / dx ** 2) * np.ones(n - 1)], [-1, 0, 1], format='csr')
    sx = np.sin(np.pi * x)
    phi = lambda t: math.exp(-t) * sx
    f = lambda t, y: A @ (y - phi(t)) - phi(t) + nl * (y - phi(t)) ** 3
    J = lambda t, y: (A + sp.diags(3 * nl * (y - phi(t)) ** 2)).tocsr()
    ft = lambda t, y: A @ phi(t) + phi(t) - 3 * nl * (y - phi(t)) ** 2 * (-phi(t))
    ref = phi(1.0)
    return dict(name=f'advdiff-{n}-D{D:g}-a{a:g}', family='advdiff-nonnormal', f=f, J=J, ft=ft, y0=phi(0.0),
                span=(0.0, 1.0), ascale=1.0, n=n, nnzJ=int(A.nnz), colors=3, h0=1e-6,
                F_rhs=14.0 * n, F_jvp=9.0 * n, F_ft=14.0 * n, ref=ref, ref_src='exact',
                err=(lambda y, ref=ref: maxrel(y, ref)), err_name='max rel (1e-10 floor) endpoint', kind='nonnormal',
                A=A)


def build(name):
    """'rob-96','hires-96','vdp-96','rot-96','forc-96','semi-96','semi-384', 'bruss1d-50','bruss1d-160',
    'bruss2d-32','bruss2d-64', 'advdiff-128' (D=0.01, a=5), 'advdiff-128-mild' (D=0.05, a=0.5)"""
    parts = name.split('-')
    if parts[0] in CORPUS_SHORT:
        return corpus(parts[0], int(parts[1]))
    if parts[0] == 'bruss1d':
        return bruss1d(int(parts[1]))
    if parts[0] == 'bruss2d':
        return bruss2d(int(parts[1]))
    if parts[0] == 'advdiff':
        if name.endswith('mild'):
            return advdiff(int(parts[1]), D=0.05, a=0.5)
        return advdiff(int(parts[1]))
    raise ValueError(name)
