# Julia Comparator Benchmark for VigilODE Evaluation (v2 with strict execution provenance)
# Problems: Robertson (n=3), HIRES (n=8), Van der Pol (mu=1000, n=2), Brusselator 1D (n=100)
# Algorithms: Rodas5P() (Rosenbrock-Wanner 5th order), FBDF() (Fixed-leading coefficient BDF), CVODE_BDF()

using OrdinaryDiffEq
using OrdinaryDiffEqBDF
using Sundials
using LinearAlgebra
using Statistics
using JSON3
using SHA

# 1. Problem Definitions

function robertson!(dy, y, p, t)
    dy[1] = -0.04 * y[1] + 1e4 * y[2] * y[3]
    dy[2] = 0.04 * y[1] - 1e4 * y[2] * y[3] - 3e7 * y[2]^2
    dy[3] = 3e7 * y[2]^2
end

function robertson_jac!(J, y, p, t)
    fill!(J, 0.0)
    J[1, 1] = -0.04
    J[1, 2] = 1e4 * y[3]
    J[1, 3] = 1e4 * y[2]
    J[2, 1] = 0.04
    J[2, 2] = -1e4 * y[3] - 6e7 * y[2]
    J[2, 3] = -1e4 * y[2]
    J[3, 2] = 6e7 * y[2]
end

function hires!(dy, y, p, t)
    dy[1] = -1.71 * y[1] + 0.43 * y[2] + 8.32 * y[3] + 0.0007
    dy[2] = 1.71 * y[1] - 8.75 * y[2]
    dy[3] = -10.03 * y[3] + 0.43 * y[4] + 0.035 * y[5]
    dy[4] = 8.32 * y[2] + 1.71 * y[3] - 1.12 * y[4]
    dy[5] = -1.745 * y[5] + 0.43 * y[6] + 0.43 * y[7]
    dy[6] = -280.0 * y[6] * y[8] + 0.69 * y[4] + 1.71 * y[5] - 0.43 * y[6] + 0.69 * y[7]
    dy[7] = 280.0 * y[6] * y[8] - 1.81 * y[7]
    dy[8] = -280.0 * y[6] * y[8] + 1.81 * y[7]
end

function hires_jac!(J, y, p, t)
    fill!(J, 0.0)
    J[1, 1] = -1.71;   J[1, 2] = 0.43;  J[1, 3] = 8.32
    J[2, 1] = 1.71;    J[2, 2] = -8.75
    J[3, 3] = -10.03;  J[3, 4] = 0.43;  J[3, 5] = 0.035
    J[4, 2] = 8.32;    J[4, 3] = 1.71;  J[4, 4] = -1.12
    J[5, 5] = -1.745;  J[5, 6] = 0.43;  J[5, 7] = 0.43
    J[6, 4] = 0.69;    J[6, 5] = 1.71;  J[6, 6] = -280.0 * y[8] - 0.43; J[6, 7] = 0.69; J[6, 8] = -280.0 * y[6]
    J[7, 6] = 280.0 * y[8];             J[7, 7] = -1.81;                 J[7, 8] = 280.0 * y[6]
    J[8, 6] = -280.0 * y[8];            J[8, 7] = 1.81;                  J[8, 8] = -280.0 * y[6]
end

function vdp!(dy, y, p, t)
    dy[1] = y[2]
    dy[2] = 1000.0 * (1.0 - y[1]^2) * y[2] - y[1]
end

function vdp_jac!(J, y, p, t)
    J[1, 1] = 0.0
    J[1, 2] = 1.0
    J[2, 1] = -2000.0 * y[1] * y[2] - 1.0
    J[2, 2] = 1000.0 * (1.0 - y[1]^2)
end

function brusselator1d!(dy, y, p, t)
    n = length(y)
    m = div(n, 2)
    d = ((m + 1)^2) / 50.0
    for k in 1:m
        a = 2 * k - 1
        b = 2 * k
        u = y[a]
        v = y[b]
        ul = (k == 1) ? 1.0 : y[a - 2]
        vl = (k == 1) ? 3.0 : y[b - 2]
        ur = (k == m) ? 1.0 : y[a + 2]
        vr = (k == m) ? 3.0 : y[b + 2]
        dy[a] = 1.0 + u^2 * v - 4.0 * u + d * (ul - 2.0 * u + ur)
        dy[b] = 3.0 * u - u^2 * v + d * (vl - 2.0 * v + vr)
    end
end

function brusselator1d_jac!(J, y, p, t)
    n = length(y)
    m = div(n, 2)
    d = ((m + 1)^2) / 50.0
    fill!(J, 0.0)
    for k in 1:m
        a = 2 * k - 1
        b = 2 * k
        u = y[a]
        v = y[b]
        if k > 1
            J[a, a - 2] = d
            J[b, b - 2] = d
        end
        J[a, a] = 2.0 * u * v - 4.0 - 2.0 * d
        J[a, b] = u^2
        J[b, a] = 3.0 - 2.0 * u * v
        J[b, b] = -u^2 - 2.0 * d
        if k < m
            J[a, a + 2] = d
            J[b, b + 2] = d
        end
    end
