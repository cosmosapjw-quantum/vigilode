ClearAll[ll,l21,l31,l32,l41,l42,l43,lam,q,b,x0,x1,d,k,h,g];
ll={{0,0,0,0},{l21,0,0,0},{l31,l32,0,0},{l41,l42,l43,0}};
invp=IdentityMatrix[4]+lam ll+lam^2 ll.ll+lam^3 ll.ll.ll;
finiteIdentity=Simplify[(IdentityMatrix[4]-lam ll).invp-IdentityMatrix[4]];
g0=q*x0+b; g1=q*x1+b; r0=g0-x0; r1=g1-x1;
acc=g1-r1/(r1-r0)*(g1-g0);
aaIdentity=FullSimplify[acc-b/(1-q),Assumptions->{q!=1,x0!=x1}];
pp=ConstantArray[1/3,{3,3}]; qq=IdentityMatrix[3]-pp;
jj=-d IdentityMatrix[3]+k pp;
resolvent=pp/(1+h*g*(d-k))+qq/(1+h*g*d);
quotientIdentity=FullSimplify[(IdentityMatrix[3]-h*g*jj).resolvent-IdentityMatrix[3]];
<|"finite_resolvent_zero"->(finiteIdentity===ConstantArray[0,{4,4}]),
  "nilpotent_four"->(ll.ll.ll.ll===ConstantArray[0,{4,4}]),
  "scalar_AA_exact_root_zero"->aaIdentity,
  "mean_field_resolvent_zero"->(quotientIdentity===ConstantArray[0,{3,3}])|>
