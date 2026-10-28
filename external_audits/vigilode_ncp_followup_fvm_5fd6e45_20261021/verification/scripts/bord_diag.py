import numpy as np, sys
n=int(sys.argv[1]); m=2
b0=np.full(n,0.125); b1=np.array([0.0625 if i%2==0 else -0.0625 for i in range(n)])
c0=b0/n; c1=b1/n
exact=np.array([((i%17)-8.0)*0.125 for i in range(n+m)])
# where is the probe's error? rerun replicated algorithm and print per-block error
import importlib.util
spec=importlib.util.spec_from_file_location('br','bordered_repro.py')
