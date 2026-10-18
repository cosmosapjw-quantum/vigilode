# A01-UFREF-01: unequal-frequency independent reference

This is a fixed-input test/oracle unit. It does not change packet production code
or expand A01 authority. All binary64 inputs are converted to exact rational
numbers by `Fraction.from_float` before the Python ODE recurrence.

## Frozen input and Rust path

- N=3, xi=(-3/4,1/8,1+2^-49), weights=(1/8,3/8,1/2+2^-42).
- z=((1/4,-1/8),(-3/16,1/16),(1/8,3/16)); width=3/4;
  chi in (0,+3/8,-3/8); kappa=0; carrier=5/4 is metadata only.
- H=1/32; requested times=(0,H/4,H/2,3H/4,H).
- Fourth initial state uses chi=+3/8, independent mode radii eta=2^-30,
  and true perturbations ((+eta,0),(0,-eta),(-eta,0)).
- Each arm executes `compile_packet(model,H,4)` ->
  `CausalWeights::compile(4,12)` -> `generate_candidate(packet,4,2)` ->
  `RepresentedPacketCurve::freeze` -> `validate_source` ->
  `PacketCurveOutput::new` -> `workspace` -> `enclose_at`.

## Independent oracle and checks

Python stdlib `Fraction` derives the complex cubic ODE Taylor coefficients
through K=40 and K=48 by an exact recurrence. It expands the full residual
through degree 3K and requires exact zero at orders 0..K-1. It requires
weighted initial energy at most the smallest stored weight and coefficient
absolute sums on [0,H] at most one. Under these conditions it uses

`L=max_j |width*xi_j|+6|chi|S^3`, `S=sum_j w_j`,
`epsilon_K(t)=exp_plus(Lt)*t*rho_inf,K`.

Here `rho_inf,K` is the largest mode residual coefficient absolute sum on
[0,H], and `exp_plus` is the degree-80 rational Taylor sum plus the geometric
tail `x^81/81!/(1-x/82)`, valid only when `0<=x<=1`.
The oracle requires epsilon_40<=2^-120 and epsilon_48<=2^-160, exact shared
coefficient identity, intersection of both reference boxes, and
epsilon_48<=epsilon_40. Returned binary64 mode/mean centers and radii are
compared as exact rationals with the farthest corners of K=48 boxes using
squared complex distances and no slack. The chi=0 arm additionally uses
independent rational sine/cosine Taylor intervals. Changed endpoint center
and radius controls must fail inclusion.

The Rust target first calibrates the same thread-local allocator by cloning a
library `CollectiveModel` with three owned Vec buffers and requires at least
three observed allocation calls. Only after that positive control passes does
it accept zero allocation for a warmed `enclose_at` call. The detected count is
included in the oracle payload. The Rust target also checks 110
output-certificate operations per N=3 query. The oracle is outside the
measured section and is bounded by a 600-second process timeout. Any
nonzero exit retains the first log and is classified by whether the failed
assertion concerns mathematical recurrence/reference enclosure, source
identity/input, numerical inclusion, implementation/counters, or environment.

Full A01 remains HOLD after this unit: this reference is one fixed input,
and it does not close the remaining packet certificate arithmetic or A02.
