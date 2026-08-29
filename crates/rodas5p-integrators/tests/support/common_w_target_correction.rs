//! Test-only candidate: solve J_R(K) z = r for R=lhs-rhs; Newton update is K-z.
//! J_R has W on the diagonal and -h(alpha_ij J_i+Gamma_ij J_n) below it.
//! No full target/stage Jacobian assembly. Explicit common W is still required.
//! A linearized correction is not a rigorous nonlinear error certificate.
use rodas5p_core::{CoreError, CoreResult, LinearOperator, LuFactorization, WorkCounters, apply_jvp_counted};
use rodas5p_integrators::StepContext;
pub fn common_w_target_correction(context: &StepContext<'_>, states: &[Vec<f64>], rhs: &[Vec<f64>], work: &mut WorkCounters) -> CoreResult<Vec<Vec<f64>>> {
    let n=context.problem.dimension; let s=context.coeffs.stages();
    if n==0 || states.len()!=s || rhs.len()!=s || states.iter().chain(rhs).any(|v| v.len()!=n) {
        return Err(CoreError::Dimension("common-W correction shape".into()));
    }
    if states.iter().chain(rhs).flatten().any(|v| !v.is_finite()) {
        return Err(CoreError::NonFinite("common-W correction input".into()));
    }
    if !context.problem.supports_matrix_free_jvp() { return Err(CoreError::InvalidInput("analytic JVP required".into())); }
    let w=context.shifted.explicit().ok_or_else(|| CoreError::InvalidInput("explicit common W required".into()))?;
    work.direct_factorizations+=1;
    let factor=LuFactorization::new(w)?;
    let mut solution: Vec<Vec<f64>>=Vec::with_capacity(s);
    let mut p=vec![0.0;n];let mut q=vec![0.0;n];let mut image=vec![0.0;n];
    for i in 0..s {
        p.fill(0.0);q.fill(0.0);
        for (j,z) in solution.iter().enumerate() { for k in 0..n {
            p[k]+=context.coeffs.alpha[(i,j)]*z[k];
            q[k]+=context.coeffs.gamma_matrix[(i,j)]*z[k];
        }}
        let mut corrected=rhs[i].clone();
        if i>0 {
            let ji=context.problem.linearize_matrix_free(context.t+context.coeffs.c[i]*context.h,&states[i])?;
            apply_jvp_counted(ji.as_ref(),&p,&mut image,work)?;
            for k in 0..n {corrected[k]+=context.h*image[k];}
            apply_jvp_counted(context.jacobian.as_ref(),&q,&mut image,work)?;
            for k in 0..n {corrected[k]+=context.h*image[k];}
        }
        if corrected.iter().any(|v| !v.is_finite()) {return Err(CoreError::NonFinite("correction RHS".into()));}
        work.direct_solve_calls+=1;
        let row=factor.solve(&corrected)?;
        if row.iter().any(|v| !v.is_finite()) {return Err(CoreError::NonFinite("correction solution".into()));}
        solution.push(row);
    }
    Ok(solution)
}
