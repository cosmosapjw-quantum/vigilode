from __future__ import annotations
import csv, io, json, math, sys, os, unittest
from pathlib import Path
import numpy as np
from numpy.polynomial.legendre import leggauss, legval
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
import lrs_polarization_replay as P

class PhysicallySourceBoundTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.archive=Path(os.environ.get('BASS21_ORIGINAL_ARCHIVE',ROOT/'source'/'VIGILODE_BASS21_FULL25_NATIVE193_20261030_SOURCE_FULL.zip'))
        cls.hist=P.load_owner_archive(cls.archive)
        cls.output=ROOT/'results'

    def test_01_immutable_count_and_full_histories(self):
        self.assertEqual(len(self.hist),50)
        for c,h in self.hist.items():
            self.assertEqual(h['photon_energy_moments'].shape,(193,9))
            self.assertTrue(np.array_equal(h['x'],self.hist['s00_m00_r00']['x']))
            self.assertTrue(np.all(np.diff(h['x'])>0))
            self.assertTrue(np.all(h['g']>=0))

    def test_02_original_clock_and_scale(self):
        for c in ('s00_m00_r00','s04_m04_r04'):
            h=self.hist[c]; self.assertAlmostEqual(h['x'][-1],math.log(7),12)
            self.assertLess(np.max(np.abs(h['g']-P.K_T*h['ne']/h['h'])),2e-16)
            self.assertGreaterEqual(h['tau'][-1],0)

    def test_03_local_gain_GL_direct_tensor(self):
        # Independent dense quadrature of actual owner energy-moment sky;
        # polarization has a nonzero Q even when input radiation is unpolarized.
        mu,w=leggauss(28)
        ph=np.arange(56)*2*np.pi/56
        xyz=np.array([(math.sqrt(1-t*t)*math.cos(p),math.sqrt(1-t*t)*math.sin(p),t)
                      for t in mu for p in ph])
        weights=np.repeat(w*2*np.pi/56,56)
        proj=np.eye(3)[None]-np.einsum('ai,aj->aij',xyz,xyz)
        for case, epoch in [('s00_m00_r00',64),('s04_m02_r02',134),('s04_m04_r04',192)]:
            mom=self.hist[case]['photon_energy_moments'][epoch]
            I=legval(xyz[:,2],mom*(2*np.arange(9)+1))
            M=np.einsum('a,aij->ij',weights*I/2,proj)
            for muv in (0.0,0.3,0.77):
                n=np.array([math.sqrt(1-muv*muv),0,muv]);es=np.array([muv,0,-math.sqrt(1-muv*muv)]);ep=np.array([0,1,0]);screen=np.eye(3)-np.outer(n,n)
                scattered=(3/(8*np.pi))*screen@M@screen
                q=es@scattered@es-ep@scattered@ep
                intensity=np.trace(scattered)
                self.assertLess(abs(q-(-.75*mom[2]*(1-muv*muv))),3e-13)
                self.assertLess(abs(intensity-(mom[0]+.5*mom[2]*.5*(3*muv*muv-1))),3e-13)

    def test_04_ray_direction_and_redshift(self):
        h=self.hist['s04_m04_r04']
        bo=h['b'][192]
        for muo in (0.,0.23,0.63,0.99,1.):
            d2,mux=P.ray_factor(h['b'][[0,50,192]],bo,np.array([muo]))
            self.assertTrue(np.all(np.abs(mux)<=1+1e-15))
            # Source axisymmetric null ray momenta: q_perp=a_o e^-b_o n_o, q_z=a_o e^2b_o mu_o.
            for i,ii in enumerate((0,50,192)):
                dx=h['b'][ii]-bo
                norm=math.sqrt((1-muo*muo)*math.exp(2*dx)+muo*muo*math.exp(-4*dx))
                self.assertAlmostEqual(math.sqrt(float(d2[i,0])),norm,14)
                self.assertAlmostEqual(float(mux[i,0]),muo*math.exp(-2*dx)/norm,14)

    def test_05_vacuum_zero_gain(self):
        x=np.linspace(0,2,193);z=np.zeros_like(x)
        q=P.integrate_los(z,np.ones_like(x),np.sin(x),z,x)
        self.assertTrue(np.array_equal(q,np.zeros_like(q)))

    def test_06_zero_photon_quadrupole(self):
        h=self.hist['s04_m02_r02'];z=np.zeros_like(h['g'])
        q=P.integrate_los(h['g'],z,h['b'],h['tau'],h['x'])
        self.assertTrue(np.array_equal(q,np.zeros_like(q)))

    def test_07_manufactured_FLRW_constant_source(self):
        x=np.linspace(0.,2.,193);gamma=.2;j2=np.full_like(x,-.1);b=np.zeros_like(x);tau=gamma*x
        q=P.integrate_los(np.full_like(x,gamma),j2,b,tau,x)
        expected=-.75*j2[-1]*(1-P.MU**2)*(1-math.exp(-gamma*x[-1]))
        self.assertLess(np.max(abs(q[-1]-expected)),4e-12)
        self.assertTrue(np.array_equal(q[:,-1],np.zeros(193)))

    def test_08_screen_basis_no_parity_odd_U(self):
        # Reflection y -> -y keeps source, ray and interaction invariant.
        h=self.hist['s04_m02_r02'];q=P.integrate_los(h['g'],h['photon_energy_moments'][:,2],h['b'],h['tau'],h['x'])
        self.assertTrue(np.array_equal(q[:, -1],np.zeros(193)))
        self.assertGreater(np.max(abs(q)),1e-4)

    def test_09_exact_swap_shapley_at_multiple_pairs(self):
        for mr in ('m00_r00','m02_r02','m04_r04'):
            a=self.hist['s00_'+mr];b=self.hist['s04_'+mr]
            corners={mask:P.swapped_final(a,b,mask) for mask in range(8)}
            attr=P.exact_three_block_shapley(corners)
            self.assertLess(np.max(abs(sum(attr.values())-(corners[7]-corners[0]))),4e-17)

    def test_10_published_results_match_current_replay(self):
        data=np.load(self.output/'LRS_50x193x17_POLARIZATION.npz')
        case=data['cases'].tolist().index('s04_m02_r02');h=self.hist['s04_m02_r02']
        result=P.integrate_los(h['g'],h['photon_energy_moments'][:,2],h['b'],h['tau'],h['x'])
        self.assertTrue(np.array_equal(result,data['Q'][case]))
        self.assertTrue(np.array_equal(data['Q'][:,0,:],np.zeros((50,17))))
        self.assertTrue(np.array_equal(data['Q'][:,:,-1],np.zeros((50,193))))

    def test_11_brightness_strictly_positive(self):
        data=np.load(self.output/'LRS_50x193x17_POLARIZATION.npz')
        self.assertTrue(np.all(data['I_baseline']>0))
        self.assertTrue(np.all(abs(data['Q'])<data['I_baseline']))

    def test_12_contrast_all_25_matched_original(self):
        d=np.load(self.output/'LRS_50x193x17_POLARIZATION.npz');q=d['Q'];cases=d['cases'].tolist();contrasts=[]
        for m in range(5):
            for r in range(5):
                ia=cases.index(f's00_m{m:02d}_r{r:02d}');ib=cases.index(f's04_m{m:02d}_r{r:02d}')
                contrasts.append(q[ib,-1,0]-q[ia,-1,0])
        self.assertEqual(len(contrasts),25)
        self.assertTrue(np.all(np.asarray(contrasts)>0))
        self.assertTrue(np.array_equal(np.asarray(contrasts),d['pair_delta_Q'][:,-1,0]))

    def test_13_metadata_no_source_migration(self):
        summ=json.loads((self.output/'SCIENCE_SUMMARY.json').read_text())
        self.assertEqual(summ['input_archive_sha256'],P.ARCHIVE_SHA)
        self.assertEqual(summ['original_atomic_network_sha256'],P.ATOM_SHA)
        self.assertEqual(summ['new_original_geometry_chemistry_IVPs'],0)
        self.assertEqual(summ['new_polarized_ivps'],0)
        self.assertEqual(summ['stokes_Q_saved_model_rows'],50*193*17)

if __name__=='__main__':unittest.main()
