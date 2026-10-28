#!/usr/bin/env julia
# Independent endpoint-reference construction for frozen local benchmark cases 4, 9, 11.
# Invocation: julia --project=PINNED_PROJECT independent_julia_reference.jl DESCRIPTOR.json NEW_OUTPUT_DIR probe|solve
# The controller owns the wall-time deadline, process group, and cumulative cost.
using JSON3, SHA, LinearAlgebra, SciMLBase, OrdinaryDiffEqTsit5, OrdinaryDiffEqRosenbrock
BLAS.set_num_threads(1)

const SCHEMA = "vigilode.local-bottleneck.external-reference.v1"
const PROBE_FACTOR = 128.0 * eps(Float64)
const LEVELS = (("tsit5_coarse", Tsit5(), 1e-8, 1e-10),
                ("tsit5_middle", Tsit5(), 1e-9, 1e-11),
                ("tsit5_tight", Tsit5(), 1e-10, 1e-12),
                ("rodas5p_tight", Rodas5P(), 1e-10, 1e-12))

function emit(path, object)
    payload = JSON3.write(object) * "\n"
    # O_EXCL also refuses an existing symlink. Preserve a partial first write
    # on I/O failure rather than truncating it during failure reporting.
    io = Base.Filesystem.open(path, Base.Filesystem.JL_O_WRONLY |
                              Base.Filesystem.JL_O_CREAT | Base.Filesystem.JL_O_EXCL, 0o600)
    try
        write(io, payload)
    finally
        close(io)
    end
end

function preserve_failure(path, row)
    try
        emit(path, row)
    catch record_error
        println(stderr, "Failure receipt was not replaced: ", sprint(showerror, record_error))
    end
    return nothing
end

function expect(condition, why)
    condition || error(why)
end

function numeric_vector(value, n, name)
    expect(length(value) == n, "$name dimension mismatch")
    vec = Float64.(value)
    expect(all(isfinite, vec), "$name is nonfinite")
    return vec
end

function near(a, b)
    isfinite(a) && isfinite(b) && abs(a-b) <= PROBE_FACTOR * max(1.0, abs(a), abs(b))
end

function expected_initial(id, n, m)
    if id == 4
        return [2*pi*i/n + 0.1*sin(3.0*i) for i in 0:n-1]
    elseif id == 9
        y = zeros(n)
        for i in 0:m-1
            y[i+1] = 0.01*sin(2*pi*i/m) + 0.001*sin(0.5*pi*i)
        end
        return y
    else
        return [i < m ? 5.0 + 0.01*sin(Float64(i)) : 0.01*cos(Float64(i)) for i in 0:n-1]
    end
end

