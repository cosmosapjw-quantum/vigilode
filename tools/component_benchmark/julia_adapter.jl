#!/usr/bin/env julia
# Exact shared JSON request/response. Install JSON3, SciMLBase, OrdinaryDiffEq,
# and Sundials in an isolated, frozen Julia project before performance use.
packages="--probe" in ARGS ? ("JSON3",) :
    ("JSON3","SciMLBase","OrdinaryDiffEq","OrdinaryDiffEqBDF","OrdinaryDiffEqFIRK","Sundials")
for package in packages
    if Base.find_package(package)!==nothing
        @eval import $(Symbol(package))
    end
end
function response(status, reason="")
    d=Dict{String,Any}("status"=>status,"times"=>Any[],"states"=>Any[],"counters"=>nothing,
        "timing"=>Dict("solve_seconds"=>nothing,"total_seconds"=>nothing,"timing_authority"=>"diagnostic_only"),"allocations"=>nothing)
    if !isempty(reason); d["reason"]=reason; end
    return d
end
function emit(d)
    # JSON3 may be unavailable before request parsing. Give a valid protocol response.
    try
        println(JSON3.write(d))
    catch
        println("{\"status\":\"ENVIRONMENT_FAILURE\",\"times\":[],\"states\":[],\"counters\":null,\"timing\":{\"solve_seconds\":null,\"total_seconds\":null,\"timing_authority\":\"diagnostic_only\"},\"allocations\":null,\"reason\":\"JSON3 unavailable\"}")
    end
