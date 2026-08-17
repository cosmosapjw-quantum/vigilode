# PHYS–MATH Preflight — Early Flow Defect

## Authority equation

For the canonical first `pexprb54s4` stage, `c2=1/4`,

\[
U_2 = y_n + h\,c_2\varphi_1(c_2 hJ_n)F_n,
\qquad
D_2 = F(U_2)-F_n-J_n(U_2-y_n).
\]

The read-only diagnostic on a declared component prefix of length `m` is

\[
\eta_{c_2}=\frac{|h|\,\|D_2^{(m)}\|_2}{\|U_2^{(m)}-y_n^{(m)}\|_2}.
\]

It is dimensionless under a common state scaling but not invariant under arbitrary
component-wise rescaling. Therefore no universal threshold is selected in this node.

## Nonautonomous systems

The fused authority path remains autonomous. Existing time augmentation maps
`(y,t)'=(F(t,y),1)` and its JVP contains `J_y v_y + F_t v_t`. Thus `D_2` already contains the
first-order time correction. The trailing clock component is excluded from the norm by setting
`m` to the original physical dimension. No independent `F_t` norm is claimed.

## Limits

- affine/linear autonomous problem: `D2=0`, hence `eta=0` when the stage increment is nonzero;
- zero increment and zero remainder: defined as `eta=0`;
- zero increment and nonzero remainder: no finite normalized scalar exists, so `eta=null`;
- `h -> 0`: the scalar is expected to shrink for smooth nonlinear flow, but this node does not
  promote that heuristic to an error theorem.

## Claim ceiling

The diagnostic can be evaluated for predictive separation against actual adaptive outcomes. It
cannot by itself certify fifth-order accuracy, local error, safe admission, or profitable switching.
