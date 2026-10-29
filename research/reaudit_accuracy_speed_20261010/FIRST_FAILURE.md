# Preserved first execution and fixture correction

The first native/checker execution at source
`a6c6ebfc8342dc7f8726ff0d6f76e031c1d55bca` produced `NATIVE_FIRST.json` and
`RESULTS_FIRST.json`: 30/30 positive enclosures, 11/12 expected rejections,
overall FAIL. No scientific threshold or positive/holdout cell was changed.

`invalid_scale_gain_overflow` supplied W=I, b=x=(1,1), scales=(2^-1074,1).
These are valid inputs under the preregistered positive-finite-scale contract.
The scaled inverse is still I, the exact error is zero, and the interval
calculation stayed finite. Its very loose bound is not a false enclosure.
Requiring rejection was a fixture design error, not a certificate defect.

The corrected negative control uses W=[[1,1],[0,1]] with the same vectors and
scales. Now the off-diagonal scaled inverse norm contains 2^1074, exceeding the
binary64 finite range, so it exercises the intended overflow rejection. The
certificate implementation and gate are unchanged. The original admitted row
remains in the first raw file; the correction is visible in git history. The
second run is limited to the tiny registered example/checker, not old research
campaigns. The separate checker-mutation study already completed and is not
repeated.
