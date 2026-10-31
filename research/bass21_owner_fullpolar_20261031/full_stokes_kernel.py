"""Covariant-screen, bolometric cold-electron Thomson collision for full (I,Q,U,V).

Physical field is a 2x2 Hermitian coherency K(n) in normal meridional/azimuthal
screen. General incoming Stokes allowed. Spatial 3x3 angular coherency M'_ij
is integrated in electron-rest tetrad with D^2 weight after null-gauge fixing.
Electron normal density n_e^normal fixed; proper n'_e=n_e^normal/gamma.
No KN recoil, frequency redistribution, electron thermal spread or new IVP.
"""
import numpy as np
import fullpolar as base

def kernels(beta,n,et,ep):
 D,nr,a,b=base.lorentz_data(beta,n,et,ep)
 aa=np.einsum('...i,...j->...ij',a,a)
 bb=np.einsum('...i,...j->...ij',b,b)
 ab=np.einsum('...i,...j->...ij',a,b)
 ba=np.einsum('...i,...j->...ij',b,a)
 pref=2*np.pi/n.shape[1]/2
 def angular(x):return pref*np.einsum('mp,mpij->mij',D**2,x,optimize=True)
 return dict(I=angular(aa+bb),Q=angular(aa-bb),U=angular(ab+ba),V=1j*angular(ba-ab))

def moments(stokes,w,angular_kernels):
 n=len(w);x=np.asarray(stokes)
 if x.ndim!=3 or x.shape[1]!=n or x.shape[2]!=4:
  raise ValueError('expect [states,mu,4] real Stokes')
 if np.any(~np.isfinite(x)):raise ValueError('nonfinite stokes')
 if np.any(x[...,0]+1e-12<np.linalg.norm(x[...,1:],axis=-1)):
  raise ValueError('unphysical polarization degree')
 return sum(np.einsum('sm,m,mij->sij',x[:,:,idx],w,angular_kernels[k],optimize=True)
            for idx,k in enumerate(['I','Q','U','V']))

def coherency(I,Q,U,V):
 r=np.empty((len(np.atleast_1d(I)),2,2),dtype=complex)
 r[:,0,0]=(I+Q)/2;r[:,1,1]=(I-Q)/2
 r[:,0,1]=(U-1j*V)/2;r[:,1,0]=(U+1j*V)/2
 return r

def collision(M, normal_direction, beta, stokes_out, gnormal):
 out=base.outgoing_coeffs(normal_direction,beta)
 D,E,_=out
 x=np.asarray(stokes_out,dtype=float)
 if x.ndim!=2 or x.shape[1]!=4:raise ValueError('Stokes out must shape (states,4)')
 G=3/(8*np.pi)*np.einsum('ai,sij,bj->sab',E,M,E,optimize=True)
 K=coherency(x[:,0],x[:,1],x[:,2],x[:,3])
 gamma=1/np.sqrt(1-np.dot(beta,beta))
 gg=np.asarray(gnormal)/gamma
 C=gg[:,None,None]*(G/D**3-D*K)
 st=np.stack((np.real(C[:,0,0]+C[:,1,1]),
              np.real(C[:,0,0]-C[:,1,1]),
              np.real(C[:,0,1]+C[:,1,0]),
              np.real(1j*(C[:,0,1]-C[:,1,0]))),axis=1)
 if np.max(np.abs(st.imag))>1e-13:raise AssertionError('complex Stokes')
 return st,C,G,K

if __name__=='__main__':
 mu,w,n,et,ep=base.angular_grid(32,64)
 st=np.zeros((1,len(mu),4));st[:,:,0]=1;st[:,:,3]=.2*mu[None,:]
 b=(.02,0.,.03)
 M=moments(st,w,kernels(b,n,et,ep))
 s=np.array([[1,0,0,.04]])
 print(collision(M,(.6,.8,0),b,s,np.array([1.]))[0])
