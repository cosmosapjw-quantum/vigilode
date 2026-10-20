"""Compare iteration splittings, NOT native float64 K/U byte equivalence.
K tableau is independently reconstructed by triangular solve from raw decimals.
"""
import numpy as np
from scipy.linalg import solve_triangular
from probe import A,C,G,BOUT,save

def main():
    S=solve_triangular(np.eye(8)/G-C,np.eye(8),lower=True)
    alpha=np.tril(A@S,-1);L=np.tril(alpha+S,-1);b=S.T@BOUT
    def curve(T,v,weights):
        ref=np.linalg.solve(np.eye(8)-T,v);x=np.zeros(8);rows=[]
        for k in range(9):
            rows.append({'round':k,'stage_inf_error':float(np.linalg.norm(x-ref,np.inf)),
                         'output_abs_error':float(abs(weights@(x-ref)))})
            x=T@x+v
        return rows,float(weights@ref)
    rows=[]
    for z in [-.01,-1.,-100.,-1e6]:
        D=1-G*z;TK=z*L/D;TU=(G*C+G*z*A)/D
        ur,uy=curve(TU,np.full(8,G*z/D),BOUT);kr,ky=curve(TK,np.full(8,z/D),b)
        rows.append({'z':z,'T_raw_norm_inf':float(np.linalg.norm(TU,np.inf)),
                     'T_K_norm_inf':float(np.linalg.norm(TK,np.inf)),
                     'raw_error_per_round':ur,'K_error_per_round':kr,
                     'coordinate_target_output_gap_float64':abs(uy-ky)})
    assert rows[0]['T_K_norm_inf']<.1 and rows[0]['T_raw_norm_inf']>60
    save('coordinate_comparison.json',{'transformation':'independent triangular float64 S, not native faer bits',
        'S_identity_defect':float(np.linalg.norm((np.eye(8)-G*C)@S-G*np.eye(8),np.inf)),
        'norm_warning':'stage norms are chart-dependent; output errors are also reported','cases':rows})
if __name__=='__main__':main()
