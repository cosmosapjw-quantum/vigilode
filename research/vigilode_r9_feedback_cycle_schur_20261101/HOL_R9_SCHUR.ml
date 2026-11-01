(* R9 Schur complement: finite exact scalar block-elimination sublemmas. *)
let R9_SCHUR_DETERMINANT_IDENTITY = prove
 (`!a b c d:real. ~(a = &0)
   ==> d-c*b/a = (a*d-b*c)/a`,
  CONV_TAC REAL_FIELD);;
let R9_SCHUR_FIRST_EQUATION = prove
 (`!a b c d r s:real.
    ~(a = &0) /\ ~(a*d-b*c = &0)
    ==> a*((r-b*((a*s-c*r)/(a*d-b*c)))/a) +
        b*((a*s-c*r)/(a*d-b*c)) = r`,
  CONV_TAC REAL_FIELD);;
let R9_SCHUR_SECOND_EQUATION = prove
 (`!a b c d r s:real.
    ~(a = &0) /\ ~(a*d-b*c = &0)
    ==> c*((r-b*((a*s-c*r)/(a*d-b*c)))/a) +
        d*((a*s-c*r)/(a*d-b*c)) = s`,
  CONV_TAC REAL_FIELD);;
let R9_NILPOTENT_TWO_EDGE_PATH = prove
 (`!t u v w:real.
    (u + t*v + t pow 2*w) - t*(v+t*w) = u`,
  CONV_TAC REAL_RING);;
print_endline "HOL_VIGILODE_R9_SCHUR_KERNEL_4_THEOREMS";;
R9_SCHUR_DETERMINANT_IDENTITY;;
R9_SCHUR_FIRST_EQUATION;;
R9_SCHUR_SECOND_EQUATION;;
R9_NILPOTENT_TWO_EDGE_PATH;;
