"""Small pre-measurement source cleanup. Record and commit before building."""
from pathlib import Path
p=Path('crates/vigilode-coupled/src/solver.rs');s=p.read_text()
s=s.replace('Basis,apply,dot,finite,norm,small_solve','Basis,dot,finite,norm,small_solve')
s=s.replace('y:&\'a[f64],h:f64,hg:f64','y:&\'a[f64],hg:f64')
s=s.replace('Frozen{p,t,y,h,hg,c,cfg','Frozen{p,t,y,hg,c,cfg')
p.write_text(s)
p=Path('crates/vigilode-coupled/src/lib.rs');s=p.read_text().replace('pub use fallback::{SelectedStep, coupled_or_sequential};','pub use fallback::{FallbackResult, SelectedStep, coupled_or_sequential};');p.write_text(s)
p=Path('.github/workflows/coupled-native-20261020.yml');s=p.read_text().replace('|| exit0','|| exit 0');p.write_text(s)
