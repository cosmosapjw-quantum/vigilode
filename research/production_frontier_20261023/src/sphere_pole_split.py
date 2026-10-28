#!/usr/bin/env python3
"""Second-level research expansion: exact longitude heat flow + RKL2 latitude.

This is a fixed linear spherical diffusion prototype, not an ODE API default.
Spatial coefficients and references come from the separately audited FV module.
"""
import argparse, hashlib, json, math, pathlib, unittest
import numpy as np
from pde_frontier import Graph, sphere_grid, sphere_x_reference, rkl2, rkl2_step


def latitude_graph(ntheta, nphi):
    g, faces, centers, widths = sphere_grid(ntheta,nphi)
    mask = (g.right-g.left)==nphi
    return Graph(g.weights,g.left[mask],g.right[mask],g.conductance[mask],g.name+'_latitude'),centers


def longitude_multiplier(centers,nphi,h):
    dp=2*math.pi/nphi
    return np.exp(-h*4*np.sin(math.pi*np.arange(nphi//2+1)/nphi)[None,:]**2/
                  (dp*dp*(1-centers[:,None]**2)))


def longitude(y, multiplier, nphi):
    return np.fft.irfft(np.fft.rfft(y.reshape(-1,nphi),axis=1)*multiplier,n=nphi,axis=1).reshape(-1)


def split(ntheta,nphi,y,t,steps):
    if not math.isfinite(t) or t<0 or not isinstance(steps,int) or steps<1:
        raise ValueError('finite nonnegative time and positive integer steps')
    g,mu=latitude_graph(ntheta,nphi)
    h=t/steps
    mult=longitude_multiplier(mu,nphi,h/2)
    out=np.array(y,dtype=float,copy=True)
    count=0
    for _ in range(steps):
        out=longitude(out,mult,nphi)
        out,stages=rkl2_step(g,out,h)
        out=longitude(out,mult,nphi)
        count+=stages
    return out,{'latitude_actions':count,'fft_pairs':2*steps,'coefficient_builds':1,
                'latitude_q':g.q,'macrosteps':steps}


class Contracts(unittest.TestCase):
    def test_longitude_positive_mass_and_discrete_eigenmode(self):
        g,_,mu,_=sphere_grid(8,16)
        y=np.zeros(128);y[0]=1
        out=longitude(y,longitude_multiplier(mu,16,.1),16)
        self.assertGreaterEqual(float(out.min()),-2e-16)
        self.assertAlmostEqual(float(g.weights@out),float(g.weights@y),places=14)
        self.assertLessEqual(g.norm2(out),g.norm2(y)+1e-14)
    def test_strang_converges_and_preserves_mass(self):
        g,y,ref,_,_=sphere_x_reference(8,16,.05)
        errs=[]
        for steps in [8,16,32]:
            out,_=split(8,16,y,.05,steps)
            errs.append(g.norm2(out-ref))
            self.assertLess(abs(g.weights@(out-y)),1e-12)
            self.assertLessEqual(g.norm2(out-1),g.norm2(y-1)+1e-12)
        self.assertGreater(errs[0]/errs[1],3)
        self.assertGreater(errs[1]/errs[2],3)
    def test_constant_state_and_zero_time(self):
        y=np.ones(128)
        out,_=split(8,16,y,.7,3)
        np.testing.assert_allclose(out,y,rtol=0,atol=2e-14)
        out,_=split(8,16,np.arange(128.),0,1)
        np.testing.assert_allclose(out,np.arange(128.),rtol=0,atol=1e-12)


def campaign():
    rows=[]
    for nt in [8,16,24]:
        np_=2*nt;t=.05
        g,y,ref,continuum,oracle=sphere_x_reference(nt,np_,t)
        glat,_=latitude_graph(nt,np_)
        for steps in [1,2,4,8,16,32,64]:
            for mode in ['full_rkl2','longitude_split_rkl2']:
                if mode=='full_rkl2':
                    out,work=rkl2(g,y,t,steps)
                    work={'full_graph_actions':work['operator_actions'],'fft_pairs':0,'macrosteps':steps}
                else:out,work=split(nt,np_,y,t,steps)
                rows.append({'candidate':mode,'ntheta':nt,'nphi':np_,'cells':len(y),'time':t,
                    'steps':steps,'q_full':g.q,'q_latitude':glat.q,
                    'relative_l2_error':g.norm2(out-ref)/g.norm2(ref),
                    'relative_spatial_error':g.norm2(ref-continuum)/g.norm2(continuum),
                    'mass_defect':abs(float(g.weights@(out-y))), 'minimum':float(out.min()),
                    'oracle_action_residual':oracle,**work})
    return {'schema':'vigilode.frontier-pole-split.v1','source_sha256':hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),
            'scope':'linear fixed sphere FV, smooth cosphi mode; accuracy/work counts, no elapsed speed claim',
            'parent':'PDE.RKL2','children':['PDE.RKL2.FULL','PDE.RKL2.POLE_SPLIT'],
            'error_budget':1e-5,'rows':rows,'limitation':'Splitting commutator constants depend on grid; no uniform second-order PDE theorem or component positivity for RKL2.'}

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--test',action='store_true');p.add_argument('--output');a=p.parse_args()
    if a.test:unittest.main(argv=['sphere_pole_split'],exit=True)
    result=campaign()
    if a.output:pathlib.Path(a.output).write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'rows':len(result['rows']),'scope':result['scope']}))
