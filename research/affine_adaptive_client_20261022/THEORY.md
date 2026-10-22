# Affine BDF control

Let the owner state satisfy v=d+Lu, R L=I, and the three elemental conservation rows C satisfy C F(v)=0. The affine reduced equation is u'=R F(d+Lu), with Jacobian J30=R J33 L. The eliminated neutral states are reconstructed from initial conserved totals. No clipping, renormalization, rate or geometry changes occur.

For a BDF residual G33(v)=v-a-hβF(v), affine initial and historical states imply a=d+L a30. Thus G33(d+Lu)=L G30(u), and JG30=I-hβR J33 L. Conservation gives C J33 L=0, hence JG33 L=L JG30. Restriction and lift therefore commute with the implicit solve on the tangent manifold, up to finite arithmetic and stopping tolerances.

History differences of order >=1 cancel d. Newton increments and D-based LTE/order estimators are tangents: they must use L e30, never d+L e30. Predictor and endpoint scales are states and require the affine lift d+Lu. The metric is sqrt(sum_i((L e30)_i/s33_i)^2/33), preserving the original full33 denominator and reconstructed neutral contributions. The unchanged C99 norm provides status-checked widened accumulation and refuses unsupported arithmetic.

The pinned SciPy owner uses predictor s33=atol33+rtol|v_predict| for Newton and endpoint s33=atol33+rtol|v_new| for LTE and order estimates. This client uses precisely those rules, not a max(old,new) surrogate. Its explicit common initial step comes from the full33 owner selector, so no native30 initial-step norm participates. Dense interpolation is sampled at the common nine points and has no separate admission rule.

Finite precision neutral subtraction can lose low bits; it is measured by the paired full33 and 21-species discrepancies, not corrected. Physical boundaries remain inherited gate decisions. The small-window pair does not establish a full-interval equivalence theorem, physical model validity or performance improvement. The existing Physical HOLD is unchanged.