end
function rhs!(dy,y,p,t,m)
    if m=="scalar_decay" || m=="sabr"; dy[1]=get(p,"rate",get(p,"lambda",0.0))*y[1]
    elseif m=="logistic"; dy[1]=p["rate"]*y[1]*(1-y[1]/p["capacity"])
    elseif m=="kepler"
        r2=y[1]^2+y[2]^2; a=-p["mu"]/(r2*sqrt(r2)); dy[1]=y[3];dy[2]=y[4];dy[3]=a*y[1];dy[4]=a*y[2]
    elseif m=="robertson" || m=="robertson_constraint"
        a=y[1];b=y[2];c=y[3];u=p["k1"]*a;v=p["k2"]*b*c;w=p["k3"]*b*b
        dy[1]=-u+v;dy[2]=u-v-w;dy[3]=m=="robertson" ? w : a+b+c-1
    elseif m=="vanderpol"; dy[1]=y[2];dy[2]=p["mu"]*(1-y[1]^2)*y[2]-y[1]
    elseif m=="hires"
        a=y[1];b=y[2];c=y[3];d=y[4];e=y[5];f=y[6];g=y[7];h=y[8];q=280*f*h
        dy[1]=-1.71*a+.43*b+8.32*c+.0007;dy[2]=1.71*a-8.75*b
        dy[3]=-10.03*c+.43*d+.035*e;dy[4]=8.32*b+1.71*c-1.12*d
        dy[5]=-1.745*e+.43*f+.43*g;dy[6]=-q+.69*d+1.71*e-.43*f+.69*g
        dy[7]=q-1.81*g;dy[8]=-q+1.81*g
    elseif m=="pr_linear" || m=="pr_nonlinear"
        d=y[1]-sin(t);dy[1]=p["lambda"]*d+cos(t)+p["beta"]*d*d
    elseif m=="rotation";dy[1]=-p["omega"]*y[2];dy[2]=p["omega"]*y[1]
    elseif m=="damping";dy[1]=y[2];dy[2]=-2*p["zeta"]*p["omega"]*y[2]-p["omega"]^2*y[1]
    elseif m=="diagonal_stiff";for i in eachindex(y);dy[i]=p["rates"][i]*y[i];end
    elseif m=="jordan";dy[1]=p["lambda"]*y[1]+p["offdiag"]*y[2];dy[2]=p["lambda"]*y[2]
    elseif m=="nonnormal";dy[1]=p["a"]*y[1]+p["b"]*y[2];dy[2]=p["d"]*y[2]
    elseif m=="periodic_heat" || m=="advection_diffusion"
        n=Int(p["cells"]); d=p["diffusion"]*n*n; v=get(p,"velocity",0.0)
        for i in 1:n; l=mod1(i-1,n);r=mod1(i+1,n);dy[i]=d*(y[l]-2*y[i]+y[r])-v*n*(y[i]-y[l]);end
    elseif m=="brusselator_1d"
        n=Int(p["cells"]);c=p["diffusion_factor"]
        for i in 1:n
            u=y[2i-1];v=y[2i];ul=i==1 ? p["boundary_u"] : y[2i-3];vl=i==1 ? p["boundary_v"] : y[2i-2]
            ur=i==n ? p["boundary_u"] : y[2i+1];vr=i==n ? p["boundary_v"] : y[2i+2]
            dy[2i-1]=1+u*u*v-4*u+c*(ul-2u+ur);dy[2i]=3u-u*u*v+c*(vl-2v+vr)
        end
    elseif m=="gray_scott_2d"
        n=Int(p["grid"]);n2=n*n
        for j in 1:n,i in 1:n
            k=i+(j-1)*n; l=mod1(i-1,n)+(j-1)*n;r=mod1(i+1,n)+(j-1)*n
            up=i+(mod1(j+1,n)-1)*n;down=i+(mod1(j-1,n)-1)*n
            u=y[k];v=y[n2+k];uv2=u*v*v
            lu=(y[l]+y[r]+y[up]+y[down]-4u)*n*n
            lv=(y[n2+l]+y[n2+r]+y[n2+up]+y[n2+down]-4v)*n*n
            dy[k]=p["du"]*lu-uv2+p["feed"]*(1-u)
            dy[n2+k]=p["dv"]*lv+uv2-(p["feed"]+p["kill"])*v
        end
    elseif m=="mass_invertible";dy[1]=-2y[1]-y[2];dy[2]=-3y[2]
    elseif m=="mass_ill_conditioned";dy[1]=-y[1];dy[2]=-p["epsilon"]*y[2]
    elseif m=="dae_index1" || m=="dae_index1_inconsistent"
        dy[1]=-y[1]+y[2];dy[2]=y[1]+y[2]-(2sin(t)+cos(t))
    elseif m=="pendulum_index3"
        x=y[1];z=y[2];vx=y[3];vz=y[4];lam=y[5]
        dy[1]=vx;dy[2]=vz;dy[3]=-lam*x;dy[4]=-p["g"]-lam*z;dy[5]=x*x+z*z-p["length"]^2
    elseif m=="fourier_path"
        a=complex(y[1],y[2]);b=complex(y[3],y[4]);phase=cis(p["omega"]*t)
        c=p["g"]+p["eps"]*real(phase*a);q=p["sigma"]*1.25
        da=im*q*c*conj(a)*b;db=0.5im*q*c*a*a
        dy[1]=real(da);dy[2]=imag(da);dy[3]=real(db);dy[4]=imag(db)
    else; error("unsupported model "*m)
    end
end
function dae_residual!(res,du,u,p,t,m)
    if m=="mass_invertible"
        res[1]=2du[1]+du[2]+2u[1]+u[2];res[2]=3du[2]+3u[2]
    elseif m=="mass_ill_conditioned"
        e=p["epsilon"];res[1]=du[1]+u[1];res[2]=e*(du[2]+u[2])
    elseif m=="dae_index1" || m=="dae_index1_inconsistent"
        res[1]=du[1]+u[1]-u[2];res[2]=-u[1]-u[2]+2sin(t)+cos(t)
    elseif m=="robertson_constraint"
        a=u[1];b=u[2];c=u[3];v=p["k2"]*b*c;w=p["k3"]*b*b
        res[1]=du[1]+p["k1"]*a-v
        res[2]=du[2]-p["k1"]*a+v+w
        res[3]=1-a-b-c
    elseif m=="pendulum_index3"
        x=u[1];z=u[2];vx=u[3];vz=u[4];lam=u[5]
        res[1]=du[1]-vx;res[2]=du[2]-vz
        res[3]=du[3]+lam*x;res[4]=du[4]+p["g"]+lam*z
        res[5]=p["length"]^2-x*x-z*z
    else
        error("unsupported DAE residual "*m)
    end
