Peter N. Brown, Alan C. Hindmarsh, Linda R. Petzold,
Consistent Initial Condition Calculation for Differential-Algebraic Systems,
SIAM J. Sci. Comput. 19(5),1495-1512 (1998), DOI 10.1137/S1064827595289996.
Exa search returned primary publisher and original author copy:
https://epubs.siam.org/doi/10.1137/S1064827595289996
https://cse.cs.ucsb.edu/sites/default/files/publications/SCE001495.pdf
SUNDIALS IDA Mathematical Considerations 5.2.1, fixed differential initial
variables, Newton/line search, explicit index-one admission:
https://sundials.readthedocs.io/en/latest/ida/Mathematics_link.html
SciSpace returned Scott & Barton (2013), Interval bounds on the solutions
of semi-explicit index-one DAEs, Parts 1 and 2. Parallel independently
returned the MIT repository record of the computation paper:
https://dspace.mit.edu/entities/publication/be35f84a-5bda-40af-ba3b-5c539e3ed46c
Their global interval propagation is not claimed here. The fixed-point
and inverse-defect arguments in this node are direct derivations.
Wolfram independent evaluation:
FullSimplify[D[z[t]-x[t]^2-t,t]/.{x'[t]->-x[t],z'[t]->1-2 x[t]^2}]
=0; derivative of Sqrt[x+t] =1/(2 Sqrt[t+x]); 2x2 adjugate residual zero.
