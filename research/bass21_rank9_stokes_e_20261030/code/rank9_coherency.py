#!/usr/bin/env python3
"""Source-owned BASS21 rank-9 cold-Thomson coherency and screen-fixed harmonic E.

On 50 immutable BASS21 193-epoch histories, re-evaluate the pre-existing
source-frozen coupled Stokes deltaI/Q Volterra field (NOT an IVP).  Compare the
frame-free rank-nine Thomson gain with direct Stokes source and extract spin-2
E_l0 in one explicit local transported-screen convention.  NOT owner full E_l,
observed CMB polarization or Einstein backreaction.
"""
from __future__ import annotations
import argparse, csv, hashlib, json, math, pathlib, sys
import numpy as np
from numpy.polynomial.legendre import leggauss
from scipy.special import lpmv

ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'source_code'))
from coupled_thomson_volterra import solve_case, load_owner_archive

OWNER_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
PR138_SHA='ab7044f604fdd7badb7eb21bae60f7900dd47e1e81b0ed1cce3091283162309f'
LC=np.arange(2,17,2,dtype=int)
E_FACTOR=np.array([np.sqrt(4*np.pi/(2*l+1))*np.sqrt(math.factorial(l+2)/math.factorial(l-2)) for l in LC])
N_FACTOR=np.array([math.factorial(l+2)/((2*l+1)*math.factorial(l-2)) for l in LC])
MU_GAIN=np.array([0.0,0.25,0.5,0.75,1.0])


def sha(p):
    with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()


def glhalf(order):
    x,w=leggauss(order);return .5*(x+1),.5*w


def moments(I,Q,mu,w):
    """Direct independent I/Q scalar integral on the positive hemisphere."""
    p2=.5*(3*mu**2-1)
    j0=I@w
    j2=I@(w*p2)
    bq=2*Q@(w*(1-mu**2))
    return j0,j2,bq


def direct_tensor(I,Q,mu,w):
    """Actual M components from coherent photon projectors, azimuth integrated."""
    j0=I@w
    zz=2*np.pi*((I+Q)@(w*(1-mu**2)))
    perp=(4*np.pi*j0-zz)/2
    return perp,zz


def tensor_from_moments(j0,j2,bq):
    return (4*np.pi*(j0/3+j2/6-bq/8),
            4*np.pi*((j0-j2)/3+bq/4))


def gains_from_tensor(perp,parallel,mu=MU_GAIN):
    mu=np.asarray(mu,float)
    si=3/(8*np.pi)*(perp[:,None]*(1+mu[None,:]**2)+parallel[:,None]*(1-mu[None,:]**2))
    sq=3/(8*np.pi)*(parallel-perp)[:,None]*(1-mu[None,:]**2)
    return si,sq


def gains_from_moments(j0,j2,bq,mu=MU_GAIN):
    mu=np.asarray(mu,float)
    si=j0[:,None]+(.5*j2-.375*bq)[:,None]*(.5*(3*mu[None,:]**2-1))
    sq=(-.75*j2+.5625*bq)[:,None]*(1-mu[None,:]**2)
    return si,sq


def screen_modes(Q,mu,w):
    P=np.stack([lpmv(2,int(l),mu) for l in LC],axis=-1)
    c=(Q*w[None,:])@P/N_FACTOR[None,:]
    E=c*E_FACTOR[None,:]
    return c,E


def direct_3d_tensor(I,Q,mu,w,nphi=16):
    """Independent tensor integration in spherical screen basis, full sky by parity.

    I and Q are 1D even parity over mu>0; all off-diagonals vanish after phi
    average. nphi equally spaced azimuths; return M[3,3].
    """
    phi=np.arange(nphi)*2*np.pi/nphi
    mu=np.asarray(mu)
    cp,sp=np.cos(phi),np.sin(phi)
    st=np.sqrt(1-mu**2)
    n=np.stack(np.broadcast_arrays(st[:,None]*cp[None,:],st[:,None]*sp[None,:],mu[:,None]*np.ones((1,nphi))),axis=-1)
    th=np.stack(np.broadcast_arrays(mu[:,None]*cp[None,:],mu[:,None]*sp[None,:],-st[:,None]*np.ones((1,nphi))),axis=-1)
    ph=np.stack(np.broadcast_arrays(-sp[None,:]*np.ones((len(mu),1)),cp[None,:]*np.ones((len(mu),1)),np.zeros((len(mu),nphi))),axis=-1)
    P=np.eye(3)[None,None,:,:]-np.einsum('...i,...j->...ij',n,n)
    Qp=np.einsum('...i,...j->...ij',th,th)-np.einsum('...i,...j->...ij',ph,ph)
    mat=.5*I[:,None,None,None]*P+.5*Q[:,None,None,None]*Qp
    return (4*np.pi/nphi)*np.einsum('m,mfij->ij',w,mat)


