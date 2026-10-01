/*
 * Native comparator driver of the stiff benchmark (research node
 * research/stiff_native_benchmark_20261001). Built by build.sh.
 *
 *   native_stiff run <problem> <arm> <rtol> <repetitions> <warmups>
 *   native_stiff parity <problem>        (states on stdin, one per line)
 *   native_stiff lu-bench <cells> <repetitions>
 *
 * Arms: cvode-bdf (SUNDIALS dense LU), cvode-bdf-lapack (OpenBLAS
 * dgetrf/dgetrs through a custom SUNLinearSolver), hairer-radau5 and
 * hairer-rodas (Hairer's Fortran codes with DECSOL). Every arm uses the
 * analytic dense Jacobian. Output is one JSON object on stdout.
 */
#define _POSIX_C_SOURCE 199309L
#define PI 3.14159265358979323846
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include <cvode/cvode.h>
#include <nvector/nvector_serial.h>
#include <sunlinsol/sunlinsol_dense.h>
#include <sunmatrix/sunmatrix_dense.h>

/* ---------------------------------------------------------------- problems */

typedef struct {
    const char *id;
    int n;
    double t0, tf, atol_scale;
    int cells; /* Brusselator only */
    void (*rhs)(const double *y, double *f);
    /* jac[i + j*ld] = d f_i / d y_j; the matrix is zeroed by the caller. */
    void (*jac)(const double *y, double *jac, int ld);
    double y0[400];
} Problem;

static const Problem *P; /* the problem being integrated (single-threaded) */

