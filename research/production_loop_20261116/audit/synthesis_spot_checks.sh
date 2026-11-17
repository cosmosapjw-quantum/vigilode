#!/usr/bin/env bash
# Loop L1 audit, synthesis stage: read-only spot checks of findings that were not sent to an
# independent verifier, plus the git facts used by LINEAGE_ADDENDUM.md.
# Usage: bash synthesis_spot_checks.sh > synthesis_spot_checks.log 2>&1
# Reads only; writes nothing except stdout. No cargo, no network, no fetch.
set -u
LOOP=/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116
SW=/home/cosmosapjw/vigilode
S=933922495045796c99c530b0f1f1efd8fec83ed0
R=6844c8fe18361f3c2243ee2db95bbf3d68a6ce7b
MB=569ce02629498fec133db9ce368eb303ba943b69
cd "$LOOP" || exit 2
H=$(git rev-parse HEAD)
U=research/production_unification_20261115

run() {
  echo
  echo "\$ $*"
  bash -c "$*" 2>&1
  echo "[exit $?]"
}

echo "# synthesis-stage spot checks, loop L1 audit; $(date -Iseconds); loop HEAD $H"

echo
echo "## C01 EVD-01/02/03: subprocess, os.system and open( counts in the six switcher eval scripts at $S"
for p in \
  research/k1_shared_krylov_basis_20261104/eval_k1.py \
  research/b7_first_batch_20261104/eval_b7.py \
  research/c1_coloured_assembly_20261104/eval_c1.py \
  research/d1_stiffness_gate_20261104/eval_d1.py \
  research/h1_hysteresis_band_20261104/eval_h1.py \
  research/e1r_work_model_v2_20261104/eval_e1r.py; do
  src=$(git -C "$SW" show "$S:$p")
  echo "$p subprocess=$(grep -c 'subprocess' <<<"$src") os_system=$(grep -c 'os\.system' <<<"$src") open_calls=$(grep -c 'open(' <<<"$src")"
done

echo
echo "## C02 EVD-01..04: ledger rows L-0205..L-0214 on the switcher line"
git -C "$SW" show "$S:research/LEDGER.jsonl" | python3 -I -c '
import sys, json
want = {"L-02%02d" % i for i in range(5, 15)}
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    r = json.loads(line)
    if r.get("id") in want:
        print(r["id"], r.get("verdict"), "supersedes=", r.get("supersedes"),
              "commit=", str(r.get("commit"))[:8], "cmd=", str(r.get("command"))[:80])
'
run "git -C $SW ls-tree -r --name-only $S | grep -cE 'k1_shared_basis_bench|b7_first_batch_bench'"
run "git -C $SW log --all --oneline -- '*k1_shared_basis_bench*' '*b7_first_batch_bench*' | wc -l"

echo
echo "## C03 EVD-06: workflows on origin/main (local ref, no fetch) and production.yml triggers"
run "git ls-tree --name-only origin/main .github/workflows/"
run "git log -1 --format='%H %ad' --date=iso origin/main"
run "ls .github/workflows | wc -l"
run "sed -n '10,25p' .github/workflows/production.yml"

echo
echo "## C04 EVD-07: ledger tail and research-node check"
run "wc -l < research/LEDGER.jsonl; grep -c 'production_unification' research/LEDGER.jsonl; tail -n 1 research/LEDGER.jsonl | cut -c1-160"
run "python3 -I tools/check-research-node.py --base bee5563c51264a7da19c519295bdda2fb317caef"

echo
echo "## C05 EVD-10a: literal G0 passes and the SUNDIALS string in the U11 evaluator"
run "sed -n '111,122p' $U/production_holdout/evaluate.py"
run "ls /usr/lib/x86_64-linux-gnu/libsundials_cvode.so.6; dpkg -s libsundials-dev 2>/dev/null | grep -E '^(Status|Version)'"

echo
echo "## C06 EVD-08: process-global live-byte counter in the sp02 test"
run "grep -n 'global_allocator\|LIVE_BYTES\|Mutex\|serial\|#\[test\]' crates/rodas5p-krylov/tests/sp02_workspace_charge.rs | head -40"