function validate_descriptor(c)
    expect(String(c.kind) == "model_contract" && Int(c.schema_version) == 1,
           "descriptor schema mismatch")
    id = Int(c.case_id)
    expect(id in (4, 9, 11), "unsupported case")
    size = String(c.size)
    expect(size in ("smoke", "standard"), "unsupported size")
    m = id == 4 ? (size == "smoke" ? 16 : 4096) :
        id == 9 ? (size == "smoke" ? 8 : 256) :
                  (size == "smoke" ? 5 : 36)
    n = id == 4 ? m : id == 9 ? 2*m : 11*m
    ny = id == 11 ? 10 : 0
    nominal_end = id == 4 ? (size == "smoke" ? 0.02 : 10.0) :
                  id == 9 ? (size == "smoke" ? 0.002 : 1.0) :
                            (size == "smoke" ? 0.01 : 5.0)
    tend = Float64(c.t_end)
    expect(isfinite(tend) && tend > 0, "invalid end")
    expect(near(tend, nominal_end), "end differs from frozen standard or smoke model")
    expect(near(Float64(c.t_start), 0.0), "start mismatch")
    expect(String(c.model_id) == "vigilode.local-bottleneck.case$(id).v1",
           "model identity mismatch")
    mc = c.model_contract
    expect(Int(mc.id) == id && Int(mc.case) == id && Int(mc.dimension) == n &&
           String(mc.size) == size && Bool(mc.autonomous), "model metadata mismatch")
    expect(Int(mc.dimension_smoke) == (id == 4 ? 16 : id == 9 ? 16 : 55) &&
           Int(mc.dimension_standard) == (id == 4 ? 4096 : id == 9 ? 512 : 396),
           "declared dimensions mismatch")
    expect(Int(mc.grid[1]) == m && Int(mc.grid[2]) == ny &&
           near(Float64(mc.t_span[1]), 0.0) && near(Float64(mc.t_span[2]), tend),
           "grid or time span mismatch")
    expect(String(mc.reference_kind) == "external-reference-required", "reference kind mismatch")
    p = c.parameters
    expect(JSON3.write(p) == JSON3.write(mc.parameters), "parameter copies mismatch")
    if id == 4
        expect(Int(p.radius) == (size == "smoke" ? 4 : 32) &&
               Float64(p.coupling) == 1.0 && Float64(p.alpha) == 0.2 &&
               String(p.frequency) == "0.5+0.1cos(2pi i/N)", "case4 parameters mismatch")
        expect(String(mc.variant) == "local periodic coupling, no self term; radius4 smoke/32standard",
               "case4 variant mismatch")
    elseif id == 9
        expect(Float64(p.spring) == 1e6 && Float64(p.quartic) == 1.0 &&
               String(p.boundary) == "periodic", "case9 parameters mismatch")
        expect(String(mc.variant) == "periodic homogeneous stiff springs plus quartic bonds",
               "case9 variant mismatch")
    else
        expect(Float64(p.F) == 10.0 && Float64(p.c) == 10.0 &&
               Float64(p.b) == 10.0 && Float64(p.h) == 1.0 &&
               String(p.fast_ring) == "global periodic ring; stated forward-advection variant",
               "case11 parameters mismatch")
        expect(String(mc.variant) == "global fast cyclic ring with attachment's forward cyclic advection",
               "case11 variant mismatch")
    end
    initial = numeric_vector(c.initial_state, n, "initial_state")
    metadata_initial = numeric_vector(mc.initial_state, n, "metadata initial_state")
    formula_initial = expected_initial(id, n, m)
    expect(all(near.(initial, metadata_initial)) && all(near.(initial, formula_initial)),
           "initial state or ordering mismatch")
    expect(c.exact_final === nothing, "external-reference case unexpectedly analytic")
    coefficients = id == 4 ? (radius=Int(p.radius), alpha=Float64(p.alpha),
                              coupling=Float64(p.coupling)) :
                   id == 9 ? (spring=Float64(p.spring), quartic=Float64(p.quartic)) :
                   (F=Float64(p.F), c=Float64(p.c), b=Float64(p.b), h=Float64(p.h))
    return (; id, size, n, m, tend, initial, coefficients)
end

# All indexing below is Julia 1-based; descriptor state ordering is checked above.
function independent_rhs!(du, y, cfg, t)
    id, n, m, p = cfg.id, cfg.n, cfg.m, cfg.coefficients
    if id == 4
        r = Int(p.radius)
        alpha, coupling = Float64(p.alpha), Float64(p.coupling)
        for i in 1:n
            s = 0.0
            for d in -r:r
                d == 0 && continue
                s += sin(y[mod1(i+d,n)] - y[i] - alpha)
            end
            du[i] = 0.5 + 0.1*cos(2*pi*(i-1)/n) + coupling*s/(2*r)
        end
    elseif id == 9
        spring, quartic = Float64(p.spring), Float64(p.quartic)
        for i in 1:m
            next = mod1(i+1,m)
            previous = mod1(i-1,m)
            a = y[next] - y[i]
            b = y[i] - y[previous]
            du[i] = y[m+i]
            du[m+i] = spring*(a-b) + quartic*(a^3-b^3)
        end
    else
        fast_count = 10*m
        F, c, b, h = Float64(p.F), Float64(p.c), Float64(p.b), Float64(p.h)
        for i in 1:m
            slow_forward = y[mod1(i+1,m)]
            slow_back = y[mod1(i-2,m)]
            slow_previous = y[mod1(i-1,m)]
            fast_sum = 0.0
            for j in 1:10
                fast_sum += y[m+10*(i-1)+j]
            end
            du[i] = (slow_forward-slow_back)*slow_previous - y[i] + F - (h*c/b)*fast_sum
        end
        for k in 1:fast_count
            next = y[m+mod1(k+1,fast_count)]
            back = y[m+mod1(k-2,fast_count)]
            previous = y[m+mod1(k-1,fast_count)]
            du[m+k] = c*b*(next-back)*previous - c*y[m+k] + (h*c/b)*y[div(k-1,10)+1]
        end
    end
    return nothing
