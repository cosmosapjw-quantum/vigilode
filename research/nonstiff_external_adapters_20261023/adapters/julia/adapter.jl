#!/usr/bin/env julia
using JSON3, SciMLBase, OrdinaryDiffEqTsit5, OrdinaryDiffEqRosenbrock
using LinearAlgebra, SHA
const ENDS=Dict("decay"=>4.,"driven"=>4.,"logistic"=>10.,"harmonic"=>12.,"rapid"=>2.,"damped"=>4.,"mixed-scales"=>4.,"nonnormal"=>4.,"large-vector"=>4.,"moderate-stiff"=>2.,"stiff-control"=>1.)
const DIMENSIONS=Dict("decay"=>8,"driven"=>8,"logistic"=>8,"harmonic"=>8,"rapid"=>8,"damped"=>8,"mixed-scales"=>12,"nonnormal"=>8,"moderate-stiff"=>12,"stiff-control"=>12)
const MODEL_SOURCE="45037fc24e8f8101605f309e1fdb3a9a4e3df56d"
mutable struct Parameters
    name::String
    rates::Vector{Float64}
    frequencies::Vector{Float64}
    attempted::Int
    completed::Int
    jacobians::Int
end
function setup(name,n)
    initial=[1/(1+mod(i,8)) for i in 0:n-1]
    rates=[[.25,1.,3.,.5][mod(i,4)+1] for i in 0:n-1]
    name=="logistic" && (initial=[.05+.01*mod(i,8) for i in 0:n-1]; rates=[.25+.1*mod(i,8) for i in 0:n-1])
    name=="mixed-scales" && (initial=[10.0^(mod(i,12)-8) for i in 0:n-1])
    name=="large-vector" && (rates=[.5+.05*mod(i,8) for i in 0:n-1])
    name=="moderate-stiff" && (rates=[[1.,30.,300.][mod(i,3)+1] for i in 0:n-1])
    name=="stiff-control" && (rates=[[1.,1000.,100000.][mod(i,3)+1] for i in 0:n-1])
    name=="nonnormal" && (initial=[Float64(mod(i,2)) for i in 0:n-1])
    name in ("harmonic","rapid","damped") && (initial=[1.0 - mod(i,2) for i in 0:n-1])
    base,scale=name=="rapid" ? (100.,5.) : name=="damped" ? (10.,1.) : (1.,.25)
    initial, Parameters(name,rates,[base+scale*mod(i,4) for i in 0:div(n,2)-1],0,0,0)
end
function rhs!(du,u,p,t)
    p.attempted+=1
    if p.name in ("harmonic","rapid","damped")
        d=p.name=="damped" ? 3. : 0.
        @inbounds for j in eachindex(p.frequencies)
            i=2j-1; w=p.frequencies[j]; du[i]=-d*u[i]-w*u[i+1];du[i+1]=w*u[i]-d*u[i+1]
        end
    elseif p.name=="nonnormal"
        @inbounds for i in 1:2:length(u);du[i]=-u[i]+30*u[i+1];du[i+1]=-2*u[i+1];end
    elseif p.name=="logistic"
        @inbounds for i in eachindex(u);du[i]=p.rates[i]*u[i]*(1-u[i]);end
    elseif p.name=="driven"
        @inbounds for i in eachindex(u);du[i]=-p.rates[i]*u[i]+cos(t)+p.rates[i]*sin(t);end
    else
        @inbounds for i in eachindex(u);du[i]=-p.rates[i]*u[i];end
    end
    p.completed+=1;nothing
end
function jac!(J,u,p,t)
    p.jacobians+=1;fill!(J,0.)
    @inbounds for i in eachindex(u);J[i,i]=-p.rates[i];end
    nothing
end
function exact(name,n,t)
    y,p=setup(name,n)
    if name=="logistic"; return 1 ./ (1 .+ (1 ./ y .- 1).*exp.(-p.rates*t))
    elseif name=="driven";return sin(t) .+ y.*exp.(-p.rates*t)
    elseif name in ("harmonic","rapid","damped")
        envelope=name=="damped" ? exp(-3*t) : 1.
        for j in eachindex(p.frequencies);y[2j-1]=envelope*cos(p.frequencies[j]*t);y[2j]=envelope*sin(p.frequencies[j]*t);end
        return y
    elseif name=="nonnormal"
        for i in 1:2:n;y[i]=30*(exp(-t)-exp(-2*t));y[i+1]=exp(-2*t);end
        return y
    end
    y.*exp.(-p.rates*t)