end

function brusselator1d_u0(n)
    m = div(n, 2)
    u0 = zeros(Float64, n)
    for k in 1:m
        x = k / (m + 1)
        u0[2 * k - 1] = 1.0 + sin(2.0 * pi * x)
        u0[2 * k] = 3.0
    end
    return u0
end

function oregonator!(dy, y, p, t)
    q = 8.375e-6
    dy[1] = 77.27 * (y[2] - y[1] * y[2] + y[1] - q * y[1]^2)
    dy[2] = (1.0 / 77.27) * (-y[2] - y[1] * y[2] + y[3])
    dy[3] = 0.161 * (y[1] - y[3])
end

function oregonator_jac!(J, y, p, t)
    q = 8.375e-6
    fill!(J, 0.0)
    J[1, 1] = 77.27 * (-y[2] + 1.0 - 2.0 * q * y[1])
    J[1, 2] = 77.27 * (1.0 - y[1])
    J[2, 1] = (1.0 / 77.27) * (-y[2])
    J[2, 2] = (1.0 / 77.27) * (-1.0 - y[1])
    J[2, 3] = 1.0 / 77.27
    J[3, 1] = 0.161
    J[3, 3] = -0.161
end

# 2. Problem Setup Registry
struct ProblemConfig
    id::String
    tspan::Tuple{Float64, Float64}
    u0::Vector{Float64}
    atol_scale::Vector{Float64}
    ode_fun::ODEFunction
end

function get_problems()
    [
        ProblemConfig(
            "robertson",
            (0.0, 40.0),
            [1.0, 0.0, 0.0],
            [1e-4, 1e-4, 1e-4],
            ODEFunction(robertson!, jac=robertson_jac!)
        ),
        ProblemConfig(
            "oregonator",
            (0.0, 360.0),
            [1.0, 2.0, 3.0],
            fill(1e-4, 3),
            ODEFunction(oregonator!, jac=oregonator_jac!)
        ),
        ProblemConfig(
            "hires",
            (0.0, 321.8122),
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
            fill(1e-4, 8),
            ODEFunction(hires!, jac=hires_jac!)
        ),
        ProblemConfig(
            "van-der-pol-mu1000",
            (0.0, 2000.0),
            [2.0, 0.0],
            [1.0, 1.0],
            ODEFunction(vdp!, jac=vdp_jac!)
        ),
        ProblemConfig(
            "brusselator-1d-n100",
            (0.0, 10.0),
            brusselator1d_u0(100),
            fill(1.0, 100),
            ODEFunction(brusselator1d!, jac=brusselator1d_jac!)
        ),
    ]
end

# WRMS Error metric matching VigilODE harness
function calc_max_wrms(states, ref_states, atol_scale, error_rtol)
    n_pts = length(ref_states)
    n = length(atol_scale)
    sqrt_n = sqrt(Float64(n))
    worst = 0.0
    common_atol = error_rtol .* atol_scale

    for k in 1:n_pts
        y = states[k]
        r = ref_states[k]
        norm = 0.0
        for i in 1:n
            scale = common_atol[i] + error_rtol * abs(r[i])
            diff = y[i] - r[i]
            e = diff / scale
            norm = hypot(norm, e / sqrt_n)
        end
        worst = max(worst, norm)
    end
    return worst
end

function get_cpu_model()
    try
        for line in eachline("/proc/cpuinfo")
            if startswith(line, "model name")
                return strip(split(line, ":")[2])
            end
        end
    catch
    end
    return "unknown"
end

function sha256_of_file(path)
    if isfile(path)
        return bytes2hex(open(sha256, path))
    end
    return nothing
end

