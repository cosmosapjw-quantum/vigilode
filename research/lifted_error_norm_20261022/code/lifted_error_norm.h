#ifndef LIFTED_ERROR_NORM_H
#define LIFTED_ERROR_NORM_H
/* Version 1: fixed BASS compact30 -> compact33 ordering, IEEE binary64.
 * err30 is an error/difference vector, NEVER an affine state.
 * scale33[i] is the original positive full-state error scale at that step.
 * 0 success; 1 invalid input; 2 unrepresentable normalized norm;
 * 3 unsupported long-double exponent/precision range.
 * On failure *norm is unchanged. No allocation and no mutable global state.
 */
int bassrei_full33_error_norm(const double err30[30],
                            const double scale33[33], double *norm);
#endif
