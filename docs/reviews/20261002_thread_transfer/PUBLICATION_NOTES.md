# Publication and terminology notes

The review's phrase "현재 operational q2 경로" in section 3.3 refers to the currently used **native-certificate admission path**, not the enum variant `Q2Admission::OperationalDiagnostic`. Precisely: `Q2Admission::NativeTargetCertificate` invokes the serial `certify_stage_target`; `OperationalDiagnostic` uses the separate eighth diagnostic batch. The radius/doubling experiments in this review do not change either path.

Publication uses GitHub's named-file Contents API on the pre-existing branch `claude/jolly-wozniak-7wl15h-wu25-stiff-benchmark`. Each accepted file creation produces an ordinary commit on that branch. It is a series of additive commits, not a single atomic local `git push`. No branch or pull request was created and no force update was requested.

A raw Git object (`create_blob`) attempt was blocked because its security status could not be determined. It produced no confirmed object SHA. The explicitly scoped named-file API accepted the user-requested review files. This delivery does not claim that the blocked call succeeded.

The source review and exact probes were performed at commit `d1e9ba3b0125ee478c28d0b2c280ca0869289c15`. The protocol was committed before the new probes at `f2797d8891eb7201da31576080f2118bb23b7b1b`. Final branch/diff and blob-identity verification are recorded in the delivery receipt. Native Rust, native CI and native performance results remain unexecuted.
