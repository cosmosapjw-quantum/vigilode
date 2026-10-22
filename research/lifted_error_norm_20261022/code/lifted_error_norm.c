#include "lifted_error_norm.h"
#include <float.h>
#include <math.h>
#include <stddef.h>

int bassrei_full33_error_norm(const double err30[30],
                            const double scale33[33], double *norm) {
    if (!err30 || !scale33 || !norm) return 1;
    for (size_t i=0;i<30;i++) if (!isfinite(err30[i])) return 1;
    for (size_t i=0;i<33;i++)
        if (!isfinite(scale33[i]) || scale33[i]<=0.) return 1;
/* Require exponent range for every finite binary64 ratio, and precision
 * representing each input exactly. Refuse binary64-only long double.
 */
#if LDBL_MAX_EXP < 2*DBL_MAX_EXP+4 || LDBL_MIN_EXP > 2*DBL_MIN_EXP-DBL_MANT_DIG-4 || LDBL_MANT_DIG < DBL_MANT_DIG
    return 3;
#else
    long double length=0.L, divisor=sqrtl(33.L);
    size_t j=0;
    for (size_t i=0;i<33;i++) {
        long double e;
        if (i==23 || i==25 || i==28) {
            size_t n=i==23 ? 1 : (i==25 ? 2 : 3);
            long double s=0.L, c=0.L;
            for (size_t k=0;k<n;k++) {
                long double x=(long double)err30[j+k], t=s+x;
                c += fabsl(s)>=fabsl(x) ? (s-t)+x : (x-t)+s;
                s=t;
            }
            e=-(s+c);
        } else {
            e=(long double)err30[j++];
        }
        /* A ratio may exceed DBL_MAX while the normalized RMS is finite. */
        long double r=(e/(long double)scale33[i])/divisor;
        length=hypotl(length,r);
    }
    if (!isfinite(length) || length>(long double)DBL_MAX) return 2;
    *norm=(double)length;
    return 0;
#endif
}
