# V1q integer-only admission: frozen next node

Parent PR81 source cc49cfd1017fc0639bf9ef6cc08535ff5a06ff7e. New general ODE source14171ed0c4f7dc453f47870cf6c1399f41afc4f2 is inspected and its callback/scale/persistent-session results reused, not copied or rerun. Native V1 owner and local dirty state remain unverified; no takeover or production hook edit.

Objective: fixed PR79 compact30/full33 physical norm threshold evaluated from binary64 bit patterns by integer operations only. Retain exact signed <=3-term rows, full denominator33 and scales supplied by caller. Approximate each nonnegative exact ratio by a certified floor/ceiling grid at2^-56; accumulate squared integer bounds. Return Accept/Reject/Indeterminate without floating arithmetic. A ratio>=8 already implies E²>=64/33>1. Validate every input before any early decision.

Theory: all finite binary64 are integers in units2^-1074. Row magnitude<2^2100. Shift by56 fits34u64 words. Ratios<8 give ceil quotient<=2^59, and33*2^118<2^128. Exact remainder controls upper endpoint; no false certainty at zero or exact boundary. This is an arithmetic predicate, not a norm estimate, physical scale constructor or trajectory certificate.

Stages S0 intake/contract; S1 derivation+RED; S2 implementation, exact oracle, bounded FP-mode experiment and kernel cost; S3 review/delivery/handoff. Each macro-stage backed up and published; R/C stage mapping in final DAG. Original PR81 oracle/reference rows may be inputs to new code but no old solver rerun. A finite corpus is not a universal formal proof. Native subagent and independent-personnel review NOT_RUN; separate computational oracle and actual Wolfram checks available.

New local compiler/test/environment subprocess cap120s with cumulative ledger, prior V1p10.984972582s and PR807.354545033s preserved. Older campaign balance unknown, not zero. No NCP, cloud job or physics IVP. No user machine, old branch, default source, sharing settings, force push or merge.

Intake: complete matching-ref metadata response, selectively read latest source; do not claim every file on every branch audited. Related PRs79-81, REI220-222 and BASS182-184 checked; adjacent science campaigns not repeated. Conversation search limited to accessible context. Dropbox VIGILODE_V1 returned11items and exhausted pagination. Drive dated search returned0; existing pinned backup is not disproved. Parent root/research AGENTS reads returned404; no claim about local or other-branch instructions.

S0 backup VIGILODE_V1Q_S0_20261024.zip1952bytes SHA25621414cc83ca7517a70b425a202bc8e48a63683fa14da00285611b2eff0f3837b. Drive1axETtjYKH6CvDJbtDJY6hB2MrCCwBg6U upload acknowledged; Dropbox initiated pending status. This checkpoint is not an implementation PASS.
