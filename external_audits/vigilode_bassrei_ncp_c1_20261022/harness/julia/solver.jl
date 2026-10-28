# Julia comparator server of the BASS/rei NCP C1 campaign.
#
#   julia --project=ENV solver.jl --server        (requests on stdin, one path per line)
#   julia solver.jl --parity REQUEST.json OUT.json  (RHS/Jacobian only; Base Julia)
#
# Server protocol: prints "READY <version> <solvers>" once the packages are
# loaded; then each stdin line is a request JSON path ("QUIT" ends). A request
# {"id", "kind": "solve"|"rhs", "model", ..., "output_path"} writes its result
# JSON to output_path and answers "DONE <id>" (or "FAIL <id> <msg>").
#
# Solvers (all with the ported RHS, its exact f_t as tgrad, vector abstol):
#   jl_rodas5p_sparse  Rodas5P, sparse Jacobian (coloured dual-number AD, exact
#                      for the linear P02 operator), jac_prototype = pattern
#   jl_fbdf_sparse     FBDF with the same Jacobian
#   jl_cvode_bdf_band  Sundials CVODE_BDF(:Band, jac_upper = jac_lower = nm+1),
#                      CVODE's own banded difference-quotient Jacobian
#   jl_cvode_bdf_klu   Sundials CVODE_BDF(:KLU) with the sparse Jacobian
#   jl_tsit5, jl_vern7 explicit (nonstiff cases)
# Every solve is preceded (once per server, solver and case) by an untimed
# warm-up solve over the first 1e-3 of the span at rtol 1e-3 with the same
# argument types (saveat vector, deadline callback), so the timed solve
# excludes compilation. Timing is diagnostic_only.

const HERE = @__DIR__
include(joinpath(HERE, "minijson.jl"))
include(joinpath(HERE, "kernels.jl"))
include(joinpath(HERE, "colorjac.jl"))
using .MiniJSON
using .Kernels
using .ColorJac
using SparseArrays

const SERVER = "--server" in ARGS

if SERVER
    using OrdinaryDiffEq
    using OrdinaryDiffEqBDF
    using SciMLBase
    const HAVE_SUNDIALS = try
        @eval using Sundials
        true
    catch e
        @warn "Sundials not loadable" exception = e
        false
    end
end

fvec(a) = Float64[Float64(x) for x in a]

struct Case
    model
    n::Int
    pattern_rows::Vector{Int}
    pattern_cols::Vector{Int}
    cj::ColoredJacobian
    proto::SparseMatrixCSC{Float64,Int}
end

const CASES = Dict{String,Case}()

function get_case(m::Dict)
    key = String(m["case"])
    haskey(CASES, key) && return CASES[key]
    M = build_model(m)
    n = nstate(M)
    n == Int(m["n"]) || error("state size $(n) != $(m["n"])")
    rows, cols = structural_pattern(M)
    cj = ColoredJacobian(n, rows, cols)
    proto = SparseMatrixCSC(n, n, copy(cj.colptr), copy(cj.rowval), zeros(length(cj.rowval)))
    c = Case(M, n, cj.rows, cj.cols, cj, proto)
    CASES[key] = c
    return c
end

f_of(c::Case) = (du, u, p, t) -> rhs!(du, u, c.model, t)
tg_of(c::Case) = (dT, u, p, t) -> tgrad!(dT, u, c.model, t)
function jac_of(c::Case)
    g! = (o, y, t) -> rhs!(o, y, c.model, t)
    return (J, u, p, t) -> (colored_jacobian!(nonzeros(J), g!, u, t, c.cj); nothing)
end

function rhs_request(req)
    c = get_case(req["model"])
    t = Float64(req["t"])
    out = Dict{String,Any}("id" => req["id"], "status" => "OK", "ncolors" => c.cj.ncolors,
                           "pattern_nnz" => length(c.cj.rows))
    rhss = Any[]
    for s in req["states"]
        y = fvec(s)
        o = zeros(c.n)
        rhs!(o, y, c.model, t)
        push!(rhss, o)
    end
    out["rhs"] = rhss
    if get(req, "jacobian", false) == true
        y = fvec(req["states"][1])
        nz = zeros(length(c.cj.rows))
        colored_jacobian!(nz, (o, yy, tt) -> rhs!(o, yy, c.model, tt), y, t, c.cj)
        out["jacobian"] = Dict("rows" => c.cj.rows, "cols" => c.cj.cols, "vals" => nz)
    end
    return out
end

