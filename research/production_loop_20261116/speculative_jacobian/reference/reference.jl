# Julia references of loop node V07 (PREDECLARED.json, section 'reference'):
# runs B (OrdinaryDiffEqFIRK RadauIIA5) and C (OrdinaryDiffEq Rodas5P) of the
# reference uncertainty, Float64, reltol = 1e-13, abstol = ref_atol, default
# ForwardDiff Jacobian. The problem definitions are those of models.py, written
# a third time; a transcription error shows as a large disagreement with the
# SciPy reference, i.e. as a large stated uncertainty.
#
# Prints one `REFJL {json}` line per run; states as the hexadecimal IEEE-754
# bits of each component. Julia's own sin and exp may differ from the C library
# by one unit in the last place in the initial state of the Brusselator and in
# the constant D of the combustion problem (relative 1e-16).
#
# Run from the repository root:
#   JULIA_PKG_OFFLINE=true julia --startup-file=no --project=tools/benchmarks/component_julia \
#     research/production_loop_20261116/speculative_jacobian/reference/reference.jl \
#     > research/production_loop_20261116/speculative_jacobian/logs/reference_julia.log 2>&1

using OrdinaryDiffEq, OrdinaryDiffEqFIRK, SciMLBase, JSON3

hexbits(x::Float64) = string(reinterpret(UInt64, x), base = 16, pad = 16)

function robertson!(du, u, p, t)
    du[1] = -0.04 * u[1] + 1.0e4 * u[2] * u[3]
    du[2] = 0.04 * u[1] - 1.0e4 * u[2] * u[3] - 3.0e7 * u[2] * u[2]
    du[3] = 3.0e7 * u[2] * u[2]
    return nothing
end

function vdpol!(du, u, p, t)
    eps = 1.0e-6
    du[1] = u[2]
    du[2] = ((1.0 - u[1] * u[1]) * u[2] - u[1]) / eps
    return nothing
end

function oregonator!(du, u, p, t)
    s, q, w = 77.27, 8.375e-6, 0.161
    du[1] = s * (u[2] + u[1] * (1.0 - q * u[1] - u[2]))
    du[2] = (u[3] - (1.0 + u[1]) * u[2]) / s
    du[3] = w * (u[1] - u[3])
    return nothing
end

function hires!(du, u, p, t)
    du[1] = -1.71 * u[1] + 0.43 * u[2] + 8.32 * u[3] + 0.0007
    du[2] = 1.71 * u[1] - 8.75 * u[2]
    du[3] = -10.03 * u[3] + 0.43 * u[4] + 0.035 * u[5]
    du[4] = 8.32 * u[2] + 1.71 * u[3] - 1.12 * u[4]
    du[5] = -1.745 * u[5] + 0.43 * u[6] + 0.43 * u[7]
    du[6] = -280.0 * u[6] * u[8] + 0.69 * u[4] + 1.71 * u[5] - 0.43 * u[6] + 0.69 * u[7]
    du[7] = 280.0 * u[6] * u[8] - 1.81 * u[7]
    du[8] = -280.0 * u[6] * u[8] + 1.81 * u[7]
    return nothing
end

const POLLUTION_K = (
    0.35, 26.6, 12300.0, 8.6e-4, 8.2e-4, 15000.0, 1.3e-4, 24000.0, 16500.0, 9000.0, 0.022,
    12000.0, 1.88, 16300.0, 4.8e6, 3.5e-4, 0.0175, 1.0e8, 4.44e11, 1240.0, 2.1, 5.78, 0.0474,
    1780.0, 3.12,
)

