(* R10 path-forest affine scan, LCF kernel sublemmas; not full n-dimensional ROW proof. *)
let R10_AFFINE_COMPOSE_ASSOC = prove
 (`!a b c d e f:real.
    (e*c)*a = e*(c*a) /\
    (f+e*d)+(e*c)*b = f+e*(d+c*b)`,
  CONV_TAC REAL_RING);;
let R10_PATH_THREE_NODES = prove
 (`!b0 b1 b2 t1 t2:real.
    b2+t2*(b1+t1*b0) = b2+t2*b1+(t2*t1)*b0`,
  CONV_TAC REAL_RING);;
let R10_PATH_FOUR_NODES = prove
 (`!b0 b1 b2 b3 t1 t2 t3:real.
    b3+t3*(b2+t2*(b1+t1*b0)) =
    b3+t3*b2+(t3*t2)*b1+(t3*t2*t1)*b0`,
  CONV_TAC REAL_RING);;
print_endline "HOL_VIGILODE_R10_SCAN_KERNEL_PASS_3";;
R10_AFFINE_COMPOSE_ASSOC;;
R10_PATH_THREE_NODES;;
R10_PATH_FOUR_NODES;;
