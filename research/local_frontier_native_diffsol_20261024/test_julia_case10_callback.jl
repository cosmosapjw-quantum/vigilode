# Bounded regression for the actual rhs! definition extracted unchanged from
# each adapter. No package installation or environment/manifest mutation.
# ARGS: prepatch|callback|parity old_adapter new_adapter descriptor output_json
using JSON3, SHA

length(ARGS) == 5 || error("mode old_adapter new_adapter descriptor output_json required")
mode, old_path, new_path, descriptor_path, output_path = ARGS
mode in ("prepatch", "callback", "parity") || error("invalid mode")
ispath(output_path) && error("refusing to replace evidence")
descriptor = JSON3.read(read(descriptor_path, String))
Int(descriptor.case_id) == 10 || error("case10 descriptor required")
String(descriptor.size) == "standard" || error("standard descriptor required")
Float64(descriptor.t_start) == 0.0 && Float64(descriptor.t_end) == 1.0 || error("frozen interval required")

function actual_callback(path, descriptor_path)
    source = read(path, String)
    first = findfirst("function rhs!", source)
    last = findfirst("\nfor probe in c.rhs_probes", source)
    first !== nothing && last !== nothing || error("adapter callback boundary absent")
    callback_source = source[first.start:prevind(source, last.start)]
    scope = Module(gensym(:ActualAdapter))
    Core.eval(scope, :(using JSON3))
    # Preserve the actual adapter's non-const global configuration semantics.
    Core.eval(scope, :(c = JSON3.read(read($descriptor_path, String))))
    Core.eval(scope, :(p = c.parameters))
    for line in split(source, '\n')
        if startswith(line, "const CASE10_OMEGA_C = ")
            Base.include_string(scope, line, path)
        end
    end
    Base.include_string(scope, callback_source, path)
    return Base.invokelatest(getfield, scope, :rhs!)
end

function allocation_bytes(callback::F, y, t) where F
    du = similar(y)
    for _ in 1:100
        callback(du, y, nothing, t)
    end
    samples = Int[]
    for _ in 1:10
        push!(samples, @allocated callback(du, y, nothing, t))
    end
    return Dict("samples" => samples, "minimum" => minimum(samples), "maximum" => maximum(samples))
end

bits(y) = [string(reinterpret(UInt64, x), base=16, pad=16) for x in y]

function compare_callbacks(old::F, new::G, states) where {F,G}
    rows = []
    for (label, y, t) in states
        a, b = similar(y), similar(y)
        old(a, y, nothing, t)
        new(b, y, nothing, t)
        push!(rows, Dict("label" => label, "state" => y, "time" => t,
                        "old_rhs_bits" => bits(a), "new_rhs_bits" => bits(b),
                        "bit_identical" => bits(a) == bits(b)))
    end
    return rows
end

function solve_case(callback::F, algorithm, initial, tend) where F
    warm_prob = ODEProblem(callback, copy(initial), (0., min(tend, 1e-4)))
    solve(warm_prob, algorithm; reltol=1e-6, abstol=1e-9, maxiters=200000,
          save_everystep=false, save_start=false, save_end=true, dense=false)
    prob = ODEProblem(callback, copy(initial), (0., tend))
    sol = solve(prob, algorithm; reltol=1e-6, abstol=1e-9, maxiters=200000,
                save_everystep=false, save_start=false, save_end=true, dense=false)
    state = Float64.(sol.u[end])
    stats = Dict(string(name) => getfield(sol.stats, name) for name in fieldnames(typeof(sol.stats)))
    return Dict("success" => SciMLBase.successful_retcode(sol.retcode) && all(isfinite, state) && sol.t[end] == tend,
                "state" => state, "state_bits" => bits(state), "final_time" => sol.t[end],
                "retcode" => string(sol.retcode), "stats" => stats)
end

old = actual_callback(old_path, descriptor_path)
initial = Float64.(descriptor.initial_state)
states = [("initial", initial, 0.0)]
for (index, probe) in enumerate(descriptor.rhs_probes)
    push!(states, ("descriptor_probe_$(index)", Float64.(probe.state), Float64(probe.t)))