echo
echo "## C07 EVD-09a: matched status counts in the spatial grid v2 raw file against the receipt text"
run "git -C $SW show $S:bench/out/adversarial_spatial_grid_v2_results.json | python3 -I -c 'import json,sys,collections; d=json.load(sys.stdin); print(len(d[\"matched\"]), dict(collections.Counter(m[\"status\"] for m in d[\"matched\"])))'"
run "git -C $SW show $S:docs/agent/production_solver_qualification_20261107_v3/FINAL_REPORT_KO.md | grep -n '953' | cut -c1-160"

echo
echo "## C08 EVD-05: agent_gate.py edits inside gated commits"
run "git -C $SW log --format='%h %ad %s' --date=short 4339facc..$S -- tools/agent_gate.py | cat"

echo
echo "## C09 COD-05/06: switcher router dispatch, failure mapping, accounting check"
run "git -C $SW show $S:crates/vigilode/src/unified/router.rs | sed -n '60,76p;283,292p'"
run "git -C $SW show $S:crates/vigilode/src/hybrid_automaton.rs | sed -n '313,321p'"

echo
echo "## C10 COD-07: preflight contractivity rule and n*n workspace"
run "git -C $SW show $S:crates/vigilode/src/unified/analyzer.rs | sed -n '172,192p'"
run "git -C $SW show $S:crates/vigilode/src/unified/workspace.rs | sed -n '40,50p'"

echo
echo "## C11 COD-09/10: reject files, workspace members, entry-point counts"
run "git ls-files | grep -E '\.rej\$'"
run "sed -n '1,12p' Cargo.toml"
run "grep -nE 'pub fn solve[a-z_]*\(' crates/vigilode/src/*.rs | wc -l; grep -c 'PeerOde\|MethodRegistry\|ProductionSolver\|AutomaticOde' README.md; sed -n '3,4p' crates/vigilode/README.md"

echo
echo "## C12 DSG-09/10 and COD-01 call site: OD-15 text, cap constant, tableau call, runner profile"
run "grep -n 'OD-15' docs/agent/unified_local_agent_20261109/OWNER_DECISIONS_20261110.json | cut -c1-260"
run "sed -n '57,58p' crates/rodas5p-integrators/src/ppsw1.rs; grep -n 'tableau(sigma)' crates/rodas5p-integrators/src/ppsw1.rs"
run "cat $U/production_holdout/runner/Cargo.toml; sed -n '33,40p' Cargo.toml"

echo
echo "## C13 DSG-07: speed arm in the U11 evaluator and commit times of preregistration and evaluator"
run "sed -n '86,90p' $U/production_holdout/evaluate.py"
run "git log --format='%h %ad %s' --date=iso -- $U/production_holdout/evaluate.py | cat; echo --; git log --format='%h %ad %s' --date=iso -- $U/production_holdout/PREREGISTRATION.json | cat"

