using JSON3
using OrdinaryDiffEq
using OrdinaryDiffEqBDF
using OrdinaryDiffEqFIRK
using SciMLBase

function rhs_for(c)
    model = String(c["model"])
    p = c["parameters"]
    if model == "diagonal_stiff"
        rates = Float64.(p["rates"])
        return (du,u,_,t)->(@inbounds for i in eachindex(u); du[i]=rates[i]*u[i]; end)
    elseif model == "logistic"
        rate=Float64(p["rate"]); cap=Float64(p["capacity"])
        return (du,u,_,t)->(du[1]=rate*u[1]*(1-u[1]/cap))
    elseif model == "rotation"
        w=Float64(p["omega"])
        return (du,u,_,t)->(du[1]=-w*u[2]; du[2]=w*u[1])
    elseif model == "damping"
        w=Float64(p["omega"]); z=Float64(p["zeta"])
        return (du,u,_,t)->(du[1]=u[2]; du[2]=-w*w*u[1]-2*z*w*u[2])
    elseif model == "nonnormal"
        a=Float64(p["a"]); b=Float64(p["b"]); d=Float64(p["d"])
        return (du,u,_,t)->(du[1]=a*u[1]+b*u[2]; du[2]=d*u[2])
    elseif model == "robertson"
        k1=Float64(p["k1"]); k2=Float64(p["k2"]); k3=Float64(p["k3"])
        return (du,u,_,t)->begin
            a=k1*u[1]; b=k2*u[2]*u[3]; z=k3*u[2]*u[2]
            du[1]=-a+b; du[2]=a-b-z; du[3]=z
        end
    elseif model == "vanderpol"
        mu=Float64(p["mu"])
        return (du,u,_,t)->(du[1]=u[2]; du[2]=mu*(1-u[1]^2)*u[2]-u[1])
    elseif model == "periodic_heat"
        d=Float64(p["diffusion"])
        return (du,u,_,t)->begin
            n=length(u); f=d*n*n
            @inbounds for i in 1:n
                du[i]=f*(u[mod1(i-1,n)]-2u[i]+u[mod1(i+1,n)])
            end
        end
    else
        error("unsupported model: "*model)
    end
end

function solveonce(c, alg)
    f=rhs_for(c)
    y0=Float64.(c["initial_state"])
    tspan=(Float64(c["t_span"][1]),Float64(c["t_span"][2]))
    prob=ODEProblem(f,y0,tspan)
    t0=time_ns()
    sol=solve(prob,alg;reltol=Float64(c["rtol"]),abstol=Float64.(c["atol"]),saveat=Float64.(c["output_times"]),save_everystep=false,maxiters=10^8)
    elapsed=(time_ns()-t0)*1e-9
    states=[Float64.(u) for u in sol.u]
    destats=sol.destats
    counters=Dict{String,Any}()
    for (key, candidates) in (("nfev",(:nf,:nf_evals)),("njev",(:njacs,:njac)),("nlu",(:nsolve,:nsolve_linear)),("accepted_steps",(:naccept,:naccepted)),("rejected_steps",(:nreject,:nrejected)))
        v=nothing
        for nm in candidates
            if hasproperty(destats,nm); v=Int(getproperty(destats,nm)); break; end
        end
        counters[key]=v
    end
    return (elapsed=elapsed,states=states,retcode=string(sol.retcode),counters=counters,solver_stats=string(destats))
end

function main()
    req=JSON3.read(read(ARGS[1],String))
    c=req["case"]; name=String(req["solver_id"]); nrep=Int(req["measurements"])
    alg=name=="Tsit5" ? Tsit5() : name=="Rodas5P" ? Rodas5P() : error("unknown algorithm")
    warm=[]
    for _ in 1:Int(req["warmups"]); push!(warm,solveonce(c,alg)); end
    rows=[]
    for i in 1:nrep
        r=solveonce(c,alg)
        push!(rows,Dict("repetition"=>i,"elapsed_seconds"=>r.elapsed,"retcode"=>r.retcode,"states"=>r.states,"counters"=>r.counters,"solver_stats"=>r.solver_stats))
    end
    JSON3.write(stdout,Dict("status"=>"OK","julia_version"=>string(VERSION),"algorithm"=>name,"warmup_seconds"=>[x.elapsed for x in warm],"measurements"=>rows))
    println()
end
main()
