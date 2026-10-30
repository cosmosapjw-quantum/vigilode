# VigilODE N4 → 다음 후속 연구 | 인계

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
RESEARCH_UNIT=N4_PARTIAL_SOURCE_BOUND_INTERVAL_KERNEL_AND_ORIGINAL_RAWU8
RUST=1.99.1
BASE_PR=140
BASE_HEAD=713b3de68e953c69899d694508e25fea0f20cf73
PARENT_N3H_ARCHIVE_SHA256=a29c0bdd84f989f8762d4e99f87c6dcae6c2fe2d91ed8fba1f161cb06e0221ea
OFFICIAL_VIGILODE_DAG=7/23
CENTRAL_REI_DAG=28/88
RVJ_FOLLOWON=3/8_PLUS_N4_PARTIAL
SUCCESSOR_ADMITTED=0/12
C9_DISTINCT_AUTHOR=0/1_NOT_RUN
C10=NOT_STARTED
PRODUCTION=HOLD
MERGE_READY=false
AUTO_NEXT_NODE=false

SOURCE_PHYSICS=a_j'=(-kappa_j+i*omega*xi_j)*a_j+i*sum_alpha chi_alpha*phi_alpha_j*|m_alpha|^2*m_alpha
MOMENTS=m_alpha=sum_j w_j*phi_alpha_j*a_j
PARAMETERS=kappa_slow=1/4,kappa_fast=1000,omega=1/2,chi=(1/20,1/50),phi=(1,(-1)^j)
KERNEL=M_l(a,b,theta,h)=integral_0^h exp((-a+i*theta)*(h-s))*exp(-b*s)*s^l ds
KERNEL_DOMAIN=h<=1/4,|theta*h|<=1/4,l<=9,exact_represented_source,positive_a_b
KERNEL_FRACTION_VS_MPMATH=240/240_PASS
KERNEL_RUST_VS_FRACTION=240/240_PASS
N3H_NATIVE_FLOAT_OUTSIDE_TIGHT_FRACTION=115/240
N3H_NATIVE_FLOAT_CONTAINED_RUST_INTERVAL=240/240
RODAS8_TABLEAU_SHA256=211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6
RODAS8_RUST_SMALL_REFERENCE=n12,n24,h=.001,.25
RODAS8_RAWU8_RUST_NATIVE_INTERVAL=4/4_SCOPED
RODAS8_RUST_VS_INDEPENDENT_PYTHON=4/4_SCOPED
RODAS8_H_0P25_EMBEDDED_WRMS=ABOUT_253200_REFUSE
N3H_SAME_SOURCE_H_0P25_MAX_BDF_ERROR=n12:1.6028975022286396e-13,n24:7.741897400936892e-14
SMALL_REFERENCE_TRANSACTION=H_0P25_REJECT;H_1E-8_ACCEPT;ATTEMPTED_16_ACCEPTED_8_REJECTED_8
RUST_TESTS=34/34_PASS
RUST_STRICT_CLIPPY_FMT=PASS
NATIVE_PR131_INTEGRATED_RUST=NOT_RUN_BUILD_TIME_BLOCKER
FULL_FLOATING_CARRIER_ENCLOSURE=NOT_RUN
MATCHED_ACHIEVED_ERROR_SPEED=NOT_RUN

NEXT_BOUNDARY:
1. Verify new full ZIP SHA256 and manifest from saved remote Drive or Library, restore source and Rust 1.99.1.
2. Integrate outward moment bounds into actual N3H complex nonlinear convolution and accepted-window carrier; build a certificate on output including roundoff.
3. Source-lock original PR131 exact callbacks and try an actual native all-eight raw-U residual test at its HEAD, not a standalone reimplementation.
4. Connect fail-closed transaction to source epoch, accepted/rejected state, error controller, original tableau, causal RHS-only homotopy primary, JVP optional fallback.
5. Compare adaptive matched achieved error and total attempted work at n3/8/12/24/65/257. Include negative/slow cases and baseline cost.
6. External C9 approval requires different actual reviewer identity and a source-locked run/review receipt; self-review does not satisfy 0/1.

STOP: no force_accept promotion, no main merges, no automatic N5, no default switches, no tolerance relaxation.