echo
echo "## C14 lineage: ancestry, unique commits, protected files changed on the switcher line, absence from lineage documents"
run "git merge-base $S $H; git merge-base --is-ancestor $S $H; echo switcher_is_ancestor_of_loop_exit=\$?; echo switcher_only=\$(git rev-list --count $H..$S) loop_only=\$(git rev-list --count $S..$H)"
run "git merge-base $R $H; git merge-base --is-ancestor $R $H; echo rei_is_ancestor_of_loop_exit=\$?; echo rei_only=\$(git rev-list --count $H..$R); git log -1 --format='%H %ad %s' --date=iso $R; git branch -a --contains $R | cat"
run "git log -1 --format='%H %ad' --date=iso \$(git merge-base $R $H)"
run "git log -1 --format='%H %ad' --date=iso $MB; echo rei_only_dated_before_20261115=\$(git rev-list --count --before=2026-11-15T00:00:00+09:00 $H..$R)"
run "echo followup_only_vs_switcher=\$(git rev-list --count $S..612063db); git branch -a --contains 612063db | cat"
run "git diff --stat=200 $MB $S -- fixtures/rodas5p_coefficients_snapshot.json crates/rodas5p-core/src/coefficients.rs crates/rodas5p-core/src/work.rs crates/rodas5p-integrators/src/sequential.rs crates/rodas5p-integrators/src/parallel.rs crates/rodas5p-integrators/src/stage_batch.rs crates/rodas5p-integrators/src/ptsw2b.rs crates/rodas5p-integrators/src/candidates.rs crates/rodas5p-integrators/src/source_owned_graph_schur.rs crates/rodas5p-integrators/src/rodas5p_fast.rs crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs crates/rodas5p-integrators/src/rodas5p_fast_dense.rs crates/rodas5p-integrators/src/rodas5p_lagged.rs crates/rodas5p-integrators/src/client_stage.rs crates/rodas5p-integrators/src/adaptive.rs crates/rodas5p-krylov/src/block_gmres.rs crates/vigilode/src/lib.rs crates/vigilode/src/automatic.rs crates/vigilode/src/switching.rs Cargo.toml Cargo.lock rust-toolchain.toml | cat"
run "git diff --stat=200 $MB $H -- fixtures/rodas5p_coefficients_snapshot.json crates/rodas5p-core/src/coefficients.rs crates/rodas5p-core/src/work.rs crates/rodas5p-integrators/src/sequential.rs crates/rodas5p-integrators/src/parallel.rs crates/rodas5p-integrators/src/stage_batch.rs crates/rodas5p-integrators/src/ptsw2b.rs crates/rodas5p-integrators/src/candidates.rs crates/rodas5p-integrators/src/source_owned_graph_schur.rs crates/rodas5p-integrators/src/rodas5p_fast.rs crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs crates/rodas5p-integrators/src/rodas5p_fast_dense.rs crates/rodas5p-integrators/src/rodas5p_lagged.rs crates/rodas5p-integrators/src/client_stage.rs crates/rodas5p-integrators/src/adaptive.rs crates/rodas5p-krylov/src/block_gmres.rs crates/vigilode/src/lib.rs crates/vigilode/src/automatic.rs crates/vigilode/src/switching.rs Cargo.toml Cargo.lock rust-toolchain.toml | cat"
run "git show $S:rust-toolchain.toml | sed -n 2p; git show $H:rust-toolchain.toml | sed -n 2p"
run "cd $U && for p in 93392249 612063db 6844c8fe switcher rei REI 569ce026; do echo \"\$p lineage.json=\$(grep -c -- \$p lineage.json) LINEAGE.md=\$(grep -c -- \$p LINEAGE.md) HISTORY_RECOVERY.json=\$(grep -c -- \$p HISTORY_RECOVERY.json) git_inventory.json=\$(grep -c -- \$p git_inventory.json)\"; done; sha256sum lineage.json; grep -n 'lineage.json' CHECKPOINT.json | cut -c1-140"
python3 -I - <<'EOF'
import json, subprocess
U = "research/production_unification_20261115/"
d = json.load(open(U + "git_inventory.json"))
print()
print("refs of the two absent lineages as recorded in git_inventory.json:")
for x in d["refs"]:
    if x["commit"][:8] in ("93392249", "612063db") or "rei" in x["ref"].lower():
        print(" ", x["ref"], x["commit"][:8], x["commit_date"][:10],
              "ancestor_of_base=", x["ancestor_of_current_research_base"])
l = json.load(open(U + "lineage.json"))
cs = set()
def walk(o):
    if isinstance(o, dict):
        for k, v in o.items():
            if k == "commit" and isinstance(v, str):
                cs.add(v)
            walk(v)
    elif isinstance(o, list):
        for v in o:
            walk(v)
walk(l["tracks"])
print("tracks:", len(l["tracks"]), "distinct source commits cited by lineage tracks:", len(cs))
S = "933922495045796c99c530b0f1f1efd8fec83ed0"
R = "6844c8fe18361f3c2243ee2db95bbf3d68a6ce7b"
MB = "569ce02629498fec133db9ce368eb303ba943b69"
def anc(a, b):
    return subprocess.run(["git", "merge-base", "--is-ancestor", a, b]).returncode == 0
for c in sorted(cs):
    on_sw_only = anc(c, S) and not anc(c, MB)
    on_rei_only = anc(c, R) and not anc(c, "HEAD")
    print(" ", c[:8], "ancestor_of_loop_head=", anc(c, "HEAD"),
          "switcher_only=", on_sw_only, "rei_only=", on_rei_only)
EOF
echo
echo "# end"
