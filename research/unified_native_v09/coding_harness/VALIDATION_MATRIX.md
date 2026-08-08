# VALIDATION_MATRIX.md

| Requirement | Test/check | Level | Expected | Status | Evidence |
|---|---|---|---|---|---|
| baseline | existing workspace release tests | software | pass before edit | PASS | `/mnt/data/rodas5p-v09-baseline-bg.log` |
| common Newton | scalar nonlinear and exact-linear tests | software/scientific | converged or explicit failure | PASS | `native_implicit_contracts` |
| transactional state | forced nonlinear failure | software | history unchanged | PASS | BDF rollback test |
| BDF1 definition | scalar amplification/mass tests | scientific | exact residual identity | PASS | order 0.98679 |
| BDF2 startup | fixed trajectory | software | first step BDF1, later BDF2 | PASS | startup test |
| BDF convergence | fixed-step sweep | numerical | order 1/2 | PASS | BDF2 order 2.02189 |
| Radau tableau | exact coefficient tests | scientific | exact identities | PASS | `A1=c`, stiff accuracy |
| Radau convergence | fixed-step sweep | numerical | order >=4.5 | PASS | order 4.99283 |
| stiff decay | scalar negative-real gate | scientific | expected damping | PASS | native gate JSON |
| noncommuting mass | manufactured problem | scientific | error within tolerance | PASS | native gate JSON |
| registry | candidate catalog | software | four anchors executable | PASS | `native_candidate_contracts` |
| unified gate | native data in unified document | operational | eligible but HOLD pending performance | PASS | schema v4 smoke output |
| existing behavior | full workspace regression | software | unchanged | PASS | 82/82 release tests |
| reproducibility | repeated report and thread checksum | operational | identical scientific fields | PASS | native byte compare; checksum `3819...db32` |
| review | independent diff audit | review | no critical/important unresolved | PASS WITH HOLD | `reviews/INDEPENDENT_DIFF_REVIEW.md` |
| global-error/total-cost ranking | complete-integrator Pareto harness | scientific/performance | required before promotion | NOT_RUN | next cycle |

Status: `PASS / CONCERN / FAIL / NOT_RUN / NOT_APPLICABLE`