end
function probe(req)
    c=req["case"];m=String(c["model"]);p=c["parameters"]
    y0=Float64.(c["initial_state"]);n=length(y0);t0=Float64(c["t_span"][1]);t1=Float64(c["t_span"][2])
    direction=[0.5+0.01*(i-1) for i in 1:n]
    points=haskey(req,"probes") ?
        [(Float64(v["time"]),Float64.(v["state"]),Float64.(v["direction"])) for v in req["probes"]] :
        [(t0,y0,direction),(t0+0.37*(t1-t0),[y0[i]+0.01*i for i in 1:n],direction)]
    rows=Any[]
    try
        for (t,y,direction) in points
            if length(y)!=n || length(direction)!=n || !isfinite(t) || t<t0 || t>t1 ||
               any(x->!isfinite(x),y) || any(x->!isfinite(x),direction)
                error("invalid frozen probe values")
            end
            f=zeros(n);rhs!(f,y,p,t,m)
            yp=copy(y);ym=copy(y);fp=zeros(n);fm=zeros(n)
            indices=n>512 ? [1,div(n,2)+1,n] : collect(1:n)
            columns=Vector{Vector{Float64}}();fd_columns=Any[]
            for j in indices
                nominal=eps(Float64)^(1/3)*max(1.0,abs(y[j]))
                yp[j]=y[j]+nominal;ym[j]=y[j]-nominal
                h=(yp[j]-ym[j])/2
                plus=yp[j];minus=ym[j]
                rhs!(fp,yp,p,t,m);rhs!(fm,ym,p,t,m)
                push!(columns,[(fp[i]-fm[i])/(2h) for i in 1:n])
                push!(fd_columns,Dict("index"=>j-1,"h"=>h,"plus"=>plus,"minus"=>minus,"denominator"=>2h))
                yp[j]=y[j];ym[j]=y[j]
            end
            if n>512
                h=eps(Float64)^(1/3)*max(1.0,maximum(abs,y))/max(1.0,maximum(abs,direction))
                for i in 1:n;yp[i]=y[i]+h*direction[i];ym[i]=y[i]-h*direction[i];end
                plus_state=copy(yp);minus_state=copy(ym)
                rhs!(fp,yp,p,t,m);rhs!(fm,ym,p,t,m)
                jvp=[(fp[i]-fm[i])/(2h) for i in 1:n]
                fd_directional=Dict("h"=>h,"plus_state"=>plus_state,"minus_state"=>minus_state,"denominator"=>2h)
                jac=nothing;column_indices=[i-1 for i in indices];jac_columns=columns
            else
                jvp=[sum(columns[j][i]*direction[j] for j in 1:n) for i in 1:n]
                jac=[[columns[j][i] for j in 1:n] for i in 1:n]
                column_indices=nothing;jac_columns=nothing;fd_directional=nothing
            end
            push!(rows,Dict("time"=>t,"state"=>y,"direction"=>direction,"rhs"=>f,
                "jacobian"=>jac,"jacobian_column_indices"=>column_indices,
                "jacobian_columns"=>jac_columns,"jvp"=>jvp,
                "fd_columns"=>fd_columns,"fd_directional"=>fd_directional))
        end
        return Dict("status"=>"OK","case_id"=>String(c["case_id"]),"probes"=>rows,
            "derivative_mode"=>"finite_difference_probe_solver_internal_autodiff",
            "rhs_semantics"=>"original_f_with_algebraic_rows_for_DAE")
    catch e
        return Dict("status"=>"UNSUPPORTED_CAPABILITY","case_id"=>String(c["case_id"]),"reason"=>sprint(showerror,e))
    end
