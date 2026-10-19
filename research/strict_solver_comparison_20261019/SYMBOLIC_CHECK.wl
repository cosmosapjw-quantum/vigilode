Clear[a,b,c,t];
f={-a/25+10000 b c,a/25-10000 b c-30000000 b^2,30000000 b^2};
j=D[f,{{a,b,c}}];
mat={{-1,0},{1000,-1000}};
y=FullSimplify[MatrixExp[t mat].{1,0}];
w=IdentityMatrix[2]-mat/100;
{j,Simplify[Total[f]],Simplify[{1,1,1}.j],y,Simplify[D[y,t]-mat.y],y/.t->0,w,LinearSolve[w,{1,0}]}
(* Independently evaluated using the connected Wolfram kernel in this session:
   Total[f]=0; {1,1,1}.j={0,0,0}; exact ODE residual={0,0}.
   y={Exp[-t],(1000/999)(Exp[-t]-Exp[-1000t])}; y(0)={1,0}.
   w={{101/100,0},{-10,11}}; solve(w,{1,0})={100/101,1000/1111}.
   Abs[w[[2,1]]]>Abs[w[[1,1]]] is True: partial pivoting swaps rows.
   A Symbol::undefined2 warning was emitted for the uppercase symbol A in the
   original evaluator call, but the displayed residuals and solutions evaluated.
   This file uses mat to avoid that symbol warning. No interval certificate claimed.
*)