end
function integrate(name,n,rtol,atol,maxiters)
    started=time_ns()
    y,p=setup(name,n);targets=collect(range(0.,ENDS[name],length=5));stiff=name in ("moderate-stiff","stiff-control")
    f=stiff ? ODEFunction(rhs!;jac=jac!,jac_prototype=zeros(n,n)) : ODEFunction(rhs!)
    accepted_state=copy(y);accepted_time=Ref(0.)
    snapshot_condition(u,t,integ)=true
    function snapshot!(integ)
        copyto!(accepted_state,integ.u);accepted_time[]=Float64(integ.t)
        SciMLBase.u_modified!(integ,false)
        nothing
    end
    observer=DiscreteCallback(snapshot_condition,snapshot!;save_positions=(false,false))
    prob=ODEProblem(f,y,(0.,ENDS[name]),p)
    algorithm=stiff ? Rodas5P() : Tsit5()
    integ=nothing;sol=nothing;message="success";ok=false
    try
        integ=init(prob,algorithm;reltol=rtol,abstol=atol,dt=1e-3,maxiters=maxiters,saveat=targets,save_everystep=false,dense=false,callback=observer)
        sol=solve!(integ);ok=SciMLBase.successful_retcode(sol.retcode);message=string(sol.retcode)
    catch err
        message=sprint(showerror,err);integ!==nothing && (sol=integ.sol)
    end
    times=sol===nothing ? [0.] : Float64.(sol.t)
    states=sol===nothing ? [copy(y)] : [copy(u) for u in sol.u]
    lasttime=accepted_time[]
    laststate=copy(accepted_state)
    elapsed=(time_ns()-started)/1e9
    (;elapsed,p,times,states,lasttime,laststate,message,ok=ok && times==targets && all(u->all(isfinite,u),states),sol)