function main()
    println("=== Starting Julia Comparator Benchmark (v2 Strict Provenance) ===")
    ref_file = nothing
    out_file = "bench/out/julia_bench_results.json"
    
    # Simple argument parsing
    i = 1
    while i <= length(ARGS)
        if ARGS[i] == "--reference" && i + 1 <= length(ARGS)
            ref_file = ARGS[i+1]
            i += 2
        elseif ARGS[i] == "--out" && i + 1 <= length(ARGS)
            out_file = ARGS[i+1]
            i += 2
        else
            i += 1
        end
    end

    ref_data = Dict{String, Any}()
    ref_sha256 = nothing
    if ref_file !== nothing && isfile(ref_file)
        println("Loading reference solution from: ", ref_file)
        ref_sha256 = sha256_of_file(ref_file)
        raw = JSON3.read(read(ref_file, String))
        for row in raw["references"]
            prob_id = String(row["problem"])
            ref_data[prob_id] = row["states"]
        end
        println("Loaded references for: ", keys(ref_data), " (SHA: ", ref_sha256[1:8], "...)")
    else
        println("No reference file provided or found; WRMS error will be omitted.")
    end

    cpu_model = get_cpu_model()
    kernel = try readchomp(`uname -sr`) catch; "unknown" end
    git_commit = try readchomp(`git rev-parse HEAD`) catch; "unknown" end
    runner_sha = sha256_of_file(@__FILE__)
    manifest_sha = sha256_of_file("tools/benchmarks/component_julia/Manifest.toml")

    provenance = Dict(
        "host_cpu" => cpu_model,
        "host_logical_cpus" => Sys.CPU_THREADS,
        "host_kernel" => kernel,
        "julia_version" => string(VERSION),
        "julia_manifest_sha256" => manifest_sha,
        "git_commit" => git_commit,
        "reference_file" => ref_file,
        "reference_sha256" => ref_sha256,
        "runner_source_sha256" => runner_sha,
        "timing_authority" => "diagnostic_only"
    )

    problems = get_problems()
    algorithms = [
        ("julia-rodas5p", Rodas5P()),
        ("julia-fbdf", FBDF()),
        ("julia-cvode-bdf", CVODE_BDF())
    ]
    rtols = [1e-4, 1e-6, 1e-8]
    grid_points = 101

    results = []

    for prob in problems
        println("\n>>> Benchmarking problem: ", prob.id, " (n = ", length(prob.u0), ")")
        t_span = prob.tspan
        grid = range(t_span[1], t_span[2], length=grid_points)
        prob_ode = ODEProblem(prob.ode_fun, copy(prob.u0), t_span)

        ref_states = get(ref_data, prob.id, nothing)

        for (alg_name, alg) in algorithms
            for rtol in rtols
                # Scalar atol matching vigilode corpus
                atol = rtol * prob.atol_scale[1]
                
                # Warmup run (2 runs matching corpus warmups)
                local sol
                try
                    solve(prob_ode, alg, reltol=rtol, abstol=atol, saveat=grid, maxiters=1000000)
                    sol = solve(prob_ode, alg, reltol=rtol, abstol=atol, saveat=grid, maxiters=1000000)
                catch e
                    println("  ", rtol, " ", alg_name, " Warmup FAILED: ", e)
                    push!(results, Dict(
                        "problem" => prob.id,
                        "algorithm" => alg_name,
                        "rtol" => rtol,
                        "status" => "Failed",
                        "error_message" => string(e)
                    ))
                    continue
                end

                if sol.retcode != SciMLBase.ReturnCode.Success
                    println("  ", rtol, " ", alg_name, " solver returned non-success retcode: ", sol.retcode)
                    push!(results, Dict(
                        "problem" => prob.id,
                        "algorithm" => alg_name,
                        "rtol" => rtol,
                        "status" => string(sol.retcode),
                        "time_median_ms" => nothing
                    ))
                    continue
                end

                # Timed repetitions (5 runs, retaining raw samples in ns)
                times_ns = Int64[]
                n_reps = 5
                for _ in 1:n_reps
                    t0 = time_ns()
                    sol = solve(prob_ode, alg, reltol=rtol, abstol=atol, saveat=grid, maxiters=1000000)
                    t1 = time_ns()
                    push!(times_ns, Int64(t1 - t0))
                end
                med_ns = median(times_ns)
                mad_ns = median(abs.(times_ns .- med_ns))
                med_time_ms = med_ns / 1e6

                # Extract states (101 points)
                states = [Vector{Float64}(sol.u[k]) for k in 1:length(sol.u)]
                err = nothing
                if ref_states !== nothing && length(states) == length(ref_states)
                    err = calc_max_wrms(states, ref_states, prob.atol_scale, 1.0)
                end

                stats = sol.stats
                println("  ", rtol, " ", rpad(alg_name, 18), ": Success | err: ", err !== nothing ? string(err) : "N/A", " | time: ", round(med_time_ms, digits=3), " ms | nf: ", stats.nf, " | njac: ", stats.njacs)

                push!(results, Dict(
                    "problem" => prob.id,
                    "algorithm" => alg_name,
                    "rtol" => rtol,
                    "status" => "Success",
                    "time_median_ms" => med_time_ms,
                    "raw_timing_samples_ns" => times_ns,
                    "timing_summary" => Dict(
                        "median_ns" => med_ns,
                        "mad_ns" => mad_ns,
                    ),
                    "error_common" => err,
                    "nf" => stats.nf,
                    "njacs" => stats.njacs,
                    "naccept" => stats.naccept,
                    "nreject" => stats.nreject,
                    "nsolve" => stats.nsolve
                ))
            end
        end
    end

    output_doc = Dict(
        "schema" => "vigilode.cross_solver_run.v2",
        "provenance" => provenance,
        "results" => results
    )

    # Write output JSON
    mkpath(dirname(out_file))
    open(out_file, "w") do io
        JSON3.pretty(io, output_doc)
    end
    println("\n=== Julia Benchmark completed. Saved to: ", out_file, " ===")
end

main()
