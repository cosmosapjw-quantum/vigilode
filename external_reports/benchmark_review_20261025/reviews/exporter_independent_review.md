# Independent review of the local scalar exporter

**Result: accepted for read-only export of benchmark receipts after one correction round.** Fifteen author tests and nine independent adversarial tests pass. No solver or local-user process was launched or modified.

The review reproduced three defects and the author corrected all three. First, a receipt disappearing during the final reread could leave its timing-candidate flag true despite a read-error status. The exception path now clears that flag. Second, null index fields or a hash belonging to a different task could create an incorrect match. Index rows now require both a task identity and a valid SHA256, and matching uses the pair. Third, an extreme integer in a numeric JSON field could abort the exporter. It now becomes a record-level numeric failure.

Independent cases also exercise partially written receipts, duplicated result lines, modified stdout, external symlinks and wrapper duplication. The tests use synthetic files in temporary directories. The original before-correction run reported five passes, three assertion failures and one exception; the archived green log records the subsequent corrected run. The final source and test hashes are in `exporter_independent_review.json`.

The exporter reads a per-file snapshot of a changing directory. A terminal receipt written after enumeration will be collected on a later export. Input files are never opened for writing, and output must be a new directory outside the input root. An interrupted export can leave a partial output directory: consumers must require a parseable summary and matching measurement-file hash, and use a fresh directory for another attempt.

The wrapper and underlying receipt retain a shared evidence identity. They must be counted once when constructing repetitions. `timing_reanalysis_candidate` identifies records suitable for a subsequent matched comparison; host, executable, model, accuracy and timing scope still come from the exported metadata. Full-state vectors remain in the original local artifacts, where a new reference comparison can use them.