end

function validate_probes(c, cfg)
    samples = Any[(label="initial", time=0.0, state=c.initial_state, expected=c.rhs_initial)]
    expect(length(c.rhs_probes) == 2, "RHS probe count mismatch")
    for (j, probe) in enumerate(c.rhs_probes)
        expected_time = (j == 1 ? 0.11 : 0.73)*cfg.tend
        expect(near(Float64(probe.t),expected_time), "perturbed probe time mismatch")
        state = numeric_vector(probe.state,cfg.n,"perturbed probe state")
        expect(all(near(state[k],cfg.initial[k]+0.001*j*k/cfg.n) for k in 1:cfg.n),
               "perturbed probe state mismatch")
        push!(samples, (label="perturbed_$j", time=Float64(probe.t),
                        state=probe.state, expected=probe.rhs))
    end
    observations = Any[]
    for sample in samples
        y = numeric_vector(sample.state, cfg.n, "probe state")
        expected = numeric_vector(sample.expected, cfg.n, "probe RHS")
        actual = similar(y)
        independent_rhs!(actual, y, cfg, sample.time)
        residual = maximum(abs.(actual .- expected))
        scaled = maximum(abs(actual[i]-expected[i]) /
                         max(1.0,abs(actual[i]),abs(expected[i])) for i in eachindex(actual))
        pass = all(near.(actual, expected))
        push!(observations, Dict("label"=>sample.label, "time"=>sample.time,
                                 "maximum_absolute_residual"=>residual,
                                 "maximum_scaled_residual"=>scaled,
                                 "threshold_factor_eps"=>PROBE_FACTOR, "pass"=>pass))
    end
    return observations
end

function callback_allocation_probe(cfg)
    state = copy(cfg.initial)
    out = similar(state)
    for _ in 1:10
        independent_rhs!(out,state,cfg,0.0)
    end
    allocations = [@allocated independent_rhs!(out,state,cfg,0.0) for _ in 1:10]
    return Dict("warmed_callback_allocated_bytes"=>allocations,
                "maximum_allocated_bytes"=>maximum(allocations),
                "requirement"=>"zero allocated bytes in actual RHS callback")
end

function invariants(cfg, y)
    if cfg.id == 9
        m = cfg.m
        spring, quartic = cfg.coefficients.spring, cfg.coefficients.quartic
        energy = 0.0
        momentum = 0.0
        for i in 1:m
            d = y[mod1(i+1,m)]-y[i]
            energy += 0.5*y[m+i]^2 + 0.5*spring*d^2 + 0.25*quartic*d^4
            momentum += y[m+i]
        end
        return Dict("energy"=>energy,"total_momentum"=>momentum,
                    "scope"=>"periodic FPUT diagnostics")
    elseif cfg.id == 4
        return Dict("phase_mean"=>sum(y)/cfg.n,
                    "scope"=>"descriptive only; no phase conservation assertion")
    else
        return Dict("maximum_absolute_state"=>maximum(abs,y),
                    "scope"=>"descriptive only; chaotic endpoint sensitivity remains")
    end
end

struct ResourceRefusal <: Exception
    message::String
