# Exact row sums and scaled outward physical-error norms

## Contract and interpretation
Let e in R^30 contain the exact real values represented by finite binary64 inputs.
Let s in R^33 contain supplied finite positive physical scales. The fixed PR79 lift
has unit-copy rows and dependent rows -(e23), -(e24+e25), -(e26+e27+e28).
The target is E^2 = (1/33) sum_i ((Le)_i/s_i)^2. The physical units cancel in each
ratio. This is an arithmetic target for an embedded difference, not a bound on
actual truncation, global trajectory error, scale-construction error, or a nonlinear
coordinate map. The affine state offset does not enter the difference.

For a two-species dependent row, the Gram block is
[[s0^-2+s1^-2,s0^-2],[s0^-2,s0^-2+s2^-2]]. It is positive definite for positive finite
scales, not diagonal. Sparse application avoids constructing this Gram matrix.
Wolfram returned identity0 and E_same^2=25/22, E_opposite^2=25/66, E_overflow^2=2/11.

## Lemma 1: exact fixed storage for each dependent sum
Every finite binary64 is sign*M*2^-1074 where M is a nonnegative integer.
For subnormal inputs M=fraction; for normal exponent bits b (1<=b<=2046),
M=(2^52+fraction)*2^(b-1). Thus M_max=(2^53-1)*2^2045 <2^2098.
A sum of at most three absolute inputs is <3*2^2098 <2^2100.
Two unsigned 33-word arrays (2112 bits each) can accumulate positive and negative
terms independently, and exact integer subtraction produces |sum|. The bounds
include cancellation across the full exponent range, minsubnormal and three maxima.
No floating-point addition occurs in this row accumulation. The fixed3-term scope
is essential; extending to arbitrary row width or non-unit products requires a new
capacity/product proof.

## Lemma 2: bounded mantissa interval without overflowing raw sums
For nonzero integer magnitude A with highest bit H, let k=H-1074. Keep its leading
53 bits and inspect the discarded bits. This constructs exact binary64 endpoints
m_lo,m_hi with 1<=m_lo<=m_hi<=2 and
m_lo*2^k <= A*2^-1074 <= m_hi*2^k.
The endpoints coincide if no bits were discarded; otherwise width is2^-52.
For H<52 shift left exactly. The scale s has an exact mantissa/exponent representation
s=m_s*2^k_s, m_s in[1,2). This is extracted from bits, including subnormals.
Consequently outward division gives a ratio interval with exponent k-k_s, without
forming either A*2^-1074 or its unscaled ratio as binary64.

## Lemma 3: outward power-of-two scaling
For finite nonnegative x, decode the exact significand and exponent. A power-of-two
rescale that remains normal is exact. If it becomes subnormal, shift the integer
significand right: discarded bits give a floor and a ceiling differing by at most
one subnormal ULP. Values below minsubnormal have [0,minsubnormal], not [0,0].
Overflow has [MAX,+infinity], though global ratio scaling uses only nonpositive
exponent differences and never needs this branch. No pow or logarithm is used.

## Theorem: squared-norm enclosure and safe tri-state decision
Let each exact ratio magnitude r_i lie in [a_i,b_i]*2^k_i. Choose K=max k_i over
nonzero rows. Outwardly scale all intervals by2^(k_i-K), square, sum, and divide by33.
All operands after scaling are bounded: ratio mantissas are approximately in[1/2,2],
so the sum is below approximately132. One-step adjacent floats enclose each
correctly rounded positive operation, including underflow. Exact zero rows are
skipped. Thus the returned [lo,hi]*2^(2K) encloses E^2 by monotonicity and induction.
Compare these scaled endpoints to1 by decoded exponents, never by overflowing pow2.
- upper<=1 implies E<=1: Accept.
- lower>1 implies E>1: Reject.
- otherwise Indeterminate: no accepted-state commit is authorized.

This is a conditional arithmetic proof, not a mechanically checked proof of Rust.
It assumes IEEE binary64 correctly rounded +,*,/, gradual underflow, and no fast-math
or FTZ/DAZ. Integer indexing/carry/borrow and bit conversion must pass code review and
finite exact-oracle tests. A compiler/host contract cannot be proved by test samples.
Near-threshold Indeterminate is intentional; this does not promise exact native
accept/reject parity for every real E. It also avoids falsely accepting exact boundary
values that cannot be resolved by the interval width.

## New contribution and reused knowledge
Exact superaccumulation is established (Neal2015, arXiv1505.05571); no novelty claim
for that technique. The new scoped construction combines a small fixed3-term exact
sum, PR79 physical scaling, exponent-separated squared bounds, and a typed
Indeterminate result suitable for later native error admission. The original C99
kernel is a point-valued comparison, not an interval oracle. The current general
AffineErrorNorm's documented overflow refusal is a narrower support contract,
not automatically an implementation bug. No production hook is modified here.