end
function run(req)
    c=req["case"];sid=String(req["solver_id"]);m=String(c["model"])
    methods=Dict("julia-tsit5"=>"Tsit5","julia-vern9"=>"Vern9","julia-rodas5p"=>"Rodas5P",
        "julia-radauiia5"=>"RadauIIA5","julia-fbdf"=>"FBDF","julia-dfbdf"=>"DFBDF","julia-ida"=>"IDA",
        "julia-cvode-bdf"=>"CVODE_BDF")
    if !haskey(methods,sid);return response("UNSUPPORTED_CAPABILITY","unknown Julia solver_id");end
    mass=c["mass_matrix"]
    if mass!==nothing && !(sid in ("julia-ida","julia-dfbdf","julia-rodas5p","julia-radauiia5","julia-fbdf")) && !(m in ("mass_invertible","mass_ill_conditioned"))
        return response("UNSUPPORTED_CAPABILITY","solver has no declared singular mass capability")
    end
    if sid=="julia-ida" && mass===nothing;return response("UNSUPPORTED_CAPABILITY","IDA requires a mass/DAE case");end
    if sid=="julia-cvode-bdf" && mass!==nothing;return response("UNSUPPORTED_CAPABILITY","CVODE_BDF handles ODE cases only");end
    total_start=time_ns()
    try
        required=sid in ("julia-ida","julia-cvode-bdf") ?
            ("SciMLBase","OrdinaryDiffEq","Sundials") :
            sid=="julia-dfbdf" ? ("SciMLBase","OrdinaryDiffEqBDF") :
            sid=="julia-radauiia5" ? ("SciMLBase","OrdinaryDiffEqFIRK") :
            ("SciMLBase","OrdinaryDiffEq")
        for package in required
            if Base.find_package(package)===nothing
                return response("ENVIRONMENT_FAILURE",package*" unavailable in active Julia project")
            end
        end
        method=methods[sid]
        mod=sid in ("julia-ida","julia-cvode-bdf") ? Sundials :
            sid=="julia-dfbdf" ? OrdinaryDiffEqBDF :
            sid=="julia-radauiia5" ? OrdinaryDiffEqFIRK : OrdinaryDiffEq
        if !isdefined(mod,Symbol(method));return response("UNSUPPORTED_CAPABILITY",method*" not installed in this Julia environment");end
        y0=Float64.(c["initial_state"]);tspan=(Float64(c["t_span"][1]),Float64(c["t_span"][2]));p=c["parameters"]
        grid=Float64.(c["output_times"]);atol=Float64.(c["atol"])
        f! = (du,u,pp,t)->rhs!(du,u,p,t,m)
        if mass!==nothing
            if !haskey(c,"initial_derivative") || length(c["initial_derivative"])!=length(y0)
                return response("ENVIRONMENT_FAILURE","missing initial derivative contract")
            end
            check=zeros(length(y0))
            dae_residual!(check,Float64.(c["initial_derivative"]),y0,p,tspan[1],m)
            if any(i->!isfinite(check[i]) || abs(check[i])>atol[i],eachindex(check))
                return response("NUMERICAL_FAILURE","initial DAE residual violates frozen atol; initial state not repaired")
            end
        end
        function one_run()
            setup_start=time_ns()
            solver=getfield(mod,Symbol(method))()
            if sid in ("julia-ida","julia-dfbdf")
                M=reduce(vcat,(reshape(Float64.(row),1,:) for row in mass))
                du0=Float64.(c["initial_derivative"])
                if length(du0)!=length(y0)
                    error("missing initial derivative contract")
                end
                check=zeros(length(y0))
                res! = (res,du,u,pp,t)->dae_residual!(res,du,u,p,t,m)
                res!(check,du0,y0,nothing,tspan[1])
                if any(i->!isfinite(check[i]) || abs(check[i])>atol[i],eachindex(check))
                    error("initial DAE residual violates frozen atol; initial state not repaired")
                end
                prob=SciMLBase.DAEProblem(res!,du0,y0,tspan;
                    differential_vars=[any(abs(Float64(v))>0 for v in row) for row in mass])
            else
                if mass===nothing
                    prob=SciMLBase.ODEProblem(f!,y0,tspan)
                else
                    M=reduce(vcat,(reshape(Float64.(row),1,:) for row in mass))
                    prob=SciMLBase.ODEProblem(SciMLBase.ODEFunction(f!;mass_matrix=M),y0,tspan)
                end
            end
            solve_start=time_ns()
            # FIRK's scalar cache requires scalar abstol. All current CaseSpecs
            # use exactly equal component tolerances, so this preserves budgets.
            method_atol=sid=="julia-radauiia5" && all(x->x==atol[1],atol) ? atol[1] : atol
            options=(reltol=Float64(c["rtol"]),abstol=method_atol,saveat=grid,save_start=true)
            sol=mass!==nothing && !(m in ("mass_invertible","mass_ill_conditioned")) ?
                SciMLBase.solve(prob,solver;options...,initializealg=SciMLBase.CheckInit()) :
                sid in ("julia-ida","julia-dfbdf") ?
                SciMLBase.solve(prob,solver;options...,initializealg=SciMLBase.CheckInit()) :
                SciMLBase.solve(prob,solver;options...)
            solve_elapsed=max((time_ns()-solve_start)/1e9,eps(Float64))
            if !SciMLBase.successful_retcode(sol.retcode)
                error("solver retcode "*string(sol.retcode))
            end
            states=[Float64.(collect(u)) for u in sol.u]
            if length(states)!=length(grid) || any(x->!isfinite(x),Iterators.flatten(states))
                error("missing/nonfinite output")
            end
            return states,solve_elapsed,max((time_ns()-setup_start)/1e9,eps(Float64))
        end
        _,_,warmup=one_run()
        states,elapsed,timed=one_run()
        return Dict{String,Any}("status"=>"OK","times"=>grid,"states"=>states,
            "counters"=>Dict("rhs_evaluations"=>nothing,"jacobian_evaluations"=>nothing,"lu_factorizations"=>nothing,"accepted_steps"=>nothing,"rejected_steps"=>nothing),
            "timing"=>Dict("solve_seconds"=>elapsed,"timed_seconds"=>timed,"warmup_seconds"=>warmup,
                "total_seconds"=>max((time_ns()-total_start)/1e9,eps(Float64)),"timing_authority"=>"diagnostic_only"),
            "timing_scope"=>"warmed_solver_setup_integration_output_certificate","measurement_mode"=>"timing",
            "allocations"=>nothing,"solver_id"=>sid,
            "atol_handling"=>(sid=="julia-radauiia5" ? "equal_vector_collapsed_to_scalar" : "component_vector"),
            "wrapper_lane"=>(sid=="julia-cvode-bdf" ? "Julia API wrapping Sundials CVODE_BDF" : nothing))
    catch e
        if e isa LoadError || e isa ArgumentError && occursin("Package",sprint(showerror,e))
            return response("ENVIRONMENT_FAILURE",sprint(showerror,e))
        end
        return response("NUMERICAL_FAILURE",sprint(showerror,e))
    end
end
function main()
    idx=findfirst(==("--request"),ARGS)
    if idx===nothing || idx==length(ARGS);emit(response("ENVIRONMENT_FAILURE","--request PATH required"));return;end
    try
        req=JSON3.read(read(ARGS[idx+1],String),Dict{String,Any})
        emit("--probe" in ARGS ? probe(req) : run(req))
    catch e
        emit(response("ENVIRONMENT_FAILURE",sprint(showerror,e)))
    end
end
main()
