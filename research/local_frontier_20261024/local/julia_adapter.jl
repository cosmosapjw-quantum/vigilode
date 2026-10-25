# Run only with an already installed, pinned project; this script never installs packages.
using JSON3, SciMLBase, OrdinaryDiffEqTsit5, OrdinaryDiffEqRosenbrock, LinearAlgebra
BLAS.set_num_threads(1)
length(ARGS)==5 || error("contract method rtol atol max_steps required")
c=JSON3.read(read(ARGS[1],String)); method=ARGS[2]; rtol=parse(Float64,ARGS[3]); atol=parse(Float64,ARGS[4]); maxsteps=parse(Int,ARGS[5])
method in ["julia-tsit5","julia-rodas5p"] || error("unsupported method")
Int(c.case_id) in [1,10,15,16] || error("unsupported model")
p=c.parameters
const CASE10_OMEGA_C = Int(c.case_id)==10 ? Float64(p.omega_c) : 0.0
function rhs!(du,y,_,t)
    id=Int(c.case_id)
    if id==1
        a=p.k1*y[1]; b=p.k2*y[2]*y[3]; z=p.k3*y[2]^2
        du[1]=-a+b; du[2]=a-b-z; du[3]=z
    elseif id==10
        g=sqrt(1+(abs2(y[4])+abs2(y[5])+abs2(y[6])))
        du[1]=y[4]/g; du[2]=y[5]/g; du[3]=y[6]/g
        du[4]=CASE10_OMEGA_C*y[5]/g; du[5]=-CASE10_OMEGA_C*y[4]/g; du[6]=0
    elseif id==15
        du[1]=y[2]; du[2]=-(2*p.gamma-p.beta)*y[2]-(p.omega0^2*exp(2*p.beta*t)+p.gamma^2-p.gamma*p.beta)*y[1]
    else
        r2=y[1]^2+y[2]^2; a=-p.alpha/r2^2; w=p.omega0/r2^.75
        du[1]=a*y[1]-w*y[2]; du[2]=a*y[2]+w*y[1]
    end
end
for probe in c.rhs_probes
    y=Float64.(probe.state); actual=similar(y); rhs!(actual,y,nothing,Float64(probe.t))
    all(abs.(actual.-Float64.(probe.rhs)).<=128eps(Float64).*max.(1.,abs.(actual),abs.(Float64.(probe.rhs)))) || error("RHS contract mismatch")
end
initial=Float64.(c.initial_state); tend=Float64(c.t_end)
algorithm=method=="julia-tsit5" ? Tsit5() : Rodas5P()
# Warm compilation uses this same model and a short subspan; it is explicitly excluded and reported.
warm_start=time_ns()
warm_prob=ODEProblem(rhs!,copy(initial),(0.,min(tend,1e-4)))
solve(warm_prob,algorithm;reltol=rtol,abstol=atol,maxiters=maxsteps,save_everystep=false,save_start=false,save_end=true,dense=false)
warm_seconds=(time_ns()-warm_start)*1e-9
start=time_ns(); prob=ODEProblem(rhs!,copy(initial),(0.,tend))
sol=solve(prob,algorithm;reltol=rtol,abstol=atol,maxiters=maxsteps,save_everystep=false,save_start=false,save_end=true,dense=false)
seconds=(time_ns()-start)*1e-9
state=Float64.(sol.u[end]); success=SciMLBase.successful_retcode(sol.retcode)&&all(isfinite,state)&&sol.t[end]==tend
reference=c.exact_final===nothing ? nothing : Float64.(c.exact_final)
error=success&&reference!==nothing ? maximum(abs.(state.-reference)) : nothing
status=!success ? "SOLVER_FAILED" : error===nothing ? "UNVALIDATED_REFERENCE" : error<=1e-4 ? "PASS_SCOPED" : "ACCURACY_FAILED"
println(JSON3.write(Dict("kind"=>"result","schema_version"=>1,"case"=>Int(c.case_id),"method"=>method,"size"=>String(c.size),"threads"=>1,"parallel_layer"=>"serial","elapsed_seconds"=>seconds,"warmup_seconds"=>warm_seconds,"solver_success"=>success,"scientific_status"=>status,"speed_comparison_eligible"=>status=="PASS_SCOPED","actual_error"=>error,"error_limit"=>1e-4,"final_state"=>all(isfinite,state) ? state : nothing,"final_time"=>sol.t[end],"julia_version"=>string(VERSION),"retcode"=>string(sol.retcode),"reference_provenance"=>"analytic endpoint from verified VigilODE model descriptor; not independent external reference","timing_scope"=>"fresh problem and solve after explicit compilation warmup; imports, warmup and JSON excluded","accuracy_scope"=>"maximum component absolute endpoint error only")))