if SERVER
    function algorithm(name::String, c::Case)
        bw = c.model.L.nm + 1
        if name == "jl_rodas5p_sparse"
            return Rodas5P(), :sparse
        elseif name == "jl_fbdf_sparse"
            return FBDF(), :sparse
        elseif name == "jl_cvode_bdf_band"
            HAVE_SUNDIALS || error("HOLD: Sundials not loadable")
            return CVODE_BDF(linear_solver=:Band, jac_upper=bw, jac_lower=bw), :plain
        elseif name == "jl_cvode_bdf_klu"
            HAVE_SUNDIALS || error("HOLD: Sundials not loadable")
            return CVODE_BDF(linear_solver=:KLU), :sparse
        elseif name == "jl_tsit5"
            return Tsit5(), :plain
        elseif name == "jl_vern7"
            return Vern7(), :plain
        end
        error("unknown solver $name")
    end

    function problem(c::Case, kind::Symbol, y0, tspan)
        f = kind === :sparse ?
            ODEFunction(f_of(c); jac=jac_of(c), jac_prototype=copy(c.proto), tgrad=tg_of(c)) :
            ODEFunction(f_of(c); tgrad=tg_of(c))
        return ODEProblem(f, y0, tspan)
    end

    function counters(sol)
        st = sol.stats
        d = Dict{String,Any}()
        for (k, nm) in (("nfev", :nf), ("njev", :njacs), ("nw", :nw), ("nsolve", :nsolve),
                        ("naccept", :naccept), ("nreject", :nreject), ("nnonliniter", :nnonliniter))
            d[k] = hasproperty(st, nm) ? Int(getproperty(st, nm)) : nothing
        end
        return d
    end

    const WARMED = Set{Tuple{String,String}}()

    # One closure type for the warm-up and the timed solves (same compiled integrator).
    deadline_callback(deadline::UInt64) =
        DiscreteCallback((u, t, integ) -> time_ns() > deadline, terminate!; save_positions=(false, false))

    function solve_request(req)
        c = get_case(req["model"])
        m = req["model"]
        name = String(req["solver"])
        alg, kind = algorithm(name, c)
        y0 = fvec(m["y0"])
        outs = fvec(m["outputs"])
        T = Float64(m["T"])
        atol = fvec(req["atol"])
        rtol = Float64(req["rtol"])
        out = Dict{String,Any}("id" => req["id"], "solver" => name, "rtol" => rtol,
                               "timing_authority" => "diagnostic_only", "julia_version" => string(VERSION))
        key = (String(m["case"]), name)
        if !(key in WARMED)
            # warm-up with the timed solve's own argument types (saveat vector, deadline callback),
            # so the first timed rung contains no compilation
            tw = time_ns()
            try
                pw = problem(c, kind, y0, (0.0, 1e-3 * T))
                solve(pw, alg; reltol=1e-3, abstol=atol .* 1e3, saveat=[0.0, 5e-4 * T, 1e-3 * T],
                      save_everystep=false, maxiters=10^5, callback=deadline_callback(time_ns() + 120_000_000_000))
            catch e
                out["warmup_error"] = sprint(showerror, e)[1:min(end, 500)]
            end
            out["warmup_seconds"] = (time_ns() - tw) * 1e-9
            push!(WARMED, key)
        end
        prob = problem(c, kind, y0, (0.0, T))
        deadline = time_ns() + round(UInt64, Float64(get(req, "timeout_s", 600)) * 1e9)
        cb = deadline_callback(deadline)
        t0 = time_ns()
        sol = solve(prob, alg; reltol=rtol, abstol=atol, saveat=outs, save_everystep=false,
                    maxiters=Int(get(req, "maxiters", 10^7)), callback=cb)
        out["solve_seconds"] = (time_ns() - t0) * 1e-9
        rc = string(sol.retcode)
        out["retcode"] = rc
        out["counters"] = counters(sol)
        out["times"] = collect(Float64, sol.t)
        out["states"] = [collect(Float64, u) for u in sol.u]
        if time_ns() > deadline && rc != "Success"
            out["status"] = "TIMEOUT"
        else
            out["status"] = rc == "Success" ? "OK" : "NUMERICAL_FAILURE"
        end
        out["jacobian"] = kind === :sparse ? "coloured dual-number AD, $(c.cj.ncolors) colours" :
                          (name == "jl_cvode_bdf_band" ? "CVODE banded difference quotient" : "none")
        return out
    end
end

function handle(path::AbstractString)
    req = jparse(read(path, String))
    id = string(get(req, "id", "?"))
    out = try
        k = req["kind"]
        if k == "rhs"
            rhs_request(req)
        elseif k == "solve" && SERVER
            solve_request(req)
        else
            Dict{String,Any}("id" => id, "status" => "REFUSED", "reason" => "unknown kind $k")
        end
    catch e
        msg = sprint(showerror, e)
        st = occursin("HOLD:", msg) ? "HOLD" : "ADAPTER_FAILURE"
        Dict{String,Any}("id" => id, "status" => st, "reason" => msg[1:min(end, 1500)])
    end
    op = String(req["output_path"])
    open(op * ".tmp", "w") do io
        jwrite(io, out)
    end
    mv(op * ".tmp", op; force=true)
    return id, out["status"]
end

if SERVER
    println("READY ", VERSION, " sundials=", HAVE_SUNDIALS)
    flush(stdout)
    for line in eachline(stdin)
        line = strip(line)
        line == "QUIT" && break
        isempty(line) && continue
        id, st = try
            handle(line)
        catch e
            ("?", "FAIL " * replace(sprint(showerror, e), '\n' => ' ')[1:min(end, 300)])
        end
        println(startswith(st, "FAIL") ? "$st" : "DONE $id $st")
        flush(stdout)
    end
elseif length(ARGS) >= 2 && ARGS[1] == "--parity"
    # the request file carries its own output_path (no temporary files)
    id, st = handle(ARGS[2])
    println("DONE $id $st")
end