def run(owner,pr138,out,internal_order=48,n64=64,n96=96,limit=50):
    assert sha(owner)==OWNER_SHA,'ORIGINAL_OWNER_RAW_SHA_MISMATCH'
    assert sha(pr138)==PR138_SHA,'PR138_SOURCE_RAW_SHA_MISMATCH'
    hist=load_owner_archive(owner);cases=sorted(hist)
    assert len(cases)==50 and 1<=limit<=50
    cases=cases[:limit];out=pathlib.Path(out);out.mkdir(parents=True,exist_ok=True)
    with np.load(ROOT/'source/pr138_high_moments.npz',allow_pickle=False) as z:
        assert np.array_equal(z['cases'][:limit],np.asarray(cases)),'PARENT_PR138_CASE_ORDER_MISMATCH'
        parent_c_high=z['spin2_c12_14_16'][:limit].copy()
    with np.load(ROOT/'source/pr134_spin2.npz',allow_pickle=False) as z:
        assert np.array_equal(z['cases'][:limit],np.asarray(cases)),'PARENT_PR134_CASE_ORDER_MISMATCH'
        parent_c_low=z['spin2_c_even'][:limit].copy()
    n=len(hist[cases[0]]['x']);assert n==193
    shp=(limit,n)
    data={k:np.zeros(shp) for k in ['Mperp','Mparallel','M_anisotropy','J0_total','J2_total','BQ_total','E2_candidate','j2_collision','BQ_collision','anisotropy_frac']}
    data['stokes_gain_I5']=np.zeros((limit,n,5));data['stokes_gain_Q5']=np.zeros((limit,n,5))
    data['c_l']=np.zeros((limit,n,len(LC)));data['E_l0_plus']=np.zeros((limit,n,len(LC)))
    data['B_l0_plus']=np.zeros((limit,n,len(LC)))
    meta=[];endpoint=[]
    mx={'tensor_crosswalk':0.,'trace':0.,'gain_I':0.,'gain_Q':0.,'GL64_96_tensor':0.,'GL64_96_c':0.,'GL64_96_E':0.,'pr138_c_high':0.,'pr134_c_low':0.,'explicit_3d_tensor':0.,'transverse_offdiag':0.,'polarization_fraction':0.,'fixed_operator_residual':0.}
    mu64,w64=glhalf(n64);mu96,w96=glhalf(n96)
    for i,k in enumerate(cases):
        h=hist[k]
        s=solve_case(h,nangular=internal_order,mu_out=np.r_[mu64,mu96])
        I,Q=s['fullI'][:,:n64],s['Q'][:,:n64]
        I96,Q96=s['fullI'][:,n64:],s['Q'][:,n64:]
        J0,J2,B=moments(I,Q,mu64,w64)
        J096,J296,B96=moments(I96,Q96,mu96,w96)
        p,zz=direct_tensor(I,Q,mu64,w64)
        p96,zz96=direct_tensor(I96,Q96,mu96,w96)
        pm,zzm=tensor_from_moments(J0,J2,B)
        si,sq=gains_from_tensor(p,zz)
        si2,sq2=gains_from_moments(J0,J2,B)
        c,E=screen_modes(Q,mu64,w64)
        c96,E96=screen_modes(Q96,mu96,w96)
        Cj2=-h['g']*(.9*J2+3*B/40)
        Cbq=-h['g']*(.8*J2+.4*B)
        if np.min(p)<-1e-12 or np.min(zz)<-1e-12:raise AssertionError('INTEGRATED_COHERENCY_NOT_POSITIVE '+k)
        if np.max(np.abs(Q)/I)>1+1e-10:raise AssertionError('PHOTON_STOKES_POSITIVITY '+k)
        if not np.allclose(s['x'],h['x'],rtol=0,atol=0):raise AssertionError('CLOCK_SOURCE_DRIFT')
        for key,val in [('Mperp',p),('Mparallel',zz),('M_anisotropy',zz-p),('J0_total',J0),('J2_total',J2),('BQ_total',B),('E2_candidate',12*c[:,0]/(5*np.sqrt(6))),('j2_collision',Cj2),('BQ_collision',Cbq),('anisotropy_frac',(zz-p)/(4*np.pi*J0))]:data[key][i]=val
        for key,val in [('stokes_gain_I5',si),('stokes_gain_Q5',sq),('c_l',c),('E_l0_plus',E)]:data[key][i]=val
        mx['tensor_crosswalk']=max(mx['tensor_crosswalk'],float(np.max(np.abs(p-pm))),float(np.max(np.abs(zz-zzm))))
        mx['trace']=max(mx['trace'],float(np.max(np.abs(2*p+zz-4*np.pi*J0))))
        mx['gain_I']=max(mx['gain_I'],float(np.max(np.abs(si-si2))))
        mx['gain_Q']=max(mx['gain_Q'],float(np.max(np.abs(sq-sq2))))
        mx['GL64_96_tensor']=max(mx['GL64_96_tensor'],float(np.max(np.abs(p-p96))),float(np.max(np.abs(zz-zz96))))
        mx['GL64_96_c']=max(mx['GL64_96_c'],float(np.max(np.abs(c-c96))))
        mx['GL64_96_E']=max(mx['GL64_96_E'],float(np.max(np.abs(E-E96))))
        mx['pr134_c_low']=max(mx['pr134_c_low'],float(np.max(np.abs(c[:,:5]-parent_c_low[i]))))
        mx['pr138_c_high']=max(mx['pr138_c_high'],float(np.max(np.abs(c[:,5:8]-parent_c_high[i]))))
        mx['polarization_fraction']=max(mx['polarization_fraction'],float(np.max(np.abs(Q)/I)))
        mx['fixed_operator_residual']=max(mx['fixed_operator_residual'],float(s['fixed_res']))
        if i in (0,25,49):
            for ix in (0,64,128,192):
                direct=direct_3d_tensor(I[ix],Q[ix],mu64,w64)
                expect=np.diag([p[ix],p[ix],zz[ix]])
                mx['explicit_3d_tensor']=max(mx['explicit_3d_tensor'],float(np.max(np.abs(direct-expect))))
                mx['transverse_offdiag']=max(mx['transverse_offdiag'],float(np.max(np.abs(direct-np.diag(np.diag(direct))))))
        meta.append({'case':k,'source_sha256':h['source_sha256'],'min_Mperp':float(np.min(p)),'min_Mparallel':float(np.min(zz)),'max_M_anisotropy':float(np.max(np.abs(zz-p))),'max_abs_Q_over_I':float(np.max(np.abs(Q)/I))})
        endpoint.append({'case':k,'source_sha256':h['source_sha256'],'Mperp_final':float(p[-1]),'Mparallel_final':float(zz[-1]),'anisotropy_final':float(zz[-1]-p[-1]),'E20plus_final':float(E[-1,0]),'E40plus_final':float(E[-1,1]),'E120plus_final':float(E[-1,5]),'J2_collision_final':float(Cj2[-1]),'BQ_collision_final':float(Cbq[-1])})
        if (i+1)%10==0:print('SOURCE_CASES',i+1,'/',limit,flush=True)
    crit={'tensor_crosswalk':1e-10,'trace':1e-10,'gain_I':1e-10,'gain_Q':1e-10,'GL64_96_tensor':1e-10,'GL64_96_c':1e-10,'GL64_96_E':1e-10,'pr138_c_high':1e-10,'pr134_c_low':1e-10,'explicit_3d_tensor':1e-10,'transverse_offdiag':1e-10}
    for key,thres in crit.items():
        if mx[key]>thres:raise AssertionError(('PRE_REGISTERED_DISCREPANCY',key,mx[key],thres))
    def write_csv(path,rows):
        with open(path,'w',newline='',encoding='utf-8') as f:
            wr=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n');wr.writeheader();wr.writerows(rows)
    write_csv(out/'COHERENCY_ENDPOINT_50.csv',endpoint)
    write_csv(out/'COHERENCY_AUDIT_50.csv',meta)
    pairs=[]
    if limit==50:
        index={k:i for i,k in enumerate(cases)}
        for m in range(5):
            for r in range(5):
                a=index[f's00_m{m:02d}_r{r:02d}'];b=index[f's04_m{m:02d}_r{r:02d}']
                pairs.append({'pair':f'm{m:02d}_r{r:02d}',
                'delta_E20_plus_final':float(data['E_l0_plus'][b,-1,0]-data['E_l0_plus'][a,-1,0]),
                'delta_E120_plus_final':float(data['E_l0_plus'][b,-1,5]-data['E_l0_plus'][a,-1,5]),
                'delta_anisotropy_final':float(data['M_anisotropy'][b,-1]-data['M_anisotropy'][a,-1]),
                'delta_Q_source_equator_final':float(data['stokes_gain_Q5'][b,-1,0]-data['stokes_gain_Q5'][a,-1,0])})
        write_csv(out/'COHERENCY_MATCHED_25.csv',pairs)
    np.savez_compressed(out/'BASS21_RANK9_EHARM_50x193.npz',cases=np.array(cases),x=hist[cases[0]]['x'],l=LC,mu_gain=MU_GAIN,**data)
    summary={'schema':'vigilode.bass21.rank9_screen_eharm.v1','original_owner_SHA256':OWNER_SHA,'parent_pr138_SHA256':PR138_SHA,
             'cases':limit,'epochs':n,'new_ivps':0,'new_coherency_scalar_values':int(limit*n*3),
             'new_E_l0_values':int(data['E_l0_plus'].size),'new_stokes_gain_samples':int(data['stokes_gain_I5'].size+data['stokes_gain_Q5'].size),
             'new_rank9_collisional_rate_values':int(data['j2_collision'].size+data['BQ_collision'].size),
             'max_abs_E20_plus':float(np.max(np.abs(data['E_l0_plus'][:,:,0]))),
             'max_abs_E120_plus':float(np.max(np.abs(data['E_l0_plus'][:,:,5]))),
             'max_E20_plus_at_final_mean':float(np.mean(data['E_l0_plus'][:,-1,0])),
             'mean_paired_delta_E20_plus_final':float(np.mean([p['delta_E20_plus_final'] for p in pairs])) if pairs else None,
             'max_abs_anisotropy_frac':float(np.max(np.abs(data['anisotropy_frac']))),
             'max_abs_tensor_anisotropy':float(np.max(np.abs(data['M_anisotropy']))),
             'max_abs_J2_collision':float(np.max(np.abs(data['j2_collision']))),
             'max_abs_BQ_collision':float(np.max(np.abs(data['BQ_collision']))),
             'max_angular_replay_and_tensor_discrepancy':mx,
             'source_case_SHA256':{k:hist[k]['source_sha256'] for k in cases},
             'physics':'fixed-screen axisymmetric E_l0 plus convention and rank-nine coherency Thomson gain on source-frozen 50x193 coupled Volterra deltaI/Q',
             'claim_ceiling':'not source-native owner dynamic E_l; not observed CMB E/B; not global BDF certificate; not Einstein/electron backreaction',
             'dag':{'vigilode':'7/23','rei_central':'28/88','rei_working':'33/88','bass_ce1':'9/15','successor_admitted':'0/12','official_delta':0},
             'production':'HOLD','distinct_author_R6_C9':'NOT_RUN'}
    (out/'SCIENCE_SUMMARY.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n')
    normalization={'l':LC.tolist(),'c_to_E_l0_plus':E_FACTOR.tolist(),'c_projection_norm':N_FACTOR.tolist(),
                   'E2_candidate_per_c2':float(12/(5*np.sqrt(6))),
                   'convention':'Q=(a2+a-2)*spin2Y with E_plus defined by E_plus_l0=integral dOmega Q * {}_2Y_l0; alternative common E=-E_plus requires sign flip',
                   'collision_J2_BQ_eigenvalues':['-1','-3/10'],
                   'tensor_gain':'(3/(8pi)) P(n) M P(n)',
                   'M_parallel':'4pi*((J0-J2)/3+BQ/4)',
                   'M_perp':'4pi*(J0/3+J2/6-BQ/8)'}
    (out/'NORMALIZATION.json').write_text(json.dumps(normalization,indent=2,sort_keys=True,allow_nan=False)+'\n')
    print('SUMMARY '+json.dumps({k:v for k,v in summary.items() if k not in ('source_case_SHA256',)},sort_keys=True),flush=True)
    return summary

if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('--owner',required=True);p.add_argument('--pr138',required=True);p.add_argument('--out',required=True)
    p.add_argument('--limit',type=int,default=50)
    args=p.parse_args();run(args.owner,args.pr138,args.out,limit=args.limit)