static void robertson_rhs(const double *y, double *f) {
    f[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    f[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    f[2] = 3.0e7 * y[1] * y[1];
}
static void robertson_jac(const double *y, double *J, int ld) {
#define A(i, j) J[(i) + (j) * ld]
    A(0, 0) = -0.04; A(0, 1) = 1.0e4 * y[2]; A(0, 2) = 1.0e4 * y[1];
    A(1, 0) = 0.04; A(1, 1) = -1.0e4 * y[2] - 6.0e7 * y[1]; A(1, 2) = -1.0e4 * y[1];
    A(2, 1) = 6.0e7 * y[1];
}

static void hires_rhs(const double *y, double *f) {
    f[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
    f[1] = 1.71 * y[0] - 8.75 * y[1];
    f[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
    f[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
    f[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
    f[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
    f[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
    f[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
}
static void hires_jac(const double *y, double *J, int ld) {
    A(0, 0) = -1.71; A(0, 1) = 0.43; A(0, 2) = 8.32;
    A(1, 0) = 1.71; A(1, 1) = -8.75;
    A(2, 2) = -10.03; A(2, 3) = 0.43; A(2, 4) = 0.035;
    A(3, 1) = 8.32; A(3, 2) = 1.71; A(3, 3) = -1.12;
    A(4, 4) = -1.745; A(4, 5) = 0.43; A(4, 6) = 0.43;
    A(5, 3) = 0.69; A(5, 4) = 1.71; A(5, 5) = -280.0 * y[7] - 0.43; A(5, 6) = 0.69;
    A(5, 7) = -280.0 * y[5];
    A(6, 5) = 280.0 * y[7]; A(6, 6) = -1.81; A(6, 7) = 280.0 * y[5];
    A(7, 5) = -280.0 * y[7]; A(7, 6) = 1.81; A(7, 7) = -280.0 * y[5];
}

static const double MU = 1.0e3;
static void vdp_rhs(const double *y, double *f) {
    f[0] = y[1];
    f[1] = MU * (1.0 - y[0] * y[0]) * y[1] - y[0];
}
static void vdp_jac(const double *y, double *J, int ld) {
    A(0, 1) = 1.0;
    A(1, 0) = -2.0 * MU * y[0] * y[1] - 1.0;
    A(1, 1) = MU * (1.0 - y[0] * y[0]);
}

static void bruss_rhs(const double *y, double *f) {
    int cells = P->cells;
    double c = (cells + 1.0) * (cells + 1.0) / 50.0;
    for (int i = 0; i < cells; i++) {
        double u = y[2 * i], v = y[2 * i + 1];
        double ul = i == 0 ? 1.0 : y[2 * i - 2], vl = i == 0 ? 3.0 : y[2 * i - 1];
        double ur = i + 1 == cells ? 1.0 : y[2 * i + 2], vr = i + 1 == cells ? 3.0 : y[2 * i + 3];
        f[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
        f[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
    }
}
static void bruss_jac(const double *y, double *J, int ld) {
    int cells = P->cells;
    double c = (cells + 1.0) * (cells + 1.0) / 50.0;
    for (int i = 0; i < cells; i++) {
        double u = y[2 * i], v = y[2 * i + 1];
        int a = 2 * i, b = 2 * i + 1;
        A(a, a) = 2.0 * u * v - 4.0 - 2.0 * c;
        A(a, b) = u * u;
        A(b, a) = 3.0 - 2.0 * u * v;
        A(b, b) = -u * u - 2.0 * c;
        if (i > 0) { A(a, a - 2) = c; A(b, b - 2) = c; }
        if (i + 1 < cells) { A(a, a + 2) = c; A(b, b + 2) = c; }
    }
#undef A
}

static Problem make_problem(const char *id) {
    Problem p;
    memset(&p, 0, sizeof p);
    p.id = id;
    if (!strcmp(id, "robertson")) {
        p.n = 3; p.tf = 40.0; p.atol_scale = 1.0e-4;
        p.rhs = robertson_rhs; p.jac = robertson_jac;
        p.y0[0] = 1.0;
    } else if (!strcmp(id, "hires")) {
        p.n = 8; p.tf = 321.8122; p.atol_scale = 1.0e-4;
        p.rhs = hires_rhs; p.jac = hires_jac;
        p.y0[0] = 1.0; p.y0[7] = 0.0057;
    } else if (!strcmp(id, "van-der-pol-mu1000")) {
        p.n = 2; p.tf = 2000.0; p.atol_scale = 1.0;
        p.rhs = vdp_rhs; p.jac = vdp_jac;
        p.y0[0] = 2.0;
    } else if (!strcmp(id, "brusselator-1d-50") || !strcmp(id, "brusselator-1d-200")) {
        p.cells = !strcmp(id, "brusselator-1d-50") ? 50 : 200;
        p.n = 2 * p.cells; p.tf = 10.0; p.atol_scale = 1.0;
        p.rhs = bruss_rhs; p.jac = bruss_jac;
        for (int i = 0; i < p.cells; i++) {
            double x = (i + 1.0) / (p.cells + 1.0);
            p.y0[2 * i] = 1.0 + sin(2.0 * PI * x);
            p.y0[2 * i + 1] = 3.0;
        }
    } else {
        fprintf(stderr, "unknown problem %s\n", id);
        exit(2);
    }
    return p;
}

/* ------------------------------------------------------------ run results */

typedef struct {
    int ok;
    char message[160];
    double y[400];
    long steps, rejected, nfev, njev, nlu;
} RunResult;

static double now(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + 1e-9 * ts.tv_nsec;
}

/* ------------------------------------------------- LAPACK SUNLinearSolver */

extern void dgetrf_(const int *m, const int *n, double *a, const int *lda, int *ipiv, int *info);
extern void dgetrs_(const char *trans, const int *n, const int *nrhs, const double *a,
                    const int *lda, const int *ipiv, double *b, const int *ldb, int *info,
                    size_t trans_len);

typedef struct {
    int n;
    double *lu;
    int *ipiv;
} LapackContent;

static SUNLinearSolver_Type lapack_gettype(SUNLinearSolver S) {
    (void)S;
    return SUNLINEARSOLVER_DIRECT;
}
static SUNLinearSolver_ID lapack_getid(SUNLinearSolver S) {
    (void)S;
    return SUNLINEARSOLVER_CUSTOM;
}
static int lapack_setup(SUNLinearSolver S, SUNMatrix A) {
    LapackContent *c = S->content;
    memcpy(c->lu, SUNDenseMatrix_Data(A), sizeof(double) * c->n * c->n);
    int info;
    dgetrf_(&c->n, &c->n, c->lu, &c->n, c->ipiv, &info);
    return info == 0 ? SUNLS_SUCCESS : (info > 0 ? SUNLS_LUFACT_FAIL : SUNLS_PACKAGE_FAIL_UNREC);
}
static int lapack_solve(SUNLinearSolver S, SUNMatrix A, N_Vector x, N_Vector b, realtype tol) {
    (void)A;
    (void)tol;
    LapackContent *c = S->content;
    N_VScale(1.0, b, x);
    int one = 1, info;
    dgetrs_("N", &c->n, &one, c->lu, &c->n, c->ipiv, N_VGetArrayPointer(x), &c->n, &info, 1);
    return info == 0 ? SUNLS_SUCCESS : SUNLS_PACKAGE_FAIL_UNREC;
}
static int lapack_free(SUNLinearSolver S) {
    if (S == NULL) return SUNLS_SUCCESS;
    LapackContent *c = S->content;
    if (c) {
        free(c->lu);
        free(c->ipiv);
        free(c);
    }
    S->content = NULL;
    SUNLinSolFreeEmpty(S);
    return SUNLS_SUCCESS;
}
static SUNLinearSolver lapack_linsol(int n, SUNContext ctx) {
    SUNLinearSolver S = SUNLinSolNewEmpty(ctx);
    S->ops->gettype = lapack_gettype;
    S->ops->getid = lapack_getid;
    S->ops->setup = lapack_setup;
    S->ops->solve = lapack_solve;
    S->ops->free = lapack_free;
    LapackContent *c = malloc(sizeof *c);
    c->n = n;
    c->lu = malloc(sizeof(double) * n * n);
    c->ipiv = malloc(sizeof(int) * n);
    S->content = c;
    return S;
}

/* ------------------------------------------------------------------ CVODE */

static int cv_rhs(realtype t, N_Vector y, N_Vector f, void *data) {
    (void)t;
    (void)data;
    P->rhs(N_VGetArrayPointer(y), N_VGetArrayPointer(f));
    return 0;
}
static int cv_jac(realtype t, N_Vector y, N_Vector fy, SUNMatrix J, void *data, N_Vector t1,
                  N_Vector t2, N_Vector t3) {
    (void)t; (void)fy; (void)data; (void)t1; (void)t2; (void)t3;
    double *d = SUNDenseMatrix_Data(J);
    memset(d, 0, sizeof(double) * P->n * P->n);
    P->jac(N_VGetArrayPointer(y), d, P->n);
    return 0;
}

static RunResult run_cvode(double rtol, int lapack) {
    RunResult r;
    memset(&r, 0, sizeof r);
    SUNContext ctx;
    SUNContext_Create(NULL, &ctx);
    N_Vector y = N_VNew_Serial(P->n, ctx);
    memcpy(N_VGetArrayPointer(y), P->y0, sizeof(double) * P->n);
    SUNMatrix A = SUNDenseMatrix(P->n, P->n, ctx);
    SUNLinearSolver LS = lapack ? lapack_linsol(P->n, ctx) : SUNLinSol_Dense(y, A, ctx);
    void *mem = CVodeCreate(CV_BDF, ctx);
    CVodeInit(mem, cv_rhs, P->t0, y);
    CVodeSStolerances(mem, rtol, rtol * P->atol_scale);
    CVodeSetLinearSolver(mem, LS, A);
    CVodeSetJacFn(mem, cv_jac);
    CVodeSetMaxNumSteps(mem, 1000000);
    CVodeSetStopTime(mem, P->tf);
    realtype t = P->t0;
    int flag = CVode(mem, P->tf, y, &t, CV_NORMAL);
    r.ok = flag >= 0 && t == P->tf;
    snprintf(r.message, sizeof r.message, "CVode flag %d at t = %.17g", flag, t);
    memcpy(r.y, N_VGetArrayPointer(y), sizeof(double) * P->n);
    long etf = 0;
    CVodeGetNumSteps(mem, &r.steps);
    CVodeGetNumErrTestFails(mem, &etf);
    r.rejected = etf;
    CVodeGetNumRhsEvals(mem, &r.nfev);
    CVodeGetNumJacEvals(mem, &r.njev);
    CVodeGetNumLinSolvSetups(mem, &r.nlu);
    CVodeFree(&mem);
    SUNLinSolFree(LS);
    SUNMatDestroy(A);
    N_VDestroy(y);
    SUNContext_Free(&ctx);
    return r;
}

/* ------------------------------------------------------------ Hairer codes */

typedef void (*fcn_t)(int *, double *, double *, double *, double *, int *);
typedef void (*jac_t)(int *, double *, double *, double *, int *, double *, int *);
extern void radau5_(int *n, fcn_t fcn, double *x, double *y, double *xend, double *h,
                    double *rtol, double *atol, int *itol, jac_t jac, int *ijac, int *mljac,
                    int *mujac, void *mas, int *imas, int *mlmas, int *mumas, void *solout,
                    int *iout, double *work, int *lwork, int *iwork, int *liwork, double *rpar,
                    int *ipar, int *idid);
extern void rodas_(int *n, fcn_t fcn, int *ifcn, double *x, double *y, double *xend, double *h,
                   double *rtol, double *atol, int *itol, jac_t jac, int *ijac, int *mljac,
                   int *mujac, void *dfx, int *idfx, void *mas, int *imas, int *mlmas,
                   int *mumas, void *solout, int *iout, double *work, int *lwork, int *iwork,
                   int *liwork, double *rpar, int *ipar, int *idid);
extern void dec_(int *n, int *ndim, double *a, int *ip, int *ier);

static void h_fcn(int *n, double *x, double *y, double *f, double *rpar, int *ipar) {
    (void)n; (void)x; (void)rpar; (void)ipar;
    P->rhs(y, f);
}
static void h_jac(int *n, double *x, double *y, double *dfy, int *ldfy, double *rpar, int *ipar) {
    (void)x; (void)rpar; (void)ipar;
    memset(dfy, 0, sizeof(double) * (size_t)(*ldfy) * (size_t)(*n));
    P->jac(y, dfy, *ldfy);
}
static void h_dummy(void) {}

static RunResult run_hairer(double rtol, int radau) {
    RunResult r;
    memset(&r, 0, sizeof r);
    int n = P->n, itol = 0, ijac = 1, mljac = n, mujac = 0, imas = 0, mlmas = 0, mumas = 0;
    int iout = 0, idid = 0, ifcn = 0, idfx = 0;
    int lwork = radau ? n * (4 * n + 12) + 20 : n * (2 * n + 14) + 20;
    int liwork = radau ? 3 * n + 20 : n + 20;
    double *work = calloc(lwork, sizeof(double));
    int *iwork = calloc(liwork, sizeof(int));
    /* Maximal number of steps: IWORK(2) in RADAU5, IWORK(1) in RODAS. */
    iwork[radau ? 1 : 0] = 1000000;
    double x = P->t0, xend = P->tf, h = 1.0e-6, atol = rtol * P->atol_scale, rpar = 0.0;
    int ipar = 0;
    memcpy(r.y, P->y0, sizeof(double) * n);
    if (radau)
        radau5_(&n, h_fcn, &x, r.y, &xend, &h, &rtol, &atol, &itol, h_jac, &ijac, &mljac, &mujac,
                (void *)h_dummy, &imas, &mlmas, &mumas, (void *)h_dummy, &iout, work, &lwork,
                iwork, &liwork, &rpar, &ipar, &idid);
    else
        rodas_(&n, h_fcn, &ifcn, &x, r.y, &xend, &h, &rtol, &atol, &itol, h_jac, &ijac, &mljac,
               &mujac, (void *)h_dummy, &idfx, (void *)h_dummy, &imas, &mlmas, &mumas,
               (void *)h_dummy, &iout, work, &lwork, iwork, &liwork, &rpar, &ipar, &idid);
    r.ok = idid == 1 && x == xend;
    snprintf(r.message, sizeof r.message, "IDID %d at x = %.17g", idid, x);
    r.nfev = iwork[13];
    r.njev = iwork[14];
    r.steps = iwork[16];   /* NACCPT */
    r.rejected = iwork[17];
    r.nlu = iwork[18];     /* NDEC */
    free(work);
    free(iwork);
    return r;
}

/* ---------------------------------------------------------------- drivers */

static int cmp_double(const void *a, const void *b) {
    double x = *(const double *)a, y = *(const double *)b;
    return (x > y) - (x < y);
}

static RunResult run_arm(const char *arm, double rtol) {
    if (!strcmp(arm, "cvode-bdf")) return run_cvode(rtol, 0);
    if (!strcmp(arm, "cvode-bdf-lapack")) return run_cvode(rtol, 1);
    if (!strcmp(arm, "hairer-radau5")) return run_hairer(rtol, 1);
    if (!strcmp(arm, "hairer-rodas")) return run_hairer(rtol, 0);
    fprintf(stderr, "unknown arm %s\n", arm);
    exit(2);
}

static void print_array(const double *a, int n) {
    putchar('[');
    for (int i = 0; i < n; i++) printf("%s%.17g", i ? "," : "", a[i]);
    putchar(']');
}

static int cmd_run(const char *id, const char *arm, double rtol, int reps, int warmups) {
    Problem p = make_problem(id);
    P = &p;
    for (int w = 0; w < warmups; w++) run_arm(arm, rtol);
    double *walls = malloc(sizeof(double) * reps);
    RunResult first;
    int deterministic = 1;
    for (int k = 0; k < reps; k++) {
        double t = now();
        RunResult r = run_arm(arm, rtol);
        walls[k] = now() - t;
        if (k == 0)
            first = r;
        else
            deterministic &= r.ok == first.ok && !memcmp(r.y, first.y, sizeof(double) * p.n) &&
                             r.nfev == first.nfev && r.nlu == first.nlu;
    }
    double *sorted = malloc(sizeof(double) * reps);
    memcpy(sorted, walls, sizeof(double) * reps);
    qsort(sorted, reps, sizeof(double), cmp_double);
    printf("{\"problem\":\"%s\",\"arm\":\"%s\",\"rtol\":%.17g,\"atol\":%.17g,", id, arm, rtol,
           rtol * p.atol_scale);
    printf("\"status\":\"%s\",\"message\":\"%s\",", first.ok ? "completed" : "failed",
           first.message);
    if (first.ok) {
        printf("\"final_state\":");
        print_array(first.y, p.n);
        printf(",");
    }
    printf("\"accepted_steps\":%ld,\"rejected_steps\":%ld,", first.steps, first.rejected);
    printf("\"counters\":{\"rhs_evaluations\":%ld,\"jacobian_builds\":%ld,"
           "\"direct_factorizations\":%ld},",
           first.nfev, first.njev, first.nlu);
    printf("\"wall_seconds\":");
    print_array(walls, reps);
    printf(",\"wall_median\":%.17g,\"deterministic\":%s}\n", sorted[reps / 2],
           deterministic ? "true" : "false");
    return 0;
}

static int cmd_parity(const char *id) {
    Problem p = make_problem(id);
    P = &p;
    double y[400], f[400];
    double *J = malloc(sizeof(double) * p.n * p.n);
    printf("[");
    int first = 1;
    while (1) {
        int i;
        for (i = 0; i < p.n; i++)
            if (scanf("%lf", &y[i]) != 1) break;
        if (i < p.n) break;
        p.rhs(y, f);
        memset(J, 0, sizeof(double) * p.n * p.n);
        p.jac(y, J, p.n);
        printf("%s{\"f\":", first ? "" : ",");
        print_array(f, p.n);
        printf(",\"jacobian\":[");
        for (int r = 0; r < p.n; r++) {
            printf("%s[", r ? "," : "");
            for (int c = 0; c < p.n; c++) printf("%s%.17g", c ? "," : "", J[r + c * p.n]);
            printf("]");
        }
        printf("]}");
        first = 0;
    }
    printf("]\n");
    free(J);
    return 0;
}

/* Median seconds of one LU, by implementation, of an n x n matrix: the
 * Brusselator iteration matrix I - 0.05 J(y0) ("banded": DECSOL skips zero
 * multipliers there) and the full matrix 1/(1 + |i - j|) + n delta_ij. */
static void lu_bench_matrix(const char *label, const double *m, int n, int reps, int last) {
    double *a = malloc(sizeof(double) * n * n);
    int *ip = malloc(sizeof(int) * n);
    double *dense = malloc(sizeof(double) * reps), *lapack = malloc(sizeof(double) * reps),
           *decsol = malloc(sizeof(double) * reps);
    SUNContext ctx;
    SUNContext_Create(NULL, &ctx);
    N_Vector y = N_VNew_Serial(n, ctx);
    SUNMatrix A = SUNDenseMatrix(n, n, ctx);
    SUNLinearSolver LS = SUNLinSol_Dense(y, A, ctx);
    SUNLinSolInitialize(LS);
    for (int k = 0; k < reps; k++) {
        memcpy(SUNDenseMatrix_Data(A), m, sizeof(double) * n * n);
        double t = now();
        SUNLinSolSetup(LS, A);
        dense[k] = now() - t;
        memcpy(a, m, sizeof(double) * n * n);
        int info;
        t = now();
        dgetrf_(&n, &n, a, &n, ip, &info);
        lapack[k] = now() - t;
        memcpy(a, m, sizeof(double) * n * n);
        t = now();
        dec_(&n, &n, a, ip, &info);
        decsol[k] = now() - t;
    }
    qsort(dense, reps, sizeof(double), cmp_double);
    qsort(lapack, reps, sizeof(double), cmp_double);
    qsort(decsol, reps, sizeof(double), cmp_double);
    printf("\"%s\":{\"sundials-dense\":%.6g,\"openblas-dgetrf\":%.6g,\"decsol-dec\":%.6g}%s",
           label, dense[reps / 2], lapack[reps / 2], decsol[reps / 2], last ? "" : ",");
    SUNLinSolFree(LS);
    SUNMatDestroy(A);
    N_VDestroy(y);
    SUNContext_Free(&ctx);
    free(a); free(ip); free(dense); free(lapack); free(decsol);
}

static int cmd_lu_bench(int cells, int reps) {
    char id[32];
    snprintf(id, sizeof id, "brusselator-1d-%d", cells);
    Problem p = make_problem(id);
    P = &p;
    int n = p.n;
    double *m = calloc((size_t)n * n, sizeof(double));
    p.jac(p.y0, m, n);
    for (int k = 0; k < n * n; k++) m[k] *= -0.05;
    for (int i = 0; i < n; i++) m[i + i * n] += 1.0;
    double *full = malloc(sizeof(double) * n * n);
    for (int i = 0; i < n; i++)
        for (int j = 0; j < n; j++)
            full[i + j * n] = 1.0 / (1.0 + abs(i - j)) + (i == j ? n : 0.0);
    printf("{\"n\":%d,", n);
    lu_bench_matrix("banded", m, n, reps, 0);
    lu_bench_matrix("full", full, n, reps, 1);
    printf("}\n");
    free(m);
    free(full);
    return 0;
}

int main(int argc, char **argv) {
    if (argc == 7 && !strcmp(argv[1], "run"))
        return cmd_run(argv[2], argv[3], atof(argv[4]), atoi(argv[5]), atoi(argv[6]));
    if (argc == 3 && !strcmp(argv[1], "parity")) return cmd_parity(argv[2]);
    if (argc == 4 && !strcmp(argv[1], "lu-bench")) return cmd_lu_bench(atoi(argv[2]), atoi(argv[3]));
    fprintf(stderr, "usage: native_stiff run <problem> <arm> <rtol> <reps> <warmups> | parity <problem> | lu-bench <cells> <reps>\n");
    return 2;
}