function pollution!(du, y, p, t)
    k = POLLUTION_K
    r1 = k[1] * y[1]
    r2 = k[2] * y[2] * y[4]
    r3 = k[3] * y[5] * y[2]
    r4 = k[4] * y[7]
    r5 = k[5] * y[7]
    r6 = k[6] * y[7] * y[6]
    r7 = k[7] * y[9]
    r8 = k[8] * y[9] * y[6]
    r9 = k[9] * y[11] * y[2]
    r10 = k[10] * y[11] * y[1]
    r11 = k[11] * y[13]
    r12 = k[12] * y[10] * y[2]
    r13 = k[13] * y[14]
    r14 = k[14] * y[1] * y[6]
    r15 = k[15] * y[3]
    r16 = k[16] * y[4]
    r17 = k[17] * y[4]
    r18 = k[18] * y[16]
    r19 = k[19] * y[16]
    r20 = k[20] * y[17] * y[6]
    r21 = k[21] * y[19]
    r22 = k[22] * y[19]
    r23 = k[23] * y[1] * y[4]
    r24 = k[24] * y[19] * y[1]
    r25 = k[25] * y[20]
    du[1] = -r1 - r10 - r14 - r23 - r24 + r2 + r3 + r9 + r11 + r12 + r22 + r25
    du[2] = -r2 - r3 - r9 - r12 + r1 + r21
    du[3] = -r15 + r1 + r17 + r19 + r22
    du[4] = -r2 - r16 - r17 - r23 + r15
    du[5] = -r3 + 2.0 * r4 + r6 + r7 + r13 + r20
    du[6] = -r6 - r8 - r14 - r20 + r3 + 2.0 * r18
    du[7] = -r4 - r5 - r6 + r13
    du[8] = r4 + r5 + r6 + r7
    du[9] = -r7 - r8
    du[10] = -r12 + r7 + r9
    du[11] = -r9 - r10 + r8 + r11
    du[12] = r9
    du[13] = -r11 + r10
    du[14] = -r13 + r12
    du[15] = r14
    du[16] = -r18 - r19 + r16
    du[17] = -r20
    du[18] = r20
    du[19] = -r21 - r22 - r24 + r23 + r25
    du[20] = -r25 + r24
    return nothing
end

function pollution_y0()
    y0 = zeros(20)
    y0[2] = 0.2
    y0[4] = 0.04
    y0[7] = 0.1
    y0[8] = 0.3
    y0[9] = 0.01
    y0[17] = 0.007
    return y0
end

function e5!(du, u, p, t)
    a, b, c, m = 7.89e-10, 1.1e7, 1.13e3, 1.0e6
    mc = m * c
    f1 = -a * u[1] - b * u[1] * u[3]
    f2 = a * u[1] - mc * u[2] * u[3]
    f4 = b * u[1] * u[3] - c * u[4]
    du[1] = f1
    du[2] = f2
    du[3] = f2 - f4
    du[4] = f4
    return nothing
end

function brusselator_rhs(cells::Int)
    c = (cells + 1.0)^2 / 50.0
    function f!(du, u, p, t)
        for i in 1:cells
            ui = u[2i - 1]
            vi = u[2i]
            ul = i == 1 ? oftype(ui, 1.0) : u[2i - 3]
            vl = i == 1 ? oftype(vi, 3.0) : u[2i - 2]
            ur = i == cells ? oftype(ui, 1.0) : u[2i + 1]
            vr = i == cells ? oftype(vi, 3.0) : u[2i + 2]
            du[2i - 1] = 1.0 + ui * ui * vi - 4.0 * ui + c * (ul - 2.0 * ui + ur)
            du[2i] = 3.0 * ui - ui * ui * vi + c * (vl - 2.0 * vi + vr)
        end
        return nothing
    end
    return f!
end

function brusselator_y0(cells::Int)
    y0 = zeros(2 * cells)
    for i in 1:cells
        x = i / (cells + 1.0)
        y0[2i - 1] = 1.0 + sin(2.0 * pi * x)
        y0[2i] = 3.0
    end
    return y0
end

function combustion_rhs(cells::Int)
    alpha, delta, r = 1.0, 20.0, 5.0
    d = r * exp(delta) / (alpha * delta)
    inv = Float64(cells) * Float64(cells)
    function f!(du, u, p, t)
        for i in 1:cells
            left = i == 1 ? u[2] : u[i - 1]
            right = i == cells ? oftype(u[i], 1.0) : u[i + 1]
            du[i] = inv * (left - 2.0 * u[i] + right) +
                    d * (1.0 + alpha - u[i]) * exp(-delta / u[i])
        end
        return nothing
    end
    return f!
