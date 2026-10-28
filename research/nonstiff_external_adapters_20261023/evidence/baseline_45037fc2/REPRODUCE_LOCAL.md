# Local reproduction receipt

Measured source:45037fc24e8f8101605f309e1fdb3a9a4e3df56d.
Tree:b43b69b1ec5b86f3577d7cd6efdefe2d3acb9bad.
Published REPRODUCE/HANDOFF copied from8c80985e into published-docs.
Source worktree:/home/cosmosapjw/vigilode-nonstiff-source-WjxDXM (detached, clean).
Run root:/home/cosmosapjw/vigilode-nonstiff-run-Ioh017, outside source.

execute_validation.sh and execute_benchmarks.sh are exact execution records.
They contain actual variable values, CARGO_TARGET_DIR, two-job/debug settings,
locked/offline commands, source label and budget limits. They are not restart
scripts: do not rerun them into this existing directory or overwrite logs.
For a new run use a fresh mktemp directory/worktree and the saved published
REPRODUCE.md. Preserve its own source/tree/binary/environment identities.

Initial metadata failed101 on anstyle-wincon3.0.11. The failed stdout/stderr,
exit code and hashes are retained separately. User-authorized cargo fetch
--locked took about3s and the next offline metadata check passed. Cargo.lock
is unchanged. Dependency preparation, builds, process wall, native solve
seconds and analysis are separate in EXECUTION_RECEIPT.json.

Main run: --external --full --budget-seconds7200,45s/process, default seed,
144settings,1warmup+3timed each. Only bundled VODE was an available compatible
external adapter. Diffsol/Julia are NOT_RUN with reasons in EXTERNAL_ADAPTERS.
Ensemble run followed main+analysis sequentially with its own180s budget.
Raw scripts, receipt argv, summaries and saved states are retained.

analyze_bench.py at the fixed source produced main SUMMARY/REPLAY/figures and
archives without rerunning solves. summarize_local_run.py in this result root
then verified all source input/binary/raw/archive hashes and summarized saved
data into RESULTS.json. write_report.py rendered REPORT_KO.md. REVIEW.json is
an independent read-only result/claims check. finalize_local_run.py attaches
that review and writes the outer artifact manifest, excluding build/cache
intermediates while including both measurement binaries explicitly.

No solver source was edited. No NCP, SSH, new external installation, source
reset/clean/forced checkout, old-result deletion, retuning or remote publication.
