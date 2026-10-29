from pathlib import Path
p=Path('crates/vigilode-hybrid/src/native.rs')
s=p.read_text();old='w.accumulate(&callback_cost.lock().map_err(|_| "counter lock")?);'
if old in s:
    assert s.count(old)==1
    s=s.replace(old,'let observed = callback_cost.lock().map_err(|_| "counter lock")?;\n        w.accumulate(&observed);')
    p.write_text(s)
else:
    assert 'w.accumulate(&observed);' in s
# Only the new crate is edited; no workflow or historical evidence mutation.