end

function nldiff_rhs(cells::Int)
    rate, u_left = 5.0, 1.0
    inv = Float64(cells) * Float64(cells)
    function f!(du, u, p, t)
        for i in 1:cells
            ul = i == 1 ? oftype(u[i], u_left) : u[i - 1]
            ur = i == cells ? u[i] : u[i + 1]
            wm = 0.5 * (ul + u[i])
            wp = 0.5 * (u[i] + ur)
            flux = wp * wp * (ur - u[i]) - wm * wm * (u[i] - ul)
            du[i] = inv * flux + rate * u[i] * (1.0 - u[i])
        end
        return nothing
    end
    return f!
end

# id, right-hand side, y0, t_end, ref_atol (1e-14 * atol_scale; e5: 1.7e-30)
const PROBLEMS = [
    ("robertson", robertson!, [1.0, 0.0, 0.0], 40.0, 1.0e-18),
    ("vdpol-eps1e-6", vdpol!, [2.0, 0.0], 2.0, 1.0e-14),
    ("oregonator", oregonator!, [1.0, 2.0, 3.0], 360.0, 1.0e-14),
    ("hires", hires!, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057], 321.8122, 1.0e-18),
    ("pollution", pollution!, pollution_y0(), 60.0, 1.0e-20),
    ("e5", e5!, [1.76e-3, 0.0, 0.0, 0.0], 1.0e13, 1.7e-30),
    ("brusselator-1d-32", brusselator_rhs(32), brusselator_y0(32), 10.0, 1.0e-14),
    ("brusselator-1d-100", brusselator_rhs(100), brusselator_y0(100), 10.0, 1.0e-14),
    ("combustion-100", combustion_rhs(100), ones(100), 0.27, 1.0e-14),
    ("nldiff-100", nldiff_rhs(100), fill(0.1, 100), 0.3, 1.0e-14),
]

function run(id, f!, y0, t_end, abstol, alg, label, method)
    start = time()
    record = Dict{String, Any}(
        "problem" => id, "n" => length(y0), "run" => label, "method" => method,
        "rtol" => 1.0e-13, "atol" => abstol, "t_end" => t_end,
    )
    try
        prob = ODEProblem(f!, copy(y0), (0.0, t_end))
        sol = solve(prob, alg; reltol = 1.0e-13, abstol = abstol, save_everystep = false,
                    maxiters = 10^8)
        record["success"] = SciMLBase.successful_retcode(sol)
        record["retcode"] = string(sol.retcode)
        record["t_last_bits"] = hexbits(Float64(sol.t[end]))
        record["y_bits"] = [hexbits(Float64(v)) for v in sol.u[end]]
        record["naccept"] = sol.stats.naccept
        record["nreject"] = sol.stats.nreject
    catch err
        record["success"] = false
        record["message"] = "exception: " * first(sprint(showerror, err), 300)
    end
    record["wall_seconds_diagnostic_only"] = round(time() - start, digits = 3)
    println("REFJL ", JSON3.write(record))
    flush(stdout)
end

function main()
    println("REFJL ", JSON3.write(Dict("kind" => "header", "julia" => string(VERSION),
                                       "y0_brusselator_32_first_bits" => hexbits(brusselator_y0(32)[1]))))
    flush(stdout)
    count = 0
    for (id, f!, y0, t_end, abstol) in PROBLEMS
        run(id, f!, y0, t_end, abstol, RadauIIA5(), "B", "OrdinaryDiffEqFIRK.RadauIIA5")
        run(id, f!, y0, t_end, abstol, Rodas5P(), "C", "OrdinaryDiffEq.Rodas5P")
        count += 2
    end
    println("REFJL ", JSON3.write(Dict("kind" => "footer", "runs" => count, "complete" => true)))
    flush(stdout)
end

main()
