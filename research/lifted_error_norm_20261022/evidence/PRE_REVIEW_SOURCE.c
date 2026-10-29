#include "lifted_error_norm.h"
#include <math.h>
#include <stddef.h>

/* Sum <=3 errors before division, including cancellation at extreme scales.
 * frexp/scalbn avoid squaring large values or forming max_error/scale first.
 */
static int reduced_sum_ratio(const double *e, size_t n, double scale, double *r) {
    double m=0., s=0., c=0.;
    for (size_t i=0;i<n;i++) m=fmax(m,fabs(e[i]));
    if (m==0.) { *r=0.; return 0; }
    for (size_t i=0;i<n;i++) {
        double x=e[i]/m, t=s+x;
        c += fabs(s)>=fabs(x) ? (s-t)+x : (x-t)+s;
        s=t;
    }
    s += c;
    if (s==0.) { *r=0.; return 0; }
    int em, ed;
    double mm=frexp(m,&em), md=frexp(scale,&ed);
    *r=scalbn(s*mm/md,em-ed);
    return isfinite(*r) ? 0 : 2;
}

int bassrei_full33_error_norm(const double err30[30],
                            const double scale33[33], double *norm) {
    if (!err30 || !scale33 || !norm) return 1;
    for (size_t i=0;i<30;i++) if (!isfinite(err30[i])) return 1;
    for (size_t i=0;i<33;i++)
        if (!isfinite(scale33[i]) || scale33[i]<=0.) return 1;
    double length=0.;
    size_t j=0;
    for (size_t i=0;i<33;i++) {
        double r;
        if (i==23 || i==25 || i==28) {
            size_t n=i==23 ? 1 : (i==25 ? 2 : 3);
            int status=reduced_sum_ratio(err30+j,n,scale33[i],&r);
            if (status) return status;
        } else {
            r=err30[j++]/scale33[i];
            if (!isfinite(r)) return 2;
        }
        length=hypot(length,r/sqrt(33.));
        if (!isfinite(length)) return 2;
    }
    *norm=length;
    return 0;
}
