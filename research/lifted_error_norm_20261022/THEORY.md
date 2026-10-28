# Lift the error before controlling an affine-reduced atomic network

The exact BASS compact30 map reconstructs three neutral fractions from conserved H/He/Li totals. For an error vector, each reconstructed neutral error is minus the sum of its charged errors; the constant conserved total cancels. The existing33state norm must therefore be

    N(e)=sqrt((1/33)Σ_i[(L e)_i/s_i]^2),
    s_i=atol_i+rtol_i max(|v_old,i|,|v_new,i|)

when that is the caller's current scaling rule. Other declared owner rules must be passed unchanged; the helper never guesses scales. Reconstruct old/new full33 states first, compute all33positive scales, then call the kernel. Dynamic/vector tolerances apply also to eliminated neutral fractions. The original denominator33 is retained. The already-removed full62 zero gas-energy placeholder is not silently restored to this norm; full62 acceptance would require its separately defined weights and isotope duplications.

Equivalently, N²=eᵀ Lᵀ diag(s^-2) L e/33. The matrix generally has nonzero off-diagonal chemical entries. Taking threeLi charged errors equal to1 with all scales1 gives full norm sqrt(12/33)=0.6030226891555273, while the naive30norm is sqrt(3/30)=0.31622776601683794. Dropping the eliminated coordinate would underweight this direction by1.90692517849.

The C99interface accepts const double err30[30], const double scale33[33], and double*out. State ordering matches BASS's KEEP selection: first23geometry entries; HII at23; HeII/III at24/25; LiII/III/IV at26/27/28; tau29. Full33neutral slots are23,25,28. RHS/Jacobian transformation is separate; this helper only evaluates the lifted error norm. No solver core/default or live NCPsource is modified.

A first finite implementation normalized each chemical error by its largest magnitude before compensation. Fresh root review supplied a concrete counterexample: Li errors[1e308,-1e308,1e-300], neutral scale1e-300 and charged scales1e308. Premature scaling lost the third error, producing0.24618 instead of0.301511. The failedsource, oldverification and reproduced counterexample are preserved. The repaired kernel accumulates the at-most3terms with compensated longdouble arithmetic, then divides and takes hypotl in extended range. This also permits an individual ratio above DBL_MAX when division bysqrt33 yields a finite RMS. It casts only the final norm to double.

The API is binary64 at its boundary but uses extended internal arithmetic. Compile-time exponent/mantissa conditions are checked. Platforms with binary64-only longdouble return status3 without changing output. Status1 indicates invalid inputs,2 an unrepresentable final norm;0success. No fast-math/FTZ assumptions are added; compile with ordinary IEEE semantics. This is a tested numerical kernel, not a correctly-rounded proof for every possible input. Caller arrays must have their declared lengths and remain valid throughout the call.

One bounded verification pass and root's one concrete repair episode used a1200digit independent Decimal oracle. The first100digit oracle itself rounded a309digit binary64 input before cancellation and failed JSON serialization; that earlier error is also retained. Final10finite cases and5negative controls pass the unchanged32ulp criterion. The corrected build/test takes0.506s; no native VigilODE trajectory, NCPcampaign or speed benchmark ran. The unsupported-longdouble status is implemented but not executed on a second platform here.

For native integration, use an explicit C FFI wrapper with fixed-size arrays and status checks, evaluated in the accepted error-control path. A rejected norm must not commit a step. Preserve same full33scale convention in local-error, embedded and any dense-output admission affected by the reduction. No claim is made that the current native driver already invokes this helper; that wiring plus one matched-owner trajectory is the remaining implementation action.
