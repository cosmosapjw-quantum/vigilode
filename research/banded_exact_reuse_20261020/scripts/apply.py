#!/usr/bin/env python3
"""Exact-parent patch; no changes to the already-correct retry invalidation.
Publish the changed source before compiling a measurement binary.
"""
import hashlib,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
P=ROOT/'crates/rodas5p-integrators/src/rodas5p_fast.rs'
OUT=Path(sys.argv[1]);OUT.mkdir(parents=True,exist_ok=True)
s=P.read_text();b=s.encode();before=hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()
if before!='7756df18367b84813f7d314a27d72b45c80b6a0d':raise RuntimeError('UNEXPECTED_PARENT:'+before)
original=s

def one(a,z):
    global s
    if s.count(a)!=1:raise RuntimeError('NONUNIQUE_ANCHOR:'+a[:110])
    s=s.replace(a,z,1)
def method(name):
    start=original.index('    fn '+name+'(')
    end=original.index('\n    }',start)+len('\n    }')
    return original[start:end]

one('use std::sync::Arc;','use std::sync::Arc;\n\n#[path = "exact_band_cache.rs"]\nmod exact_band_cache;\nuse exact_band_cache::ExactBandCache;')
one('    Slices,\n}', '''    Slices,
    /// Opt-in Slices specialization for lower=upper=2. Arithmetic order
    /// and pivot ties are unchanged; all other widths fall back to Slices.
    FixedWidth2,
    /// FixedWidth2 plus exact finite shifted-matrix LU reuse. This still
    /// evaluates J and f at every new state, and never freezes J approximately.
    FixedWidth2ExactCache,
}''')
one('    kernel: BandedKernel,\n}', '    kernel: BandedKernel,\n    exact_cache: Option<ExactBandCache>,\n}')
one('''            kernel,
        };
        work.band''','''            kernel,
            exact_cache: (kernel == BandedKernel::FixedWidth2ExactCache)
                .then(|| ExactBandCache::new(n * (l + u + 1))),
        };
        work.band''')
one('''        kernel: BandedKernel::Indexed,
    };
    state.factor''','''        kernel: BandedKernel::Indexed,
        exact_cache: None,
    };
    state.factor''')
one('stored_slots: n * (3 * l + 2 * u + 2),', '''stored_slots: n * (3 * l + 2 * u + 2)
                    + if kernel == BandedKernel::FixedWidth2ExactCache { n * (l + u + 1) } else { 0 },''')
one('/// f64 slots of the Jacobian band and the factors.', '/// Eight-byte slots of the Jacobian band, factors and optional exact key.')
one('''        if self.kernel == BandedKernel::Slices {
            return self.factor_slices(n, inv);
        }''','''        if matches!(self.kernel, BandedKernel::FixedWidth2 | BandedKernel::FixedWidth2ExactCache)
            && self.l == 2 && self.u == 2
        {
            return self.factor_fixed2(n, inv);
        }
        if self.kernel != BandedKernel::Indexed {
            return self.factor_slices(n, inv);
        }''')
one('''        if self.kernel == BandedKernel::Slices {
            return self.solve_slices(n, b);
        }''','''        if matches!(self.kernel, BandedKernel::FixedWidth2 | BandedKernel::FixedWidth2ExactCache)
            && self.l == 2 && self.u == 2
        {
            return self.solve_fixed2(n, b);
        }
        if self.kernel != BandedKernel::Indexed {
            return self.solve_slices(n, b);
        }''')
