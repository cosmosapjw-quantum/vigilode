# Next Node after v3.7 Continuation Transaction

## Required transition

Merge the continuation-transaction implementation PR only after reviewing its
bounded-resume API, atomic batch guard, dedicated v3.7 schema, 64/62/2 replay,
and compatibility evidence.

## Next implementation node

`v3.7 Timing Authority Validator`

Implement the already-sealed whole-campaign host-quality validator without
producing a new paired-wall campaign. The validator must:

- attest exact Git/toolchain/binary/contract identity;
- capture host, kernel, CPU, microcode, NUMA, governor, turbo, affinity, and
  thread-environment fields;
- enforce the predeclared idle/steal, swap, exposed thermal-throttle, arm-span,
  and order-sensitivity gates;
- reject only whole campaigns while retaining every pair and every failed
  attempt;
- require three passing complete campaigns within five attempts;
- remain independent of whether the shadow/R ratio is favorable.

Only after that validator is merged may a new timing campaign be generated.
Even a passing campaign supports descriptive host-qualified timing only; it
still does not authorize a speedup claim or active switching.
