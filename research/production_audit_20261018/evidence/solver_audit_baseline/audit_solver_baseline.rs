use rodas5p_core::{DenseMatrix, WorkCounters};
use rodas5p_core::leja_action::{CertifiedLejaCoeffCache, CertifiedLejaWorkspace, leja_phi_action_certified_with_cache_and_workspace, leja_node_bounds, leja_points};
use rodas5p_core::polynomial_action::{JointPhiInput, SymmetricNonpositiveOperator};
use rodas5p_core::sparse_direct::SparseDirectSolver;
fn main() {
 let op=SymmetricNonpositiveOperator::gershgorin(DenseMatrix::from_rows(&[&[-1.,0.],&[0.,-2.]]).unwrap()).unwrap();
 let w=[1.,1.]; let inp=JointPhiInput::SameVector {vector:&w,scales:[1.,0.,0.,0.,0.]};
 let mut work=WorkCounters::default(); let mut ws=CertifiedLejaWorkspace::new(2,1); let mut cache=CertifiedLejaCoeffCache::new();
 let rep=leja_phi_action_certified_with_cache_and_workspace(&op,1.,inp,1e-6,16,Some(&mut cache),&mut work,&mut ws).unwrap();
 let center=rep.center; let scale=rep.scale;
 cache.insert_certified(center,scale,16,vec![[0.;5];17],vec![[0.;5];17]);
 let bad=leja_phi_action_certified_with_cache_and_workspace(&op,1.,inp,1e-6,16,Some(&mut cache),&mut work,&mut ws).unwrap();
 println!("FORGED_CACHE status={:?} fused={:?} bound={} actual_error={}",bad.total_error(),bad.fused,bad.total_bound(),((-1.0_f64).exp().powi(2)+(-2.0_f64).exp().powi(2)).sqrt());
 let xi=leja_points(); println!("SUFFIX prefix_bound={} actual_suffix_at_2={} nodes={:?}",leja_node_bounds()[1],(2.-xi[1])*(2.-xi[2]),&xi[..3]);
 let panic=std::panic::catch_unwind(|| SparseDirectSolver::try_new_symbolic(2,&[0,1,2],&[0,2]).is_err()); println!("SPARSE_INVALID_ROW {:?}",panic);
 let panic=std::panic::catch_unwind(|| {let mut s=SparseDirectSolver::try_new_symbolic(2,&[0,1,2],&[0,1]).unwrap();s.solve_in_place(&mut [1.,1.],&mut WorkCounters::default()).is_err()}); println!("SPARSE_BEFORE_FACTOR {:?}",panic);
 let mut s=SparseDirectSolver::try_new_symbolic(2,&[0,1,2],&[0,1]).unwrap();
 let altered=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.refactorize(&[0,1,2],&[1,0],&[1.,1.],&mut WorkCounters::default()))); println!("SPARSE_ALTERED_PATTERN {:?}",altered);
}
