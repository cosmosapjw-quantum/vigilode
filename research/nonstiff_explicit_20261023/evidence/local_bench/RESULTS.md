# Local nonstiff campaign results

Each error is re-scored from every saved state against an independent analytic expression. All output rows and failures remain in the raw archive. Native methods land on five observation times; VODE may interpolate its internal steps to the same requested observations.

| Case | n | Method | rtol | All runs pass | Max normalized error | Median seconds | RHS attempts |
|---|---:|---|---:|---|---:|---:|---:|
| decay | 8 | dopri5 | 1e-04 | True | 3.378932620173858e-06 | 6.4721e-05 | 121 |
| decay | 8 | rodas-dense | 1e-04 | True | 8.723620285172595e-08 | 0.000628417 | 120 |
| decay | 8 | vode | 1e-04 | True | 5.7878107570207005e-06 | 0.002466026009642519 | 76 |
| decay | 8 | dopri5 | 1e-06 | True | 2.6969699255113833e-08 | 7.0382e-05 | 241 |
| decay | 8 | rodas-dense | 1e-06 | True | 1.0381949674953539e-09 | 0.000860318 | 232 |
| decay | 8 | vode | 1e-06 | True | 1.2890401451456923e-07 | 0.001751837000483647 | 130 |
| decay | 8 | dopri5 | 1e-08 | True | 2.316318387096672e-10 | 9.8472e-05 | 541 |
| decay | 8 | rodas-dense | 1e-08 | True | 5.799957736307704e-12 | 0.001393218 | 504 |
| decay | 8 | vode | 1e-08 | True | 3.9845310245700816e-09 | 0.001648505000048317 | 244 |
| driven | 8 | dopri5 | 1e-04 | True | 5.9850030374009044e-05 | 8.7598e-05 | 103 |
| driven | 8 | rodas-dense | 1e-04 | True | 6.715194915907219e-06 | 0.00101823 | 111 |
| driven | 8 | vode | 1e-04 | True | 0.0005359802742809228 | 0.0015024980093585327 | 59 |
| driven | 8 | dopri5 | 1e-06 | True | 5.477927950137129e-07 | 7.9467e-05 | 211 |
| driven | 8 | rodas-dense | 1e-06 | True | 1.0627401647589352e-07 | 0.001195103 | 199 |
| driven | 8 | vode | 1e-06 | True | 8.840186125347316e-07 | 0.002771311003016308 | 86 |
| driven | 8 | dopri5 | 1e-08 | True | 4.800793096393363e-09 | 0.000237022 | 451 |
| driven | 8 | rodas-dense | 1e-08 | True | 1.3598135950587675e-09 | 0.000596987 | 430 |
| driven | 8 | vode | 1e-08 | True | 2.0937740874416022e-08 | 0.0030993049876997247 | 154 |
| logistic | 8 | dopri5 | 1e-04 | True | 3.755821674222837e-05 | 6.46e-05 | 79 |
| logistic | 8 | rodas-dense | 1e-04 | True | 2.642316120737842e-05 | 0.002616278 | 136 |
| logistic | 8 | vode | 1e-04 | True | 7.800887393527578e-05 | 0.0016612670005997643 | 49 |
| logistic | 8 | dopri5 | 1e-06 | True | 2.3347251065786168e-07 | 7.1511e-05 | 157 |
| logistic | 8 | rodas-dense | 1e-06 | True | 7.07736700245043e-07 | 0.000492453 | 232 |
| logistic | 8 | vode | 1e-06 | True | 1.3317991827133113e-06 | 0.002346471999771893 | 121 |
| logistic | 8 | dopri5 | 1e-08 | True | 2.5411852622170272e-09 | 0.000273767 | 289 |
| logistic | 8 | rodas-dense | 1e-08 | True | 6.9592281937147504e-09 | 0.000694171 | 504 |
| logistic | 8 | vode | 1e-08 | True | 1.518249836962582e-08 | 0.0014913289924152195 | 162 |
| harmonic | 8 | dopri5 | 1e-04 | True | 0.0006558597222903195 | 8.0909e-05 | 241 |
| harmonic | 8 | rodas-dense | 1e-04 | True | 7.234008992096363e-05 | 0.001129046 | 305 |
| harmonic | 8 | vode | 1e-04 | True | 0.0036559181520662287 | 0.0032826899987412617 | 283 |
| harmonic | 8 | dopri5 | 1e-06 | True | 4.196956043922562e-06 | 0.000150388 | 655 |
| harmonic | 8 | rodas-dense | 1e-06 | True | 2.712583215824793e-07 | 0.000744799 | 621 |
| harmonic | 8 | vode | 1e-06 | True | 1.723339906989363e-05 | 0.0038826590025564656 | 266 |
| harmonic | 8 | dopri5 | 1e-08 | True | 3.2842580766079266e-08 | 0.000221719 | 1639 |
| harmonic | 8 | rodas-dense | 1e-08 | True | 9.228313738174343e-10 | 0.000928979 | 1521 |
| harmonic | 8 | vode | 1e-08 | True | 1.5831788990894324e-07 | 0.005391279992181808 | 485 |
| rapid | 8 | dopri5 | 1e-04 | True | 0.005424295771994991 | 0.000693077 | 2431 |
| rapid | 8 | rodas-dense | 1e-04 | True | 0.000702620576694013 | 0.000984751 | 2322 |
| rapid | 8 | vode | 1e-04 | True | 0.007450235046361442 | 0.019877744998666458 | 1669 |
| rapid | 8 | dopri5 | 1e-06 | True | 2.9982858601673712e-05 | 0.002229797 | 7369 |
| rapid | 8 | rodas-dense | 1e-06 | True | 2.1310258940809845e-06 | 0.002897722 | 6361 |
| rapid | 8 | vode | 1e-06 | True | 0.00016051079598167206 | 0.03176839300431311 | 2864 |
| rapid | 8 | dopri5 | 1e-08 | True | 2.1705652775150241e-07 | 0.002172435 | 19381 |
| rapid | 8 | rodas-dense | 1e-08 | True | 5.666393576575501e-09 | 0.005934655 | 17650 |
| rapid | 8 | vode | 1e-08 | True | 9.503549655009991e-07 | 0.039894715999253094 | 4821 |
| damped | 8 | dopri5 | 1e-04 | True | 1.6660520046042626e-05 | 0.000101741 | 445 |
| damped | 8 | rodas-dense | 1e-04 | True | 2.8993495750119502e-06 | 0.000711895 | 432 |
| damped | 8 | vode | 1e-04 | True | 7.358196292456959e-06 | 0.005146486990270205 | 455 |
| damped | 8 | dopri5 | 1e-06 | True | 1.7517415610776577e-07 | 0.000160323 | 1075 |
| damped | 8 | rodas-dense | 1e-06 | True | 9.31367593118293e-09 | 0.00083763 | 1015 |
| damped | 8 | vode | 1e-06 | True | 1.0500521253203887e-07 | 0.006487969003501348 | 550 |
| damped | 8 | dopri5 | 1e-08 | True | 1.5568823868994386e-09 | 0.000624448 | 2665 |
| damped | 8 | rodas-dense | 1e-08 | True | 3.9595174361473084e-11 | 0.001487705 | 2482 |
| damped | 8 | vode | 1e-08 | True | 8.546182990454287e-09 | 0.011998863992630504 | 818 |
| mixed-scales | 12 | dopri5 | 1e-04 | True | 1.3174826291253438e-05 | 7.6073e-05 | 139 |
| mixed-scales | 12 | rodas-dense | 1e-04 | True | 3.358779890039187e-07 | 0.000341854 | 128 |
| mixed-scales | 12 | vode | 1e-04 | True | 8.223203749384566e-06 | 0.0020486860012169927 | 94 |
| mixed-scales | 12 | dopri5 | 1e-06 | True | 1.1405594097624316e-07 | 0.000124707 | 277 |
| mixed-scales | 12 | rodas-dense | 1e-06 | True | 4.52024300656717e-09 | 0.000341784 | 256 |
| mixed-scales | 12 | vode | 1e-06 | True | 4.6325127621926755e-07 | 0.0018839889962691814 | 146 |
| mixed-scales | 12 | dopri5 | 1e-08 | True | 9.56808330343506e-10 | 0.000429431 | 631 |
| mixed-scales | 12 | rodas-dense | 1e-08 | True | 2.5365434197510693e-11 | 0.000930936 | 592 |
| mixed-scales | 12 | vode | 1e-08 | True | 1.504603478252875e-08 | 0.0027453280054032803 | 301 |
| nonnormal | 8 | dopri5 | 1e-04 | True | 1.5767601910701955e-05 | 0.000112982 | 121 |
| nonnormal | 8 | rodas-dense | 1e-04 | True | 4.5804208951672954e-07 | 0.000388819 | 120 |
| nonnormal | 8 | vode | 1e-04 | True | 3.136553133079568e-05 | 0.0018016550020547584 | 66 |
| nonnormal | 8 | dopri5 | 1e-06 | True | 1.602734425066951e-07 | 5.8627e-05 | 229 |
| nonnormal | 8 | rodas-dense | 1e-06 | True | 5.286289654407674e-09 | 0.001549519 | 216 |
| nonnormal | 8 | vode | 1e-06 | True | 2.696869343646046e-07 | 0.0027391789917601272 | 125 |
| nonnormal | 8 | dopri5 | 1e-08 | True | 1.7185204164672212e-09 | 0.000792532 | 523 |
| nonnormal | 8 | rodas-dense | 1e-08 | True | 3.592651971723132e-11 | 0.000827212 | 488 |
| nonnormal | 8 | vode | 1e-08 | True | 2.0954787784856404e-08 | 0.0031987069960450754 | 197 |
| large-vector | 4096 | dopri5 | 1e-04 | True | 1.6797304681001601e-06 | 0.007850237 | 73 |
| large-vector | 4096 | rodas-mf | 1e-04 | True | 2.4220646641093246e-08 | 0.118884217 | 72 |
| large-vector | 4096 | vode | 1e-04 | True | 5.353337232394395e-06 | 0.008366666996153072 | 44 |
| large-vector | 4096 | dopri5 | 1e-06 | True | 1.7751264028142977e-08 | 0.017397528 | 115 |
| large-vector | 4096 | rodas-mf | 1e-06 | True | 6.649526693158769e-10 | 0.250426214 | 120 |
| large-vector | 4096 | vode | 1e-06 | True | 1.422094269010188e-07 | 0.008403177009313367 | 55 |
| large-vector | 4096 | dopri5 | 1e-08 | True | 1.8656642897241227e-10 | 0.024544138 | 211 |
| large-vector | 4096 | rodas-mf | 1e-08 | True | 4.0510372834035024e-12 | 0.473791979 | 216 |
| large-vector | 4096 | vode | 1e-08 | True | 3.192813902153091e-09 | 0.012981678009964526 | 111 |
| stiff-control | 12 | dopri5 | 1e-04 | True | 1.7628627533568984e-07 | 0.171704692 | 186085 |
| stiff-control | 12 | rodas-dense | 1e-04 | True | 5.042566364465983e-10 | 0.000619693 | 284 |
| stiff-control | 12 | vode | 1e-04 | True | 1.8978828084104116e-06 | 0.8108381229976658 | 241340 |
| stiff-control | 12 | dopri5 | 1e-06 | True | 4.789107999963271e-10 | 0.181226721 | 186247 |
| stiff-control | 12 | rodas-dense | 1e-06 | True | 5.010679648975724e-10 | 0.001253743 | 572 |
| stiff-control | 12 | vode | 1e-06 | True | 1.875077245472312e-06 | 0.9588153320073616 | 240282 |
| stiff-control | 12 | dopri5 | 1e-08 | True | 7.540965782218711e-12 | 0.200043785 | 186643 |
| stiff-control | 12 | rodas-dense | 1e-08 | True | 1.9609591728197984e-11 | 0.001006113 | 1284 |
| stiff-control | 12 | vode | 1e-08 | True | 1.8710344821259994e-06 | 1.0030638860043837 | 238261 |

![Measured work–precision](WORK_PRECISION.png)

Common-error comparisons are in SUMMARY.json. A ratio larger than one means the comparator took longer than DOPRI5 on these samples. The comparison includes setup and callbacks; it does not isolate mathematical-stage cost or compiler/language overhead. VODE callbacks run in Python/NumPy and its imports are excluded. Three repetitions with randomized order provide a local observation, not a universal speed guarantee or confidence interval. Stiff controls identify the explicit stability penalty and are retained regardless of performance.