end
Base.showerror(io::IO, err::ResourceRefusal) = print(io, err.message)

function enforce_resource_gate(cfg, outdir)
    # The frozen cross-method check cannot use a dense 4096x4096 Jacobian.
    # A verified sparse Jacobian path is a prerequisite for any case4 solve;
    # completing only the explicit levels would not construct a reference.
    if cfg.id == 4
        reason = "case4 requires independently verified sparse Jacobian path before Rodas5P crosscheck"
        emit(joinpath(outdir, "RESOURCE_REFUSAL.json"),
             Dict("status"=>"HOLD_RESOURCE_REFUSAL", "case_id"=>cfg.id,
                  "dimension"=>cfg.n, "reason"=>reason,
                  "started_solve_levels"=>0, "entire_solve_refused"=>true,
                  "cross_method_check_status"=>"NOT_RUN_RESOURCE_REFUSAL",
                  "reference_eligible"=>false))
        throw(ResourceRefusal(reason))
    end
    expect(cfg.id in (9,11), "dense Rodas5P not authorized for this case")
    return nothing
end

function solve_level(label, algorithm, rtol, atol, cfg, outdir)
    rhs_count = Ref(0)
    function counted_rhs!(du,y,p,t)
        rhs_count[] += 1
        independent_rhs!(du,y,p,t)
    end
    prob = ODEProblem(counted_rhs!, copy(cfg.initial), (0.0,cfg.tend), cfg)
    started = time_ns()
    try
        sol = solve(prob, algorithm; reltol=rtol, abstol=atol, maxiters=2_000_000,
                    save_everystep=false, save_start=false, save_end=true, dense=false)
        elapsed = (time_ns()-started)*1e-9
        state = isempty(sol.u) ? nothing : Float64.(sol.u[end])
        success = SciMLBase.successful_retcode(sol.retcode) && state !== nothing &&
                  length(state) == cfg.n && all(isfinite,state) && sol.t[end] == cfg.tend
        stats = sol.stats
        row = Dict("label"=>label,"algorithm"=>string(typeof(algorithm)),
                   "rtol"=>rtol,"atol"=>atol,"maxiters"=>2_000_000,
                   "elapsed_seconds"=>elapsed,"retcode"=>string(sol.retcode),
                   "solver_success"=>success,"final_time"=>isempty(sol.t) ? nothing : sol.t[end],
                   "full_endpoint_state"=>state !== nothing && all(isfinite,state) ? state : nothing,
                   "rhs_callback_count"=>rhs_count[],
                   "solver_counters"=>Dict("naccept"=>stats.naccept,"nreject"=>stats.nreject,
                                           "nf"=>stats.nf,"njacs"=>stats.njacs),
                   "invariants"=>success ? invariants(cfg,state) : nothing)
        emit(joinpath(outdir,"$(label).json"),row)
        return row
    catch err
        row = Dict("label"=>label,"status"=>"FAILED_SOLVE_EXCEPTION",
                   "elapsed_seconds"=>(time_ns()-started)*1e-9,
                   "exception"=>sprint(showerror,err),"rhs_callback_count"=>rhs_count[])
        preserve_failure(joinpath(outdir,"$(label).json"),row)
        rethrow()
    end
end

function endpoint_refinement(rows)
    expect(length(rows) == length(LEVELS) &&
           all(rows[i]["label"] == LEVELS[i][1] for i in eachindex(rows)),
           "incomplete endpoint refinement levels")
    states = [row["full_endpoint_state"] for row in rows]
    pairwise = Dict{String,Float64}()
    for i in eachindex(rows), j in i+1:length(rows)
        pairwise["$(rows[i]["label"])__$(rows[j]["label"])"] = maximum(abs.(states[i].-states[j]))
    end
    U = max(1e-12,10*maximum(values(pairwise)))
    return (; states, pairwise, U, reference_state=states[3])
end

