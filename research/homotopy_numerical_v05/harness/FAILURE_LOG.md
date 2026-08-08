# Failure log

- Runtime reset removed the first worktree and local toolchain during baseline compilation. Reconstructed from durable bundle; no transcript-only code inherited.
- An exploratory six-level global-order run exceeded the execution window because q=7 plus exact target-Jacobian certification is expensive over many timesteps. Partial output was discarded. The final four-level screen completed and was reproduced byte-for-byte.

- A separate review-worktree test invocation exceeded the execution window during a cold release
  compile. It produced no test failure evidence and was not used as verification.
- The first independent result-parser helper assumed the SABR control name was `sabr-nilpotent`;
  the actual emitted name is `sabr-nilpotent-3`. The helper stopped on its assertion, was corrected,
  and then reproduced the canonical counts; no source or result artifact was changed by the helper.

- Final-gate attempt 1 called `rodas5p validate` without its required `--output` argument. The
  validation script was corrected and the gate restarted from the beginning.
- Final-gate attempt 2 expected the canonical screen status literal `pass`; the emitted contract is
  `screen-complete`. The script assertion was corrected and the gate restarted from the beginning.
- Final-gate attempt 3 reached `git diff --check`, which found Markdown hard-break trailing spaces in
  the independent review header. They were removed before the next exact-head gate.