end
append!(states, [
    ("mixed_sign_momentum", [0.2, -0.3, 0.4, -0.7, 0.4, -0.3], 0.4),
    ("zero_momentum", zeros(6), 0.7),
    ("small_momentum", [0., 0., 0., 1e-100, -2e-100, 3e-100], 0.8),
    ("large_finite_momentum", [0., 0., 0., 1e100, -2e100, 3e100], 0.9),
])
report = Dict{String,Any}(
    "schema" => "vigilode.julia-case10-callback-regression.v1", "mode" => mode,
    "julia_version" => string(VERSION), "project" => Base.active_project(),
    "project_sha256" => bytes2hex(sha256(read(Base.active_project()))),
    "manifest_sha256" => bytes2hex(sha256(read(joinpath(dirname(Base.active_project()), "Manifest.toml")))),
    "old_source_sha256" => bytes2hex(sha256(read(old_path))),
    "new_source_sha256" => bytes2hex(sha256(read(new_path))),
    "descriptor_sha256" => bytes2hex(sha256(read(descriptor_path))),
    "settings" => Dict("rtol" => 1e-6, "atol" => 1e-9, "max_steps" => 200000, "threads" => Threads.nthreads()),
    "callback_configuration_semantics" => "actual extracted function; non-const c and p globals retained",
    "timing_authority" => "diagnostic_only", "timing_comparison_performed" => false)
report["old_allocations"] = Base.invokelatest(allocation_bytes, old, initial, 0.0)

if mode == "prepatch"
    report["status"] = report["old_allocations"]["minimum"] > 0 ? "PREPATCH_ALLOCATION_FAILURE_REPRODUCED" : "PREPATCH_FAILURE_NOT_REPRODUCED"
else
    new = actual_callback(new_path, descriptor_path)
    report["rhs_controls"] = Base.invokelatest(compare_callbacks, old, new, states)
    report["new_allocations"] = Base.invokelatest(allocation_bytes, new, initial, 0.0)
    report["status"] = !all(row["bit_identical"] for row in report["rhs_controls"]) ? "RHS_BIT_IDENTITY_FAILED" :
                       report["new_allocations"]["maximum"] != 0 ? "ALLOCATION_HOLD" : "CALLBACK_PASS"
    if mode == "parity" && report["status"] == "CALLBACK_PASS"
        @eval using SciMLBase, OrdinaryDiffEqTsit5, OrdinaryDiffEqRosenbrock, LinearAlgebra
        BLAS.set_num_threads(1)
        parity = Dict{String,Any}()
        for (method, algorithm) in (("julia-tsit5", Tsit5()), ("julia-rodas5p", Rodas5P()))
            a = Base.invokelatest(solve_case, old, algorithm, initial, Float64(descriptor.t_end))
            open(output_path * ".samples.jsonl", "a") do stream
                println(stream, JSON3.write(Dict("method" => method, "source" => "old", "outcome" => a)))
            end
            b = Base.invokelatest(solve_case, new, algorithm, initial, Float64(descriptor.t_end))
            open(output_path * ".samples.jsonl", "a") do stream
                println(stream, JSON3.write(Dict("method" => method, "source" => "new", "outcome" => b)))
            end
            equal = a["success"] && b["success"] && a["state_bits"] == b["state_bits"] &&
                    a["final_time"] == b["final_time"] && a["retcode"] == b["retcode"] && a["stats"] == b["stats"]
            parity[method] = Dict("old" => a, "new" => b, "endpoint_and_stats_identical" => equal)
        end
        report["solver_parity"] = parity
        report["status"] = all(row["endpoint_and_stats_identical"] for row in values(parity)) ? "PARITY_PASS" : "SOLVER_PARITY_FAILED"
    end
end
open(output_path, "w") do stream
    JSON3.write(stream, report)
    println(stream)
end
println(JSON3.write(report))
report["status"] in ("PREPATCH_ALLOCATION_FAILURE_REPRODUCED", "CALLBACK_PASS", "PARITY_PASS") || exit(2)