fixedfactor=method('factor_slices').replace('fn factor_slices','fn factor_fixed2',1)
fixedfactor=fixedfactor.replace('(self.l, self.u, self.width())','(2usize, 2usize, 7usize)',1).replace('self.at(', 'at(')
fixedfactor=fixedfactor.replace('        let jw = l + u + 1;', '        let at = |i: usize, j: usize| i * 7 + (j + 2 - i);\n        let jw = l + u + 1;',1)
fixedsolve=method('solve_slices').replace('fn solve_slices','fn solve_fixed2',1).replace('(self.l, self.u, self.width())','(2usize, 2usize, 7usize)',1)
cachemethod='''    /// Reuse only successful finite factors of an exactly identical matrix.
    /// Solves do not mutate factors. Failed factorization invalidates the key.
    fn factor_with_policy(&mut self, n: usize, inv: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        if self.exact_cache.as_ref().is_some_and(|key| key.matches(inv, &self.jacobian)) {
            return Ok(());
        }
        if let Some(key) = self.exact_cache.as_mut() { key.invalidate(); }
        counters.direct_factorizations += 1;
        self.factor(n, inv)?;
        if let Some(key) = self.exact_cache.as_mut() {
            if self.factors.iter().all(|x| x.is_finite()) { key.record(inv, &self.jacobian); }
        }
        Ok(())
    }
'''
one('\n}\n\n/// Matrices with at most', '\n\n'+fixedfactor+'\n\n'+fixedsolve+'\n\n'+cachemethod+'}\n\n/// Matrices with at most')
one('''        if let Some(band) = self.band.as_mut() {
            counters.direct_factorizations += 1;
            return band.factor(n, inv);
        }''','''        if let Some(band) = self.band.as_mut() {
            return band.factor_with_policy(n, inv, counters);
        }''')
start=s.index('    if n == 0\n',s.index('pub fn rodas5p_fast_banded_solve('));end=s.index('    let mut state = BandState {',start)
s=s[:start]+'''    let invalid = || CoreError::InvalidInput("invalid banded solve input or storage size".into());
    if n == 0 || lower >= n || upper >= n || b.len() != n { return Err(invalid()); }
    let width = lower.checked_mul(2).and_then(|v| v.checked_add(upper))
        .and_then(|v| v.checked_add(1)).ok_or_else(invalid)?;
    let jwidth = lower.checked_add(upper).and_then(|v| v.checked_add(1)).ok_or_else(invalid)?;
    let jlen = n.checked_mul(jwidth).ok_or_else(invalid)?;
    let flen = n.checked_mul(width).filter(|v| *v <= isize::MAX as usize / std::mem::size_of::<f64>())
        .ok_or_else(invalid)?;
    if jacobian.len() != jlen { return Err(invalid()); }
    if !inv.is_finite() || jacobian.iter().chain(b).any(|x| !x.is_finite()) {
        return Err(CoreError::NonFinite("nonfinite banded solve input".into()));
    }
'''+s[end:]
one('factors: vec![0.0; n * (2 * lower + upper + 1)],', 'factors: vec![0.0; flen],')
one('''    state.solve(n, &mut x);
    Ok((x, state.pivots))''','''    state.solve(n, &mut x);
    if x.iter().any(|v| !v.is_finite()) {
        return Err(CoreError::LinearSolve("banded solve produced NaN/Inf".into()));
    }
    Ok((x, state.pivots))''')
one('''            Some((_, BandedKernel::Slices)) => RODAS5P_FAST_BANDED_SLICES_DRIVER_ID,''','''            Some((_, BandedKernel::Slices)) => RODAS5P_FAST_BANDED_SLICES_DRIVER_ID,
            Some((_, BandedKernel::FixedWidth2)) => "rodas5p-fast-banded-fixed2-v1",
            Some((_, BandedKernel::FixedWidth2ExactCache)) => "rodas5p-fast-banded-fixed2-exact-cache-v1",''')
s+='\n#[cfg(test)]\n#[path = "banded_exact_tests.rs"]\nmod banded_exact_tests;\n'
P.write_text(s)
receipt={'parent_blob':before,'new_sha256':hashlib.sha256(s.encode()).hexdigest(),'changed':str(P.relative_to(ROOT)),
 'default_kernel_unchanged':True,'retry_recovery_unchanged':True,
 'refuted_hypothesis':'baseline callback-retry tests passed; the existing Err branch invalidates fresh_state',
 'reuse':'exact finite shift and Jacobian bits; Jacobian and RHS calls are not skipped'}
(OUT/'SOURCE_PATCH.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
