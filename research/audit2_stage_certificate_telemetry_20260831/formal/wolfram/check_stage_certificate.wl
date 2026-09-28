(* Candidate-free exact-rational cross-checks for FORMAL_SCOPE.md F02 and F03. *)
ClearAll[assertToken];
assertToken[token_, proposition_] := If[TrueQ[FullSimplify[proposition]],
  Print[token], Print["FAILED: " <> token]; Exit[1]];

(* Declared two-timescale triangular synthetic operator and exact Jacobi map. *)
w = {{2, 1}, {0, 3}};
d = {{2, 0}, {0, 3}};
leftDeviation = Inverse[d].w - IdentityMatrix[2];
rightDeviation = w.Inverse[d] - IdentityMatrix[2];
assertToken["F02_WOLFRAM_JACOBI_SQUARE_ZERO",
  leftDeviation.leftDeviation == ConstantArray[0, {2, 2}] &&
  rightDeviation.rightDeviation == ConstantArray[0, {2, 2}]];

(* Exact approximate-solve fixture: b - W xhat is the residual. *)
xhat = {1, 1};
b = {3, 4};
residual = b - w.xhat;
kappa = 1;
xExact = xhat + Inverse[w].residual;
assertToken["F03_WOLFRAM_APPROXIMATE_SOLVE_BOUND",
  Norm[xExact, 2] <= Norm[xhat, 2] + kappa Norm[residual, 2]];

Print["WOLFRAM_FORMAL_PASS"];
