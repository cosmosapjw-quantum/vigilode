# Selective supplied-JVP control

Reuse PR90's existing adaptive evidence switcher. No new selector theorem or
threshold fitting is claimed. A supplied JVP-only provider is present in both
arms, and derivative authorization differs: Forbidden vs ProvidedOnly.

Autonomous y'=-rate*y, n=16, y0_i=1, [0,1], rate=1,1000,10000. Same 8 requested
windows, rtol1e-7, atol1e-10, h0=1e-4, max50000 attempts. Every returned component
must pass exact exp(-rate*t) error1e-6. This is a sampled trajectory ceiling.
Native callback atomics must equal accounted RHS/JVP totals; Jacobian builds
and explicit-time derivatives must be zero. All losing probes remain charged.
The supplied-JVP path is chosen only through existing same-checkpoint evidence;
on the easy control it may perform a bounded losing pilot if work pressure
triggers one. Such pilot cost will not be hidden. No timing is planned.
