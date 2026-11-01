# WP9B preregistration: measured OpenEvolve mutation + MCTS-UCT prior corrections

Status at registration: DESIGN, before new WP9B perturbation IVP runs. Immutable WP9 parent: 1e4acb486fe5f497594b9cf040e9b791aca1e39b (PR #172). Original core RHS blob d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482; original frozen cases declaration blob 1bfc17ec2ed4b902cc06073807388f6e332e31d6. OpenEvolve offline source ZIP sha256 18ac7f1f7b9e1aca7c1539a768ea6cf4e9c2aee5452d52e5371faef34727ba91. The source/RHS files and WP1-WP9 data are read-only.

## Experiment and target

Use the actual frozen cooperative cubic RHS and four declared configurations, with asymmetric_2_refined treated ONLY as a resolution control of asymmetric_2, not an independent physical source. Candidate space is (cohort, physical parameter axis) with axes EXCHANGE_C01_REL (if c_01 positive), EXCHANGE_ALL_REL, CUBIC_ALL_REL (if g nonzero), DECAY_ALL_REL and FORCING_CONST. Forcing is a distinct source-conditional intervention +theta*k_i*y0_i.

A candidate experiment solves theta=+0.03,-0.03 and 0 from t=0 to 0.15 on 65 fixed output times using Radau and DOP853 with rtol=2e-12, atol=2e-14, both independently. Freeze everything before learning. For each theta, recompute pi(theta) from the left kernel of row-conservative Q(theta), require positive normalized weights and W(theta)=pi(theta)^T y(theta) with y_i>=-1e-12. Numerical cross-integrator consensus must be <=5e-10 absolute in the entire sampled state and observable; otherwise preserve FAIL, do not reward.

Compute dimensionless first- and second-response magnitudes from actual trajectories, not a proposal prior:
S1=abs(W(+h)-W(-h))/(2*h*max(abs(W(0)),1e-12)),
S2=abs(W(+h)+W(-h)-2*W(0))/(h*h*max(abs(W(0)),1e-12)).
Declare bounded reward r=[0.75*tanh(S1)+0.25*tanh(S2)]/(1+0.001*total_nfev), with zero if any safety/consensus failure. This is a candidate search heuristic, NOT formal error certification or a cosmological observable.

## Online action-feedback

Run 16 alternating waves, 8 OpenEvolve ProgramDatabase-backed mutation proposals and 8 MCTS-UCT proposals. After EVERY new candidate's genuine double-solver IVP experiment, update next-wave priors and OpenEvolve's actual ProgramDatabase fitness record. Use posterior-shrinkage mean m=(alpha*0.25+sum(rewards))/(alpha+n), alpha=2; MCTS prior P_i=epsilon/K+(1-epsilon)*softmax(beta*m_i), epsilon=0.1, beta=4.0. Mutation-axis probabilities use the same formula aggregated by axis with beta=6.0. Preserve 10% uniform exploration; no feedback from the experiment may alter the trajectory already being measured.

Predefine comparison arms with same seed 170117: adaptive policy updates both MCTS priors and OpenEvolve mutation distributions; fixed-uniform arm uses the same exact integrations, response metric, and record format but does not update the priors. Both arms have UCT value updates; only online prior/mutation correction distinguishes them. Share a measurement cache indexed by source, parameter and settings to avoid rerunning identical physics; record cache hits separately from solver integrations.

After policies are frozen, independently calculate holdout theta=+-0.018, horizon 0.20 for the selected top candidate per arm, again with Radau and DOP853, and report predicted vs actually measured response. Do not claim in-sample scoring superiority as external generalization.

## Admission and provenance

Verify tracked original blob identity before and after, no change to solver implementations, no tolerance relaxation, no merge, no main/default change. Extract source parameters from locked Rust source declaration (match by exact original source blob, parser and fixtures), not synthetic replacements. Record all failures and null controls (linear g=0 cubic; uniform symmetric exchange). Preserve raw JSONL trace of wave-by-wave pre/post priors, mutation probabilities, program DB ids and measured rewards, and pinned true evaluation counts. The real OpenEvolve Python module must be imported from the isolated local source ZIP, without external LLM calls, and class identity checked.

Acceptance: source locks match, 16 waves per arm complete with actual physical evaluations, observed prior/mutation distribution changes in adaptive but not control, all tests pass; raw data and reproducible command saved. This is exploratory measured feedback, NOT a rigorous bound, independent-author review, real REI Bianchi IGM owner-IVP, or matched-achieved-error native BE-v-RODAS5P.

OFFICIAL_DAG=7/23. SUCCESSOR_ADMITTED=0/12. PRODUCTION=HOLD. NO_MERGE=true. NO_DEFAULT_CHANGE=true. NO_TOLERANCE_RELAXATION=true. AUTO_NEXT_NODE=false.
