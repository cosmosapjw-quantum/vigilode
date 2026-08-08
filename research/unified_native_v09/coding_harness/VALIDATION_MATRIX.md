# VALIDATION_MATRIX.md

| Requirement | Test/check | Level | Expected | Status | Evidence |
|---|---|---|---|---|---|
| baseline | existing workspace release tests | software | pass before edit | PASS | `/mnt/data/rodas5p-v09-baseline-bg.log` |
| common Newton | scalar and vector nonlinear direct-oracle tests | software/scientific | converged or explicit failure | NOT_RUN | |
| transactional state | forced nonlinear failure | software | history unchanged | NOT_RUN | |
| BDF1 definition | one-step scalar/mass tests | scientific | exact residual identity | NOT_RUN | |
| BDF2 startup | fixed trajectory | software | first step BDF1, later BDF2 | NOT_RUN | |
| BDF convergence | fixed-step sweep | numerical | order 1/2 | NOT_RUN | |
| Radau tableau | exact coefficient tests | scientific | exact identities | NOT_RUN | |
| Radau convergence | fixed-step sweep | numerical | order >=4.5 | NOT_RUN | |
| stiff decay | scalar negative-real sweep | scientific | expected damping | NOT_RUN | |
| noncommuting mass | affine manufactured problem | scientific | residual/error within tolerance | NOT_RUN | |
| registry | candidate catalog | software | native anchors executable, deferred families retained | NOT_RUN | |
| unified gate | global order/stiff report | operational | candidate rows and no hidden work | NOT_RUN | |
| existing behavior | full workspace regression | software | unchanged | NOT_RUN | |
| reproducibility | repeated report checksum | operational | identical scientific fields | NOT_RUN | |
| review | independent diff audit | review | no critical/important unresolved | NOT_RUN | |

Status: `PASS / CONCERN / FAIL / NOT_RUN / NOT_APPLICABLE`
