(* Exact supporting algebra checks; not a proof of native floating arithmetic. *)
Module[{a,b,z,s,t,h,lambda,e,delta,zz,u1,u2,mat,mom,expected,
        q0,q1,q2,q3,d0,d1,d2,d3,qp,dp,poly,power,beta,mid,checks},
 mat={{0,0,0,b-a^2},{1,0,0,-1},{0,1,0,2a},{0,0,1,0}};
 mom=Table[Expand[Tr[MatrixPower[mat,k]]],{k,0,6}];
 expected={4,0,4a,-3,4a^2+4b,-10a,4a^3+12a b+3};
 qp=Sum[{q0,q1,q2,q3}[[j+1]] Binomial[3,j]s^j(1-s)^(3-j),{j,0,3}];
 dp=Sum[{d0,d1,d2,d3}[[j+1]] Binomial[3,j]s^j(1-s)^(3-j),{j,0,3}];
 poly=Expand[dp+qp^2/4];
 power=Table[Coefficient[poly,s,k],{k,0,6}];
 beta=Table[Sum[Binomial[j,k]/Binomial[6,k]power[[k+1]],{k,0,j}],{j,0,6}];
 mid={(d0+3d1+d2)/5+(2q0 q2+3q1^2)/20,
      (d0+9d1+9d2+d3)/20+(q0 q3+9q1 q2)/40,
      (d1+3d2+d3)/5+(2q1 q3+3q2^2)/20};
 zz=lambda h; u1=(e+h delta)/(1+zz);
 u2=((1-zz/2)e+h delta)/(1+zz/2);
 checks=<|
  "seven_companion_moments"->And@@Thread[Expand[mom-expected]==ConstantArray[0,7]],
  "triple_factorization"->(Expand[(z-1/2)^3(z+3/2)-(z^4-3z^2/2+z-3/16)]===0),
  "complex_pair_factorization"->(Expand[(z+1/2)^2(z^2-z+5/4)-(z^4+z^2/2+z+5/16)]===0),
  "trace_mixed_invariant"->(Expand[a^2 mom[[3]]-2a mom[[5]]+mom[[7]]-(4a b+3)]===0),
  "three_direct_bernstein_coefficients"->And@@Thread[Expand[beta[[3;;5]]-mid]=={0,0,0}],
  "sharp_slow_factor"->(Expand[3/4-3s(1-s)-3(s-1/2)^2]===0),
  "interior_weight_identity"->(Expand[Sum[Binomial[6,j]s^j(1-s)^(6-j),{j,2,4}]-(15(s(1-s))^2-10(s(1-s))^3)]===0),
  "sharp_algebraic_factor"->((15t^2-10t^3/.t->1/4)===25/32),
  "rational_radius_side"->(Together[u1-u2-h zz(lambda e-delta)/(2(1+zz)(1+zz/2))]===0),
  "shared_field_identity"->(Expand[(-b+a/16+2(4a^3+16a^2+12a b+16b+3)/512)-2(-7a/8+(4a^3+16a^2+12a b+16b+3)/512)-(29a/16-b)]===0)
 |>;
 <|"schema"->"vigilode.rvj.exact-symbolic.v1","checks"->checks,
   "check_count"->Length[checks],"all_pass"->And@@Values[checks],
   "scope"->"Exact symbolic supporting checks only; native enclosure and physical trajectory require separate original-equation validation"|>
]
