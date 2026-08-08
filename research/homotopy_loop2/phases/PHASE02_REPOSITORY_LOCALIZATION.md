# Phase 2 — Repository localization

The numerical primitives live in `rodas5p-core`; structured stage algebra and protected RODAS/SABR live in `rodas5p-integrators`; machine-readable commands live in `rodas5p-cli`. The bounded patch adds `integrators::homotopy`, exports it through `lib.rs`, and adds one CLI subcommand. No existing integration or Krylov call path is modified.
