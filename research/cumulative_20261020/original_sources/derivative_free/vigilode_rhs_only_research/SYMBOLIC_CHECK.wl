(* Executed using the connected Wolfram evaluator on 2026-10-20.
   Exact source/model-specific checks; no native solver performance claim. *)
Clear[u,x,q,x0,a,k,h,z,v];
f={-k u (1-x),k u (1-x)};
balance=x-x0-a(q-x)(1-x);
bb=1+a(q+1);
disc=Factor[bb^2-4a(x0+a q)];
stableRoot=2(x0+a q)/(bb+Sqrt[disc]);
ident=Expand[(z-v)((-2z-z^2)-(-2v-v^2))+(2+z+v)(z-v)^2];
<|"conservation"->Expand[Total[f]],
  "elimination_polynomial"->Expand[-balance],
  "discriminant"->disc,
  "physical_root_example"->N[stableRoot/.{q->4/5,x0->1/5,a->1000},25],
  "two_point_identity"->ident,
  "frozen_factor_local_error"->Series[Exp[h]-1/(1-h),{h,0,4}],
  "endpoint_only_misses_interior_defect"->Integrate[(1-2z),{z,0,1}]|>
