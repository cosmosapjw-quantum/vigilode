# Independent endpoint comparator on the frozen U11 holdouts.
# The project and its manifest are checked in under tools/benchmarks/component_julia.
using OrdinaryDiffEq, SciMLBase, JSON3

function hires!(d, y, _, t)
    d[1] = -1.71y[1] + 0.43y[2] + 8.32y[3] + 0.0007
    d[2] = 1.71y[1] - 8.75y[2]
    d[3] = -10.03y[3] + 0.43y[4] + 0.035y[5]
    d[4] = 8.32y[2] + 1.71y[3] - 1.12y[4]
    d[5] = -1.745y[5] + 0.43y[6] + 0.43y[7]
    d[6] = -280y[6]*y[8] + 0.69y[4] + 1.71y[5] - 0.43y[6] + 0.69y[7]
    d[7] = 280y[6]*y[8] - 1.81y[7]
    d[8] = -d[7]
    nothing
end
function oregonator!(d, y, _, t)
    d[1] = 77.27 * (y[2] + y[1]*(1 - 8.375e-6*y[1] - y[2]))
    d[2] = (y[3] - (1+y[1])*y[2])/77.27
    d[3] = 0.161*(y[1]-y[3])
    nothing
end
function bruss!(d, y, _, t)
    n = 32
    c = (n+1)^2/50
    for i in 1:n
        u, v = y[i], y[i+n]
        ul, ur = i == 1 ? 1. : y[i-1], i == n ? 1. : y[i+1]
        vl, vr = i == 1 ? 3. : y[i+n-1], i == n ? 3. : y[i+n+1]
        d[i] = 1 + u*u*v - 4u + c*(ul-2u+ur)
        d[i+n] = 3u - u*u*v + c*(vl-2v+vr)
    end
    nothing
end
models = [
    ("hires",hires!, [1.,0.,0.,0.,0.,0.,0.,0.0057],321.8122),
    ("oregonator",oregonator!,[1.,2.,3.],360.),
    ("brusselator_mol32",bruss!,vcat([1+sin(2pi*i/33) for i in 1:32],fill(3.,32)),10.)
]
for (name, f, y0, stop) in models, tol in (1e-4,1e-5,1e-6,1e-7,1e-8,1e-13)
    sol = solve(ODEProblem(f,y0,(0.,stop)),Rodas5P();reltol=tol,abstol=0.01*tol,
                save_everystep=false,maxiters=1000000)
    @assert SciMLBase.successful_retcode(sol) && sol.t[end] == stop
    println(JSON3.write((problem=name,rtol=tol,arm="julia_rodas5p",version=string(VERSION),
             result=(accepted=sol.stats.naccept,rejected=sol.stats.nreject,y_end=sol.u[end]))))
end