function construct_reference(descriptor_path, outdir, mode)
    descriptor_bytes = read(descriptor_path)
    descriptor_sha = bytes2hex(sha256(descriptor_bytes))
    descriptor = JSON3.read(String(descriptor_bytes))
    cfg = validate_descriptor(descriptor)
    probes = validate_probes(descriptor,cfg)
    allocations = callback_allocation_probe(cfg)
    initial_invariants = invariants(cfg,cfg.initial)
    control = Dict("schema"=>"vigilode.independent-julia-reference-control.v1",
                   "source_descriptor_sha256"=>descriptor_sha,
                   "julia_source_sha256"=>bytes2hex(sha256(read(@__FILE__))),
                   "case_id"=>cfg.id,"size"=>cfg.size,"dimension"=>cfg.n,
                   "t_end"=>cfg.tend,"rhs_probes"=>probes,
                   "callback_allocation_probe"=>allocations,
                   "initial_invariants"=>initial_invariants,
                   "julia_version"=>string(VERSION),"mode"=>mode)
    emit(joinpath(outdir,"PROBE_CONTROL.json"),control)
    expect(all(row["pass"] for row in probes), "RHS contract mismatch")
    expect(allocations["maximum_allocated_bytes"] == 0, "RHS callback allocated")
    mode == "probe" && return
    enforce_resource_gate(cfg,outdir)
    rows = Any[]
    for (label,algorithm,rtol,atol) in LEVELS
        row = solve_level(label,algorithm,rtol,atol,cfg,outdir)
        push!(rows,row)
        expect(row["solver_success"], "$label solver failure")
    end
    refinement = endpoint_refinement(rows)
    pairwise, U = refinement.pairwise, refinement.U
    evidence = Dict("schema"=>"vigilode.independent-julia-reference-evidence.v1",
                    "source_descriptor_sha256"=>descriptor_sha,"case_id"=>cfg.id,
                    "size"=>cfg.size,"dimension"=>cfg.n,"t_end"=>cfg.tend,
                    "probe_control"=>control,"levels"=>rows,
                    "pairwise_endpoint_inf"=>pairwise,"empirical_uncertainty"=>U,
                    "uncertainty_rule"=>"max(1e-12, 10 times largest pairwise infinity difference)",
                    "claim_ceiling"=>"empirical full-vector endpoint comparison only")
    evidence_path = joinpath(outdir,"REFERENCE_EVIDENCE.json")
    emit(evidence_path,evidence)
    expect(U <= 1e-5,"empirical uncertainty exceeds 1e-5")
    reference = Dict("schema"=>SCHEMA,"model_descriptor"=>descriptor,
                     "model_descriptor_sha256"=>descriptor_sha,
                     "norm"=>"max_component_absolute",
                     "reference_state"=>refinement.reference_state,"uncertainty"=>U,
                     "provenance"=>Dict("source"=>"independent Julia RHS from frozen VigilODE source equations",
                                        "method"=>"Tsit5 three tolerances and Rodas5P cross-method endpoint refinement",
                                        "evidence_sha256"=>bytes2hex(sha256(read(evidence_path)))))
    emit(joinpath(outdir,"EXTERNAL_REFERENCE.json"),reference)
end

function main(args=ARGS)
    expect(length(args)==3, "descriptor path, new output directory, probe|solve required")
    descriptor_path, outdir, mode = args
    expect(mode in ("probe","solve"), "mode must be probe or solve")
    # This mkdir is deliberately outside the catch. Its success establishes
    # ownership; an existing directory/file/symlink must receive no writes.
    mkdir(outdir)
    try
        construct_reference(descriptor_path,outdir,mode)
    catch err
        status = err isa ResourceRefusal ? "HOLD_RESOURCE_REFUSAL" : "FAILED_REFERENCE_CONSTRUCTION"
        preserve_failure(joinpath(outdir,"FAILED.json"),
                         Dict("status"=>status,"exception"=>sprint(showerror,err,catch_backtrace())))
        rethrow()
    end
end

if abspath(PROGRAM_FILE) == @__FILE__
    main()
end
