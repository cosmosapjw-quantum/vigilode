#!/usr/bin/env python3
"""Generate a common-model comparison adapter from the frozen prior adapter.
Only arm dispatch changes; model, references, timers and output semantics remain.
"""
from pathlib import Path
import hashlib,json,sys
out=Path(sys.argv[1]);src=Path('crates/vigilode-coupled/examples/coupled_bench.rs');s=src.read_text();b=s.encode()
assert hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()=='a00a2d477c53ff19c31ffdeb6e3c375f6c294b8c'
def replace(a,z):
 global s
 assert s.count(a)==1,a
 s=s.replace(a,z,1)
replace('use vigilode_coupled::{Attempt, Config, SelectedStep, coupled_or_sequential, coupled_step};',
 'use vigilode_coupled::{Attempt, Config, SelectedStep, coupled_step, coupled_step_residual_first, coupled_or_sequential_budgeted};')
replace('''        "coupled-1" | "sequential-mf" => 1,
        "coupled-2" => 2,
        "coupled-4" | "transactional-4" => 4,''','''        "eager-1" | "lazy-1" | "sequential-mf" => 1,
        "eager-4" | "lazy-4" | "txn" | "cap64" | "cap256" => 4,''')
replace('''        } else if arm == "transactional-4" {
            Run::Transactional(coupled_or_sequential(
                &problem, 0.0, &y, h, &cfg, &exec, &lin,
            ))
        } else {
            Run::Coupled(coupled_step(&problem, 0.0, &y, h, &cfg, &exec))
        }''','''        } else if matches!(arm.as_str(), "txn" | "cap64" | "cap256") {
            let cap = match arm.as_str() { "cap64" => 64, "cap256" => 256, _ => cfg.max_jvp };
            Run::Transactional(coupled_or_sequential_budgeted(
                &problem, 0.0, &y, h, &cfg, &exec, &lin, cap,
            ))
        } else if arm.starts_with("lazy-") {
            Run::Coupled(coupled_step_residual_first(&problem, 0.0, &y, h, &cfg, &exec))
        } else {
            Run::Coupled(coupled_step(&problem, 0.0, &y, h, &cfg, &exec))
        }''')
replace('"schema":"coupled-native-step-benchmark-v1"','"schema":"coupled-economy-step-benchmark-v1"')
replace('"root_completed":complete,', '''"candidate_jvp_cap":match arm.as_str(){"cap64"=>Some(64u64),"cap256"=>Some(256u64),"txn"=>Some(cfg.max_jvp),_=>None},
  "root_completed":complete,''')
dest=Path('crates/vigilode-coupled/examples/economy_bench.rs');dest.write_text(s)
(out/'BENCHMARK_SOURCE.json').write_text(json.dumps({'template_sha256':hashlib.sha256(b).hexdigest(),'generated_sha256':hashlib.sha256(s.encode()).hexdigest(),'models_and_reference_unchanged':True},indent=2)+'\n')
