(* R8 source-owned 2x2 exact linear algebra; LCF HOL Light kernel. *)
let R8_W_2X2_LEFT = prove
 (`!d1 d2 a b r1 r2:real.
     ~(d1*d2-a*b = &0)
     ==> d1*((d2*r1+a*r2)/(d1*d2-a*b)) -
         a*((b*r1+d1*r2)/(d1*d2-a*b)) = r1`,
  CONV_TAC REAL_FIELD);;
let R8_W_2X2_RIGHT = prove
 (`!d1 d2 a b r1 r2:real.
     ~(d1*d2-a*b = &0)
     ==> d2*((b*r1+d1*r2)/(d1*d2-a*b)) -
         b*((d2*r1+a*r2)/(d1*d2-a*b)) = r2`,
  CONV_TAC REAL_FIELD);;
let R8_SCALAR_NEUMANN_REMAINDER = prove
 (`!t:real. (&1-t)*(&1+t+t pow 2+t pow 3) = &1-t pow 4`,
  CONV_TAC REAL_RING);;
print_endline "HOL_VIGILODE_R8_KERNEL_PASS_3";;
R8_W_2X2_LEFT;;
R8_W_2X2_RIGHT;;
R8_SCALAR_NEUMANN_REMAINDER;;
