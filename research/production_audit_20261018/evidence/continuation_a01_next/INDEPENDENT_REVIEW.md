# Independent review: A01 capacity and recurrence slice

Review scope: read-only verification of the implementation at source EXEC `244da1b4211964cff47f1a08793c883e29eb1e7d`, reviewed by a requested gpt-6.1-sol/xhigh profile; observed runtime identity is not independently verified.

**Scoped PASS. Full A01 remains HOLD.** Streaming spectral powers preserve the prior directed repeated-multiply enclosure: before coefficient index k, each stored power is the k-fold interval multiplication by the represented xi. The loop reduces these products to Np and charges each attempt before the fallible multiplication. Duplicate complex multiplication removal reuses the same pure result. Checked lengths/reservations reject the demonstrated `usize::MAX-1` capacity panic, and output mode-count multiplication is checked before allocation. The new test target passed 38/38 after the source commit. No tolerance or certificate authority changed; `roundoff_enclosed` remains false.

Evidence limitation: the first-failure artifact is explicitly a transcription, and the worker receipt records Clippy/fmt/diff exits without their raw logs. This review admits only code identities and the retained final packet test log.

Remaining A01 obligations: complete force/convolution/output sizing and setup/replay/output arithmetic accounting, eliminate or bound nested infallible Vec/clone allocations, establish warmed allocation behavior, and obtain independent full theorem/implementation review. These remain typed HOLD.
