# Sharp structural residual transport, independent of fitted thresholds

Let r_t=d and r_c=d−κ. For either rate r define the scalar lower-triangular
operator M(r) with diagonal 1+hγr and strict lower entries
hγr a_ij−γc_ij. The original raw-U residual of a candidate is exactly
M(r) applied to its stage error in that channel. Thus the output error is
b^T M(r)^{-1} R in that channel. Compute w(r)=M(r)^{-T}b by the reverse
scalar recurrence; no state matrix or nonlinear derivative oracle is needed.

For n components let P=11^T/n and Q=I−P. The block mapping residual R_i
to the output error is w_t,i Q+w_c,i P. Each such block has diagonal
w_t,i+(w_c,i−w_t,i)/n and off-diagonal (w_c,i−w_t,i)/n. Therefore the exact
induced stacked-infinity to output-infinity norm is

    A_n = sum_i [ |w_t,i+(w_c,i−w_t,i)/n|
                 +(n−1)|(w_c,i−w_t,i)/n| ].

This is sharp: choose one output row and give each stage residual component
the sign of the corresponding entry of that row. Its output attains A_n
when all residuals have maximum magnitude one. Dense test oracles independently
assemble the complete original target, recover that row, and realize this
sign-aligned extremizer.

The affine discrete one-step map has linear part
R_t Q+R_c P, where R(r)=1−r sum_i b_i t_i(r). Its exact infinity norm is

    G_n = |R_t+(R_c−R_t)/n| +(n−1)|(R_c−R_t)/n|.

For an initially identical trajectory, a uniform residual cap delta gives

    ||e_N||_infinity <= A_n delta sum_{j=0}^{N−1} G_n^j.

This follows by induction from e_{j+1}=S e_j+algebraic_output_error_j.
The bound holds without imposing G_n<1. The finite sum is evaluated by
s_0=0, s_{j+1}=1+G_n s_j, including G_n=1 without a singular denominator.
Hence a specified algebraic budget E>0 yields the derived cap

    delta = E / (A_n sum_{j=0}^{N−1} G_n^j).

There is no tuned safety factor. E is a scientific accuracy specification;
the theorem determines how that specification constrains algebraic residuals.
The bound is about the exact affine discrete target, not its time-truncation
error. If an evaluated residual differs from the true original-target
residual by e_R and output rounding contributes e_out, then the per-step
term becomes A_n(delta+e_R)+e_out. The present structural API computes
ordinary binary64 constants and operational residuals, so the code does not
label those values outward interval certificates. The exact-rational audit
checks formulas and coefficient semantics, not every floating execution.

## Exact accumulated constant used in the implementation

The shared physical projectors give a tighter exact finite-horizon constant.
Writing S=R_t Q+R_c P, the block mapping R_{N−1−j,i} to the final output is
S^j T_i=R_t^j w_t,i Q+R_c^j w_c,i P. Therefore

    C_N = sum_{j=0}^{N−1} sum_i
          [ |R_t^j w_t,i+(R_c^j w_c,i−R_t^j w_t,i)/n|
           +(n−1)|(R_c^j w_c,i−R_t^j w_t,i)/n| ].

It is the sharp induced norm from the concatenated residuals of every step
and stage to the final output. Choose their signs independently to align one
output row to attain equality. The partial sums C_m are nondecreasing, so
C_N also controls every earlier grid output. This does not require G<1 or
nonnegative channel stability values. Scalar powers evaluated by recurrence
cost O(Ns), once per fixed-h error budget. The implemented residual limit is
E/C_N, and A sum G^j is retained as a simpler upper-bound comparison only.
