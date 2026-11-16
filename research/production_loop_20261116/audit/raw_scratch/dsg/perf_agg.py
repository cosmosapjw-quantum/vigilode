import json, collections, re
tot = collections.Counter(); bysym = collections.Counter(); n = 0
for line in open('/tmp/claude-1000/dsg-audit/perf_script.txt'):
    m = re.match(r"\s*(\S+)\s+(\d+)\s+[0-9a-f]+\s*(.*)$", line.rstrip("\n"))
    if not m: continue
    comm, per, sym = m.group(1), int(m.group(2)), m.group(3).strip()
    tot[comm] += per; bysym[(comm, sym)] += per; n += 1
total = sum(tot.values()); main = tot['u11-holdout-run']
def share(pred): return sum(v for (c, s), v in bysym.items() if c == 'u11-holdout-run' and pred(s))
tab = share(lambda s: s in ('rodas5p_integrators::ppsw1::mm', 'rodas5p_integrators::ppsw1::tableau'))
fma = share(lambda s: s == 'fma' or 'fma_with_fma' in s)
rod = share(lambda s: 'rodas5p_fast' in s)
ppsw_other = share(lambda s: 'ppsw1' in s) - tab
alloc = share(lambda s: any(k in s for k in ('malloc', 'free', 'calloc', 'memalign', 'unlink_chunk', 'realloc')))
# tableau calls = PPSW1 attempts x 2 arms x 9 reps (from the rerun raw output of the same process)
runs = [json.loads(l) for l in open('/tmp/claude-1000/dsg-audit/rerun_holdout_runs.jsonl')]
att = sum(r['result']['accepted'] + r['result']['rejected'] for r in runs if r['arm'] == 'ppsw1_t1')
att6 = sum(r['result']['accepted'] + r['result']['rejected'] for r in runs if r['arm'] == 'ppsw1_t6')
calls = 9 * (att + att6)
out = {"samples": n, "total_cycles_u": total, "main_thread_share": main/total,
       "rayon_worker_share": 1 - main/total,
       "tableau_mm_share_of_total": tab/total, "tableau_mm_share_of_main_thread": tab/main,
       "fma_libcall_share_of_main_thread": fma/main,
       "tableau_mm_fma_share_of_main_thread": (tab+fma)/main,
       "rodas5p_fast_symbols_share_of_main_thread": rod/main,
       "other_ppsw1_symbols_share_of_main_thread": ppsw_other/main,
       "malloc_free_share_of_main_thread": alloc/main,
       "ppsw1_attempts_per_rep_t1": att, "tableau_calls_in_process": calls,
       "cycles_per_tableau_call_mm_plus_tableau": tab/calls, "cycles_per_tableau_call_incl_fma": (tab+fma)/calls}
json.dump(out, open('/tmp/claude-1000/dsg-audit/perf_agg.json', 'w'), indent=1)
print(json.dumps(out, indent=1))
