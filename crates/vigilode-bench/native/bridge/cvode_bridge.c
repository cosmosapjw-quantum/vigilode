/* C bridge from vigilode-bench to SUNDIALS CVODE 6.4.1 (system package,
 * headers in /usr/include). BDF with the dense or band direct linear solver,
 * analytic Jacobian from the shared Rust model, scalar tolerances, every
 * other option at its CVODE default. Grid outputs through CVodeGetDky after
 * each internal step (CV_ONE_STEP), so the step sequence is CVODE's own. */
#include <cvode/cvode.h>
#include <nvector/nvector_serial.h>
#include <sundials/sundials_context.h>
#include <sunlinsol/sunlinsol_band.h>
#include <sunlinsol/sunlinsol_dense.h>
#include <sunmatrix/sunmatrix_band.h>
#include <sunmatrix/sunmatrix_dense.h>
#include <math.h>
#include <string.h>
#include <sundials/sundials_config.h>

/* The SUNDIALS version of the headers this bridge was compiled against. */
const char *bench_sundials_version(void) { return SUNDIALS_VERSION; }

typedef void (*bench_rhs_cb)(const void *ctx, double t, const double *y, double *f);
typedef void (*bench_jac_cb)(const void *ctx, double t, const double *y, int ml, int mu,
                             int ld, double *pd);

typedef struct {
  const void *ctx;
  bench_rhs_cb rhs;
  bench_jac_cb jac;
  int ml, mu;
} bench_user;

static int bench_f(realtype t, N_Vector y, N_Vector ydot, void *ud) {
  bench_user *u = (bench_user *)ud;
  u->rhs(u->ctx, t, N_VGetArrayPointer(y), N_VGetArrayPointer(ydot));
  return 0;
}

static int bench_jac_dense(realtype t, N_Vector y, N_Vector fy, SUNMatrix J, void *ud,
                           N_Vector t1, N_Vector t2, N_Vector t3) {
  bench_user *u = (bench_user *)ud;
  (void)fy; (void)t1; (void)t2; (void)t3;
  /* Column-major, leading dimension = number of rows. */
  u->jac(u->ctx, t, N_VGetArrayPointer(y), -1, -1, (int)SUNDenseMatrix_Rows(J),
         SUNDenseMatrix_Data(J));
  return 0;
}

/* Band storage: element (i, j) at data[j * ldim + smu + i - j]; the callback
 * writes (mu + i - j) + ld * j from a pointer shifted by smu - mu. */
static int bench_jac_band(realtype t, N_Vector y, N_Vector fy, SUNMatrix J, void *ud,
                          N_Vector t1, N_Vector t2, N_Vector t3) {
  bench_user *u = (bench_user *)ud;
  (void)fy; (void)t1; (void)t2; (void)t3;
  sunindextype smu = SUNBandMatrix_StoredUpperBandwidth(J);
  sunindextype ldim = SUNBandMatrix_LDim(J);
  u->jac(u->ctx, t, N_VGetArrayPointer(y), u->ml, u->mu, (int)ldim,
         SUNBandMatrix_Data(J) + (smu - u->mu));
  return 0;
}

/* Returns 0 on a completed grid; the CVODE flag of the first failure
 * otherwise (or -1000 - k for a setup failure at stage k).
 * stats: nst, nfe, nsetups, netf, nni, ncfn, nje, nfeLS, completed outputs. */
int bench_cvode(const void *ctx, bench_rhs_cb rhs, bench_jac_cb jac, int n, int nt,
                const double *times, const double *y0, double rtol, double atol, int ml,
                int mu, long max_steps, double *values, int *completed, long *stats) {
  SUNContext sctx = NULL;
  void *mem = NULL;
  N_Vector y = NULL, ytmp = NULL;
  SUNMatrix A = NULL;
  SUNLinearSolver LS = NULL;
  bench_user u = {ctx, rhs, jac, ml, mu};
  int flag = 0, next = 1;
  long steps = 0;
  realtype tret = times[0];
  for (int k = 0; k < n * nt; ++k) values[k] = NAN;
  for (int k = 0; k < 9; ++k) stats[k] = 0;
  *completed = 0;
  if (SUNContext_Create(NULL, &sctx)) return -1001;
  y = N_VNew_Serial(n, sctx);
  ytmp = N_VNew_Serial(n, sctx);
  if (!y || !ytmp) { flag = -1002; goto done; }
  memcpy(N_VGetArrayPointer(y), y0, sizeof(double) * n);
  memcpy(values, y0, sizeof(double) * n);
  *completed = 1;
  mem = CVodeCreate(CV_BDF, sctx);
  if (!mem) { flag = -1003; goto done; }
  if (CVodeInit(mem, bench_f, times[0], y)) { flag = -1004; goto done; }
  if (CVodeSStolerances(mem, rtol, atol)) { flag = -1005; goto done; }
  if (CVodeSetUserData(mem, &u)) { flag = -1006; goto done; }
  if (ml < 0) {
    A = SUNDenseMatrix(n, n, sctx);
    LS = A ? SUNLinSol_Dense(y, A, sctx) : NULL;
  } else {
    A = SUNBandMatrix(n, mu, ml, sctx);
    LS = A ? SUNLinSol_Band(y, A, sctx) : NULL;
  }
  if (!A || !LS) { flag = -1007; goto done; }
  if (CVodeSetLinearSolver(mem, LS, A)) { flag = -1008; goto done; }
  if (CVodeSetJacFn(mem, ml < 0 ? bench_jac_dense : bench_jac_band)) { flag = -1009; goto done; }
  while (next < nt) {
    if (steps >= max_steps) { flag = CV_TOO_MUCH_WORK; break; }
    flag = CVode(mem, times[nt - 1], y, &tret, CV_ONE_STEP);
    if (flag < 0) break;
    ++steps;
    while (next < nt && times[next] <= tret) {
      if (times[next] == tret) {
        memcpy(values + (size_t)next * n, N_VGetArrayPointer(y), sizeof(double) * n);
      } else {
        int g = CVodeGetDky(mem, times[next], 0, ytmp);
        if (g) { flag = g; goto stats; }
        memcpy(values + (size_t)next * n, N_VGetArrayPointer(ytmp), sizeof(double) * n);
      }
      ++next;
      *completed = next;
    }
  }
  if (next == nt) flag = 0;
stats:
  CVodeGetNumSteps(mem, &stats[0]);
  CVodeGetNumRhsEvals(mem, &stats[1]);
  CVodeGetNumLinSolvSetups(mem, &stats[2]);
  CVodeGetNumErrTestFails(mem, &stats[3]);
  CVodeGetNumNonlinSolvIters(mem, &stats[4]);
  CVodeGetNumNonlinSolvConvFails(mem, &stats[5]);
  CVodeGetNumJacEvals(mem, &stats[6]);
  CVodeGetNumLinRhsEvals(mem, &stats[7]);
  stats[8] = next;
done:
  if (LS) SUNLinSolFree(LS);
  if (A) SUNMatDestroy(A);
  if (mem) CVodeFree(&mem);
  if (ytmp) N_VDestroy(ytmp);
  if (y) N_VDestroy(y);
  if (sctx) SUNContext_Free(&sctx);
  return flag;
}
