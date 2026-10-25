Module[{f,j,pt,k,residuals,checks,timing,cpu0,sample},
 cpu0=TimeUsed[];
 timing=AbsoluteTiming[
  k=k0+eta*t^2;
  f={-lambda*x+k*y^2,lambda*x-mu*y-k*y^2};
  j=D[f,{{x,y}}]; pt=D[f,t];
  residuals=<|
   "jacobian"->Simplify[j-{{-lambda,2*k*y},{lambda,-mu-2*k*y}}],
   "time_derivative"->Simplify[pt-{2*eta*t*y^2,-2*eta*t*y^2}],
   "sum_loss"->Simplify[Total[f]+mu*y],
   "x_boundary"->Simplify[(f[[1]]/.x->0)-k*y^2],
   "y_boundary"->Simplify[(f[[2]]/.y->0)-lambda*x],
   "state_remainder"->Simplify[(f/.{x->x+dx,y->y+dy})-f-j.{dx,dy}-{k*dy^2,-k*dy^2}],
   "determinant_J"->Simplify[Det[j]-lambda*mu],
   "determinant_I_hJ"->Simplify[Det[IdentityMatrix[2]-h*j]-(1+h*(lambda+mu+2*k*y)+h^2*lambda*mu)],
   "represented_secant"->Simplify[((f/.t->tp)-(f/.t->tm))/(tp-tm)-{eta*(tp+tm)*y^2,-eta*(tp+tm)*y^2}],
   "symmetric_secant"->Simplify[((f/.t->t+eps)-(f/.t->t-eps))/(2*eps)-pt],
   "directional_Jv"->Simplify[j.{1,-1}-{-lambda-2*k*y,lambda+mu+2*k*y}],
   "wrong_J_plus_I"->Simplify[(j+IdentityMatrix[2]-j).{1,-1}-{1,-1}]
  |>;
  checks=Map[Function[z,And@@(TrueQ[#==0]& /@ Flatten[{z}])],residuals];
  sample={f,j,pt}/.{lambda->3,mu->1,k0->2,eta->1/2,t->1/2,x->1/4,y->3/4};
 ];
 ExportString[<|"version"->$Version,"wall_seconds"->timing[[1]],"cpu_seconds"->(TimeUsed[]-cpu0),"residual_groups"->Length[checks],"checks"->checks,"all_zero"->And@@Values[checks],"sample_exact"->ToString[sample,InputForm]|>,"RawJSON"]
]