end
filehash(path)=bytes2hex(sha256(read(path)))
function main()
    VERSION == v"1.12.7" || error("Julia 1.12.7 required")
    req=JSON3.read(read(stdin,String),Dict{String,Any});name=String(req["case"]);n=Int(req["dimension"]);rtol=Float64(req["rtol"]);atol=Float64(req["atol"]);steps=Int(get(req,"max_steps",100000))
    haskey(ENDS,name) || error("unknown case")
    n>0 || error("dimension must be positive")
    get(req,"mode","julia")=="julia" || error("mode must be julia")
    (name=="large-vector" ? n in (4096,16384) : n==DIMENSIONS[name]) || error("unregistered case/dimension pair")
    name in ("harmonic","rapid","damped","nonnormal") && isodd(n) && error("paired family requires even dimension")
    name in ("moderate-stiff","stiff-control") && n!=12 && error("stiff contract dimension must be 12")
    isfinite(rtol)&&isfinite(atol)&&rtol>0&&atol>0 || error("tolerances must be positive finite")
    steps>0 || error("max_steps must be positive")
    req["source_commit"]==MODEL_SOURCE || error("model source identity mismatch")
    actual_identity=get(ENV,"VIGILODE_ADAPTER_SOURCE_COMMIT","UNCOMMITTED_DEVELOPMENT")
    haskey(req,"adapter_source_commit") && req["adapter_source_commit"]!=actual_identity && error("adapter source identity mismatch")
    prestart=time_ns();prewarm=integrate(name,n,rtol,atol,steps);preseconds=(time_ns()-prestart)/1e9
    run=integrate(name,n,rtol,atol,steps)
    references=[exact(name,n,t) for t in run.times];initial=exact(name,n,0.)
    errors=[abs.(u-r) for (u,r) in zip(run.states,references)]
    scales=[max.(1.,max.(abs.(initial),abs.(r))) for r in references]
    finite=all(u->all(isfinite,u),run.states)
    stats=run.sol===nothing ? nothing : run.sol.stats
    getstat(k)=stats!==nothing && hasproperty(stats,k) ? getproperty(stats,k) : nothing
    work=Dict("rhs_attempted"=>run.p.attempted,"rhs_completed"=>run.p.completed,"analytic_jacobian_calls"=>run.p.jacobians,"solver_stats"=>Dict(string(k)=>getstat(k) for k in (:nf,:nf2,:nw,:nsolve,:njacs,:nnonliniter,:nnonlinconvfail,:naccept,:nreject)),"hot_loop_allocations"=>nothing)
    adapter=Dict("solver"=>"OrdinaryDiffEq","method"=>(name in ("moderate-stiff","stiff-control") ? "Rodas5P" : "Tsit5"),"julia_version"=>string(VERSION),"package_versions"=>Dict("JSON3"=>string(pkgversion(JSON3)),"SciMLBase"=>string(pkgversion(SciMLBase)),"OrdinaryDiffEqTsit5"=>string(pkgversion(OrdinaryDiffEqTsit5)),"OrdinaryDiffEqRosenbrock"=>string(pkgversion(OrdinaryDiffEqRosenbrock))),"jacobian_policy"=>"analytic diagonal dense Jacobian only for dimension-12 stiff controls; no Jacobian for Tsit5","linear_solver_policy"=>"Rodas5P package default linear solver; explicit Tsit5 has none","autodiff_policy"=>"Rodas5P default AutoForwardDiff with supplied analytic state Jacobian; package may differentiate time dependence; Tsit5 uses no differentiation","initial_step"=>1e-3,"accepted_state_policy"=>"preallocated state/time snapshot via observation-only DiscreteCallback after accepted steps, no state mutation, save_positions=false, u_modified=false; setup/copy costs included in timer, one additional O(n) buffer","step_cap_policy"=>"package maxiters bounds total attempted steps","julia_threads"=>Threads.nthreads(),"blas_threads"=>BLAS.get_num_threads(),"output_policy"=>"one solve history, saveat interpolation at five target times; dense history disabled","jit_policy"=>"one complete same-request method/type/shape preparation solve per process; fresh measured problem/workspace/state/counters; preparation may fail and measured attempt still runs","preparation_seconds"=>preseconds,"preparation_success"=>prewarm.ok,"preparation_message"=>prewarm.message,"adapter_source_commit"=>get(ENV,"VIGILODE_ADAPTER_SOURCE_COMMIT","UNCOMMITTED_DEVELOPMENT"),"adapter_source_sha256"=>filehash(@__FILE__),"project_sha256"=>filehash(joinpath(@__DIR__,"Project.toml")),"manifest_sha256"=>filehash(joinpath(@__DIR__,"Manifest.toml")),"executable"=>joinpath(Sys.BINDIR,Base.julia_exename()),"executable_sha256"=>filehash(joinpath(Sys.BINDIR,Base.julia_exename())))
    record=merge(req,Dict("schema"=>"vigilode.nonstiff-benchmark-run.v1","adapter_source_commit"=>actual_identity,"mode"=>"julia","end"=>ENDS[name],"success"=>(run.ok && prewarm.ok),"message"=>run.message,"elapsed_seconds"=>run.elapsed,"requested_times"=>collect(range(0.,ENDS[name],length=5)),"times"=>run.times,"states"=>run.states,"reference"=>references,"max_abs_error"=>(finite ? maximum(maximum(e) for e in errors) : nothing),"max_normalized_error"=>(finite ? maximum(maximum(e./s) for (e,s) in zip(errors,scales)) : nothing),"max_reference_scaled_wrms"=>(finite ? maximum(sqrt(sum(abs2,e./(atol .+rtol.*abs.(r)))/n) for (e,r) in zip(errors,references)) : nothing),"last_accepted_time"=>run.lasttime,"last_accepted_state"=>run.laststate,"work"=>work,"adapter"=>adapter,"timing_authority"=>"diagnostic_only","timing_scope"=>"fresh initial data, parameter/problem/workspace construction, solve and callbacks, rejections, saved-output and per-accepted-step snapshot collection; imports/JIT preparation/reference/errors/JSON excluded"))
    JSON3.write(stdout,record);println()
end
abspath(PROGRAM_FILE) == (@__FILE__) && main()
