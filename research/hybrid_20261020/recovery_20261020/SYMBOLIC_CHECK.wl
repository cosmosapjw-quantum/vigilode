Clear[q1,q2,q3,d,la,al,ep];
q={q1,q2,q3}; n=3; m=Total[q]/n;
lap={{-1,1,0},{1,-2,1},{0,1,-1}};
f=d lap.q-la q+al m (1-q)-ep q^2;
variance=Total[(q-m)^2]/n;
j=D[f,{q}];
local=d lap-DiagonalMatrix[Table[la+al m+2 ep q[[i]],{i,n}]];
border=Outer[Times,al (1-q),ConstantArray[1/n,n]];
<|"mean_identity_residual"->Expand[Total[f]/n-((al-la)m-(al+ep)m^2-ep variance)],
"zero_epsilon_transverse_identity"->(Expand[(f-Total[f]/n)-(d lap.(q-m)-(la+al m)(q-m))]/.ep->0),
"jacobian_band_rank_one_identity"->Simplify[j-local-border],
"row_sum_minus_alpha_minus_lambda"->Expand[j.ConstantArray[1,n]-(al-la)ConstantArray[1,n]]|>
