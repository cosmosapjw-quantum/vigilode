#!/usr/bin/env julia
# Independent, probe-gated local benchmark references. A controller must impose
# the absolute deadline, 12 GiB RSS cap, and exclusive output/process ownership.
# Usage: julia --project=PINNED independent_sparse_references.jl DESCRIPTOR NEW_DIR probe|solve
using JSON3, SHA, LinearAlgebra, SciMLBase, OrdinaryDiffEqTsit5, OrdinaryDiffEqRosenbrock
include(joinpath(@__DIR__,"sparse_jacobian_support.jl"))
using .SparseJacobianSupport
BLAS.set_num_threads(1)
const PROBE_EPS = 128eps(Float64)
const REF_SCHEMA = "vigilode.local-bottleneck.external-reference.v1"

function emit(path, data)
    payload=JSON3.write(data)*"\n"
    io=Base.Filesystem.open(path,Base.Filesystem.JL_O_WRONLY |
                           Base.Filesystem.JL_O_CREAT | Base.Filesystem.JL_O_EXCL,0o600)
    try
        write(io,payload)
    finally
        close(io)
    end
end
function preserve_failure(path,row)
    try
        emit(path,row)
    catch record_error
        println(stderr,"Failure receipt was not replaced: ",sprint(showerror,record_error))
    end
    nothing
end
check(x,msg) = x || error(msg)
near(a,b) = isfinite(a) && isfinite(b) && abs(a-b) <= PROBE_EPS*max(1.0,abs(a),abs(b))
function vector(x,n,name)
    check(length(x)==n,"$name length mismatch")
    y=Float64.(x)
    check(all(isfinite,y),"$name nonfinite")
    y
end

function config(c)
    check(String(c.kind)=="model_contract" && Int(c.schema_version)==1,"descriptor schema")
    id=Int(c.case_id); check(id in (4,8,12,14,17,18,20),"case unsupported")
    size=String(c.size);check(size in ("smoke","standard"),"size")
    smoke=size=="smoke"
    nx,ny,n,tend = if id==4
        k=smoke ? 16 : 4096; (k,0,k,smoke ? 0.02 : 10.0)
    elseif id==8
        (0,0,3,smoke ? 0.01 : 30.0)
    elseif id==12
        k=smoke ? 16 : 1024; (k,0,k,smoke ? 0.01 : 10.0)
    elseif id==14
        k=smoke ? 16 : 4096; (k,0,k,smoke ? 0.0001 : 1.0)
    elseif id==17
        a,b=smoke ? (4,8) : (32,64); (a,b,a*b,smoke ? 0.01 : 1.0)
    elseif id==18
        a,b=smoke ? (4,8) : (32,128); (a,b,a*b,smoke ? 0.001 : 0.1)
    else
        k=smoke ? 4 : 32; (k,0,k*k,smoke ? 0.001 : 2.0)
    end
    check(Int(c.model_contract.id)==id && Int(c.model_contract.dimension)==n &&
          String(c.model_contract.size)==size && Int(c.model_contract.grid[1])==nx &&
          Int(c.model_contract.grid[2])==ny,"metadata dimension/grid")
    check(near(Float64(c.t_start),0.0) && near(Float64(c.t_end),tend) &&
          near(Float64(c.model_contract.t_span[2]),tend),"time span")
    check(String(c.model_id)=="vigilode.local-bottleneck.case$(id).v1" &&
          String(c.model_contract.reference_kind)=="external-reference-required" &&
          c.exact_final===nothing,"reference identity")
    check(JSON3.write(c.parameters)==JSON3.write(c.model_contract.parameters),"parameter copies")
    initial=vector(c.initial_state,n,"initial state")
    metadata_initial=vector(c.model_contract.initial_state,n,"metadata initial")
    formula_initial = if id==4
        [2*pi*i/n+0.1*sin(3.0*i) for i in 0:n-1]
    elseif id==8
        [1.0,2.0,3.0]
    elseif id==12
        [cos((32*pi*i/n)/16)*(1+sin((32*pi*i/n)/16)) for i in 0:n-1]
    elseif id==14
        [0.1*cos(2*pi*i/n) for i in 0:n-1]
    elseif id in (17,18)
        [begin
             mu=-1.0+div(i,ny)*2.0/nx+1.0/nx
             phi=2*pi*mod(i,ny)/ny
             1.0+0.1*sqrt(1.0-mu*mu)*cos(phi)
         end for i in 0:n-1]
    else
        [0.01/(1.0+i) for i in 0:n-1]
    end
    check(all(near.(initial,metadata_initial)) && all(near.(initial,formula_initial)),
          "initial state or ordering")
    p=c.parameters
    if id==4
        check(Int(p.radius)==(smoke ? 4 : 32) && Float64(p.coupling)==1.0 &&
              Float64(p.alpha)==0.2 && String(p.frequency)=="0.5+0.1cos(2pi i/N)",
              "Kuramoto parameters")
        check(String(c.model_contract.variant)=="local periodic coupling, no self term; radius4 smoke/32standard",
              "Kuramoto variant")
        coeff=(radius=Int(p.radius),coupling=Float64(p.coupling),alpha=Float64(p.alpha))
    elseif id==8
        check(Float64(p.s)==77.27 && Float64(p.q)==8.375e-6 && Float64(p.w)==0.161,
              "Oregonator parameters")
        check(String(c.model_contract.variant)=="specified 3-variable coefficients",
              "Oregonator variant")
        coeff=(s=Float64(p.s),q=Float64(p.q),w=Float64(p.w))
    elseif id==12
        check(near(Float64(p.length),32*pi) && Float64(p.nu)==1.0 &&
              String(p.boundary)=="periodic" &&
              String(p.discretization)=="centered D1,D2,D2 squared; RHS=-u D1u-D2u-D2^2u",
              "KS parameters")
        check(String(c.model_contract.variant)=="periodic centered semidiscretization; correct negative RHS signs",
              "KS variant")
        coeff=(dx=32*pi/n,)
    elseif id==14
        check(Float64(p.epsilon)==0.02 && Float64(p.velocity)==0.1 &&
              near(Float64(p.length),2*pi) && String(p.boundary)=="periodic",
              "Cahn-Hilliard parameters")
        check(String(c.model_contract.variant)=="periodic 1D centered conservative semidiscretization",
              "Cahn-Hilliard variant")
        coeff=(dx=2*pi/n,)
    elseif id==17
        axis=Float64.(p.rotation_axis)
        check(length(axis)==3 && near(axis[1],inv(sqrt(2.0))) && axis[2]==0.0 &&
              near(axis[3],inv(sqrt(2.0))) && Float64(p.shear)==0.1 &&
              String(p.discretization)=="first-order conservative upwind equal-mu FVM; zero polar face flux",
              "rotation-shear parameters")
        check(String(c.model_contract.variant)=="equal-mu first-order conservative FVM; explicit rotation and shear fields",
              "rotation-shear variant")
        coeff=(invroot2=axis[1],shear=Float64(p.shear))
    elseif id==18
        check(Float64(p.d_mu)==0.01 && String(p.d_phi)=="0.1(1+mu^2/2)" &&
              String(p.opacity)=="1+1000 I(mu>.5 and cos(phi)>.5)" &&
              String(p.discretization)=="conservative equal-mu fluxes, periodic phi",
              "diffusion-absorption parameters")
        check(String(c.model_contract.variant)=="anisotropic centered fluxes plus fixed angular opacity patch",
              "diffusion-absorption variant")
        coeff=(dmu=Float64(p.d_mu),)
    else
        check(Float64(p.k)==1.0 && Float64(p.gamma0)==1e4 &&
              Int(p.gamma_exponent)==4 && Float64(p.baryon_dipole)==0.0,
              "hierarchy parameters")
        check(String(p.normalization)=="orthonormal fixed-m antisymmetric chains; top moment zero; external fixed matter bath" &&
              String(c.model_contract.variant)=="fixed matter bath and normalized real chains; no full photon-baryon claim",
              "hierarchy variant")
        coeff=(gamma0=Float64(p.gamma0),)
    end
    (;id,size,n,nx,ny,m=nx,tend,initial,coeff)
end

# Scalar coefficient captures make the callback independent of JSON3 and allocation free.
@inline speed_flux(speed,left,right) = speed*(speed>=0 ? left : right)
@inline function rotation_vm(mu,phi,axis,shear)
    axis*sqrt(max(0.0,1-mu*mu))*sin(phi)+shear*mu*(1-mu*mu)
end
@inline function rotation_vp(mu,phi,axis)
    axis*(1-mu*cos(phi)/sqrt(1-mu*mu))
end
function rhs!(du,y,c,t)
    if c.id==4
        r,alpha,coupling=c.coeff.radius,c.coeff.alpha,c.coeff.coupling
        for i in 1:c.n
            s=0.0
            for d in -r:r
                d==0 && continue
                s+=sin(y[mod1(i+d,c.n)]-y[i]-alpha)
            end
            du[i]=0.5+0.1*cos(2*pi*(i-1)/c.n)+coupling*s/(2*r)
        end
    elseif c.id==8
        s,q,w=c.coeff.s,c.coeff.q,c.coeff.w
        a=y[1]*y[2]
        du[1]=s*(y[2]-a+y[1]-q*y[1]*y[1])
        du[2]=(-y[2]-a+y[3])/s
        du[3]=w*(y[1]-y[3])
    elseif c.id in (12,14)
        dx=c.coeff.dx
        for i in 1:c.n
            center=y[i]
            right=y[mod1(i+1,c.n)]; left=y[mod1(i-1,c.n)]
            d1=(right-left)/(2*dx)
            d2=(right-2*center+left)/(dx*dx)
            d4=(y[mod1(i+2,c.n)]-4*right+6*center-4*left+y[mod1(i-2,c.n)])/dx^4
            if c.id==12
                du[i]=-center*d1-d2-d4
            else
                lapcube=(right^3-2*center^3+left^3)/(dx*dx)
                du[i]=lapcube-d2-0.0004*d4-0.1*d1
            end
        end
    elseif c.id==17
        nx,ny=c.nx,c.ny
        dm=2.0/nx;dp=2*pi/ny
        axis,shear=c.coeff.invroot2,c.coeff.shear
        for i in 1:c.n
            a=div(i-1,ny);b=mod(i-1,ny)
            mu=-1.0+(a+0.5)*dm;phi=b*dp
            up=a+1<nx ? speed_flux(rotation_vm(mu+dm/2,phi,axis,shear),
                                   y[i],y[i+ny]) : 0.0
            down=a>0 ? speed_flux(rotation_vm(mu-dm/2,phi,axis,shear),
                                  y[i-ny],y[i]) : 0.0
            right=speed_flux(rotation_vp(mu,phi+dp/2,axis),y[i],
                             y[a*ny+mod(b+1,ny)+1])
            left=speed_flux(rotation_vp(mu,phi-dp/2,axis),
                            y[a*ny+mod(b-1,ny)+1],y[i])
            du[i]=-(up-down)/dm-(right-left)/dp
        end
    elseif c.id==18
        nx,ny=c.nx,c.ny
        dm=2.0/nx;dp=2*pi/ny
        for i in 1:c.n
            a=div(i-1,ny);b=mod(i-1,ny)
            mu=-1.0+(a+0.5)*dm;phi=b*dp
            up=a+1<nx ? 0.01*(1-(mu+dm/2)^2)*(y[i+ny]-y[i])/(dm*dm) : 0.0
            down=a>0 ? 0.01*(1-(mu-dm/2)^2)*(y[i-ny]-y[i])/(dm*dm) : 0.0
            right=y[a*ny+mod(b+1,ny)+1]
            left=y[a*ny+mod(b-1,ny)+1]
            angular=0.1*(1+mu*mu/2)/(1-mu*mu)*(right-2*y[i]+left)/(dp*dp)
            opacity=mu>0.5 && cos(phi)>0.5 ? 1001.0 : 1.0
            du[i]=up+down+angular-opacity*y[i]
        end
    else
        g=c.coeff.gamma0/(1.0+t)^4
        for k in 0:c.n-1
            l=floor(Int,sqrt(Float64(k)))
            mm=k-l*l-l
            low=0.0
            if l>0 && abs(mm)<l
                a=sqrt(Float64(l*l-mm*mm))/sqrt(Float64(4*l*l-1))
                low=a*y[l*l-l+mm+1]
            end
            high=0.0
            if l+1<c.m
                z=l+1
                a=sqrt(Float64(z*z-mm*mm))/sqrt(Float64(4*z*z-1))
                high=a*y[z*z+z+mm+1]
            end
            damping=l==0 ? 0.0 : l==2 ? 0.9 : 1.0
            du[k+1]=low-high-damping*g*y[k+1]
        end
    end
    nothing
end

function jvp!(out,y,v,c,t)
    if c.id==4
        r,alpha,coupling=c.coeff.radius,c.coeff.alpha,c.coeff.coupling
        for i in 1:c.n
            s=0.0
            for d in -r:r
                d==0 && continue
                j=mod1(i+d,c.n)
                s+=cos(y[j]-y[i]-alpha)*(v[j]-v[i])
            end
            out[i]=coupling*s/(2*r)
        end
    elseif c.id==8
        s,q,w=c.coeff.s,c.coeff.q,c.coeff.w
        out[1]=s*((1-y[1])*v[2] + (1-y[2]-2q*y[1])*v[1])
        out[2]=(-v[2]-v[1]*y[2]-y[1]*v[2]+v[3])/s
        out[3]=w*(v[1]-v[3])
    elseif c.id in (12,14)
        dx=c.coeff.dx
        for i in 1:c.n
            center=y[i];vc=v[i]
            right=y[mod1(i+1,c.n)];left=y[mod1(i-1,c.n)]
            vr=v[mod1(i+1,c.n)];vl=v[mod1(i-1,c.n)]
            vd1=(vr-vl)/(2*dx)
            vd2=(vr-2*vc+vl)/(dx*dx)
            vd4=(v[mod1(i+2,c.n)]-4*vr+6*vc-4*vl+v[mod1(i-2,c.n)])/dx^4
            if c.id==12
                yd1=(right-left)/(2*dx)
                out[i]=-vc*yd1-center*vd1-vd2-vd4
            else
                cube_direction=(3*right^2*vr-6*center^2*vc+3*left^2*vl)/(dx*dx)
                out[i]=cube_direction-vd2-0.0004*vd4-0.1*vd1
            end
        end
    else
        rhs!(out,v,c,t) # cases 17,18,20 are linear in state at fixed t
    end
    nothing
end

function probe_controls(descriptor,c)
    check(length(descriptor.rhs_probes)==2,"probe count")
    entries=Any[(label="initial",t=0.0,state=descriptor.initial_state,rhs=descriptor.rhs_initial)]
    for (j,p) in enumerate(descriptor.rhs_probes)
        t=(j==1 ? 0.11 : 0.73)*c.tend
        check(near(Float64(p.t),t),"probe time")
        state=vector(p.state,c.n,"probe state")
        check(all(near(state[k],c.initial[k]+0.001*j*k/c.n) for k in 1:c.n),
              "probe state construction")
        push!(entries,(label="perturbed_$j",t=Float64(p.t),state=p.state,rhs=p.rhs))
    end
    results=Any[]
    for entry in entries
        y=vector(entry.state,c.n,"probe state")
        target=vector(entry.rhs,c.n,"probe RHS")
        actual=similar(y);rhs!(actual,y,c,entry.t)
        residual=maximum(abs.(actual.-target))
        pass=all(near.(actual,target))
        push!(results,Dict("label"=>entry.label,"time"=>entry.t,
                           "maximum_absolute_residual"=>residual,"pass"=>pass))
    end
    results
end

function derivative_controls(c)
    y=copy(c.initial)
    for k in eachindex(y)
        y[k]+=0.0001*sin(Float64(k))
    end
    v=[sin(0.37*k) for k in 1:c.n]
    t=0.31*c.tend
    exact=similar(y);jvp!(exact,y,v,c,t)
    plus=similar(y);minus=similar(y)
    h=1e-6/max(1.0,maximum(abs,v))
    yp=similar(y);ym=similar(y)
    for k in eachindex(y)
        yp[k]=y[k]+h*v[k]; ym[k]=y[k]-h*v[k]
    end
    rhs!(plus,yp,c,t);rhs!(minus,ym,c,t)
    residual=maximum(abs.(exact.-(plus.-minus)./(2h)))
    scale=max(1.0,maximum(abs,exact))
    # Directional differencing is an independent finite-difference control,
    # not a reference accuracy bound or a relaxed RHS-probe criterion.
    pass=residual/scale <= 1e-6
    Dict("time"=>t,"step"=>h,"maximum_absolute_residual"=>residual,
         "relative_scale_residual"=>residual/scale,"pass"=>pass,
         "method"=>"central directional difference on independent RHS")
end

function allocation_controls(c)
    y=copy(c.initial);du=similar(y);v=copy(y)
    for _ in 1:10
        rhs!(du,y,c,0.0); jvp!(du,y,v,c,0.0)
    end
    rhs_alloc=[@allocated rhs!(du,y,c,0.0) for _ in 1:10]
    jvp_alloc=[@allocated jvp!(du,y,v,c,0.0) for _ in 1:10]
    Dict("rhs_bytes"=>rhs_alloc,"jvp_bytes"=>jvp_alloc,
         "pass"=>maximum(rhs_alloc)==0 && maximum(jvp_alloc)==0)
end

function invariants(c,y)
    if c.id in (12,14,17)
        Dict("sum_state"=>sum(y),"minimum_component"=>minimum(y),
             "scope"=>"periodic or zero-polar-flux semidiscrete mass diagnostic")
    elseif c.id==18
        Dict("sum_state"=>sum(y),"minimum_component"=>minimum(y),
             "scope"=>"diffusion with absorption; mass is not conserved")
    else
        Dict("minimum_component"=>minimum(y),"l2_norm"=>sqrt(sum(abs2,y)),
             "scope"=>"descriptive only")
    end
end

function boundary_controls(c)
    if !(c.id in (12,14,17,18))
        return Dict("status"=>"NOT_APPLICABLE")
    end
    deriv=similar(c.initial)
    rhs!(deriv,c.initial,c,0.0)
    mass=sum(deriv)
    if c.id==18
        absorption=0.0
        for i in 1:c.n
            a=div(i-1,c.ny);b=mod(i-1,c.ny)
            mu=-1.0+(a+0.5)*2.0/c.nx
            phi=2*pi*b/c.ny
            opacity=mu>0.5 && cos(phi)>0.5 ? 1001.0 : 1.0
            absorption+=opacity*c.initial[i]
        end
        residual=abs(mass+absorption)
        scale=max(1.0,abs(absorption),c.n*maximum(abs,deriv))
        return Dict("kind"=>"diffusive_flux_balance_plus_absorption",
                    "absolute_residual"=>residual,"scaled_residual"=>residual/scale,
                    "pass"=>residual/scale<=1e-8)
    else
        residual=abs(mass)
        scale=max(1.0,c.n*maximum(abs,deriv))
        return Dict("kind"=>"zero_total_boundary_flux",
                    "absolute_residual"=>residual,"scaled_residual"=>residual/scale,
                    "pass"=>residual/scale<=1e-8)
    end
end

function counted_reference_rhs!(du,y,p,t)
    p.counts.rhs[]+=1
    rhs!(du,y,p.config,t)
end
function counted_case20_rhs!(du,y,p,t)
    p.counts.rhs[]+=1
    case20_rhs!(du,y,p.support,t)
end
function counted_case20_jac!(J,y,p,t)
    p.counts.jac[]+=1
    case20_jacobian!(J,y,p.support,t)
end
function counted_case20_tgrad!(du,y,p,t)
    p.counts.tgrad[]+=1
    case20_tgrad!(du,y,p.support,t)
end

function reference_problem(c)
    counts=(rhs=Ref(0),jac=Ref(0),tgrad=Ref(0))
    if c.id==20
        support=build_case20_support(c.m)
        p=(support=support,counts=counts)
        object=ODEFunction(counted_case20_rhs!;jac=counted_case20_jac!,
                           tgrad=counted_case20_tgrad!,jac_prototype=copy(support.prototype))
    else
        p=(config=c,counts=counts)
        object=ODEFunction(counted_reference_rhs!)
    end
    ODEProblem(object,copy(c.initial),(0.0,c.tend),p),counts
end

function reference_callback_controls(c)
    c.id==20 || return Dict("status"=>"NOT_APPLICABLE")
    prob,counts=reference_problem(c)
    f=prob.f;p=prob.p;y=prob.u0;du=similar(y);J=copy(f.jac_prototype)
    for _ in 1:10
        f(du,y,p,0.73);f.jac(J,y,p,0.73);f.tgrad(du,y,p,0.73)
    end
    rhs_alloc=[@allocated f(du,y,p,0.73) for _ in 1:10]
    jac_alloc=[@allocated f.jac(J,y,p,0.73) for _ in 1:10]
    tgrad_alloc=[@allocated f.tgrad(du,y,p,0.73) for _ in 1:10]
    Dict("rhs_bytes"=>rhs_alloc,"jac_bytes"=>jac_alloc,"tgrad_bytes"=>tgrad_alloc,
         "jacobian_storage"=>"SparseMatrixCSC{Float64,Int}",
         "pattern"=>case20_pattern_bytes(p.support),
         "pass"=>maximum(rhs_alloc)==0 && maximum(jac_alloc)==0 && maximum(tgrad_alloc)==0)
end

function full_endpoint_success(retcode,times,states,n,tend)
    SciMLBase.successful_retcode(retcode) && !isempty(times) && !isempty(states) &&
        times[end]==tend && length(states[end])==n && all(isfinite,states[end])
end

function reference_levels(c)
    check(c.id==8 || (c.id==20 && c.size=="standard" && c.n==1024 && c.tend==2.0),
          "only case8 and frozen standard case20 have reviewed reference solve paths")
    (("tsit5_1e8",Tsit5(),1e-8,1e-10),
     ("tsit5_1e9",Tsit5(),1e-9,1e-11),
     ("tsit5_1e10",Tsit5(),1e-10,1e-12),
     ("rodas5p_1e10",Rodas5P(),1e-10,1e-12))
end

function solve_level(label,algorithm,rtol,atol,c,outdir)
    prob,counts=reference_problem(c)
    start=time_ns()
    try
        sol=solve(prob,algorithm;reltol=rtol,abstol=atol,maxiters=2_000_000,
                  save_everystep=false,save_start=false,save_end=true,dense=false)
        state=isempty(sol.u) ? nothing : Float64.(sol.u[end])
        success=full_endpoint_success(sol.retcode,sol.t,sol.u,c.n,c.tend)
        row=Dict("label"=>label,"algorithm"=>string(typeof(algorithm)),
                 "rtol"=>rtol,"atol"=>atol,"elapsed_seconds"=>(time_ns()-start)*1e-9,
                 "retcode"=>string(sol.retcode),"success"=>success,
                 "final_time"=>isempty(sol.t) ? nothing : sol.t[end],
                 "full_endpoint_state"=>state!==nothing && all(isfinite,state) ? state : nothing,
                 "rhs_callback_count"=>counts.rhs[],"jac_callback_count"=>counts.jac[],
                 "tgrad_callback_count"=>counts.tgrad[],"timing_authority"=>"diagnostic_only",
                 "jacobian_storage"=>c.id==20 ? "SparseMatrixCSC{Float64,Int}" : "default case8 storage",
                 "stats"=>Dict("naccept"=>sol.stats.naccept,"nreject"=>sol.stats.nreject,
                               "nf"=>sol.stats.nf,"njacs"=>sol.stats.njacs),
                 "invariants"=>success ? invariants(c,state) : nothing)
        emit(joinpath(outdir,"$(label).json"),row)
        row
    catch err
        preserve_failure(joinpath(outdir,"$(label).json"),Dict("label"=>label,
             "status"=>"FAILED_SOLVE_EXCEPTION","elapsed_seconds"=>(time_ns()-start)*1e-9,
             "exception"=>sprint(showerror,err),"rhs_callback_count"=>counts.rhs[],
             "jac_callback_count"=>counts.jac[],"tgrad_callback_count"=>counts.tgrad[],
             "timing_authority"=>"diagnostic_only"))
        rethrow()
    end
end

function construct_reference(descriptor_path,outdir,mode)
    raw=read(descriptor_path)
    # String(::Vector{UInt8}) may consume its input. Bind the original bytes
    # before parsing and reuse this identity in every resulting receipt.
    descriptor_sha=bytes2hex(sha256(raw))
    desc=JSON3.read(String(raw));c=config(desc)
    probes=probe_controls(desc,c)
    derivative=derivative_controls(c)
    allocations=allocation_controls(c)
    reference_allocations=reference_callback_controls(c)
    boundary=boundary_controls(c)
    control=Dict("schema"=>"vigilode.independent-sparse-reference-control.v1",
                 "descriptor_sha256"=>descriptor_sha,
                 "source_sha256"=>bytes2hex(sha256(read(@__FILE__))),
                 "sparse_support_sha256"=>bytes2hex(sha256(read(joinpath(@__DIR__,"sparse_jacobian_support.jl")))),
                 "case_id"=>c.id,"size"=>c.size,"dimension"=>c.n,
                 "rhs_probes"=>probes,"jacobian_action"=>derivative,
                 "boundary_flux"=>boundary,
                 "callback_allocations"=>allocations,"mode"=>mode,
                 "reference_callback_allocations"=>reference_allocations,
                 "initial_invariants"=>invariants(c,c.initial),
                 "julia_version"=>string(VERSION))
    emit(joinpath(outdir,"PROBE_CONTROL.json"),control)
    check(all(row["pass"] for row in probes),"RHS probe mismatch")
    check(derivative["pass"],"Jacobian action check failed")
    check(allocations["pass"],"warmed callback allocated")
    check(get(reference_allocations,"pass",true),"warmed sparse reference callback allocated")
    check(get(boundary,"pass",true),"boundary flux check failed")
    mode=="probe" && return
    levels=reference_levels(c)
    rows=Any[]
    for (label,alg,rtol,atol) in levels
        row=solve_level(label,alg,rtol,atol,c,outdir)
        push!(rows,row)
        check(row["success"],"$label failed")
    end
    complete_reference(desc,descriptor_sha,c,control,rows,outdir)
end

function complete_reference(desc,descriptor_sha,c,control,rows,outdir)
    levels=reference_levels(c)
    check(length(rows)==4,"four full endpoint levels required")
    for (row,(label,alg,rtol,atol)) in zip(rows,levels)
        state=get(row,"full_endpoint_state",nothing)
        check(get(row,"label",nothing)==label && get(row,"rtol",nothing)==rtol &&
              get(row,"atol",nothing)==atol,"reference level identity or tolerance mismatch")
        check(get(row,"success",false)===true && get(row,"final_time",nothing)==c.tend &&
              state!==nothing && length(state)==c.n && all(isfinite,state),
              "$label requires successful finite full endpoint at frozen t_end")
    end
    states=[row["full_endpoint_state"] for row in rows]
    differences=Dict{String,Float64}()
    for i in 1:3, j in i+1:4
        differences["$(rows[i]["label"])__$(rows[j]["label"])"]=maximum(abs.(states[i].-states[j]))
    end
    U=max(1e-12,10*maximum(values(differences)))
    evidence=Dict("schema"=>"vigilode.independent-sparse-reference-evidence.v1",
                  "descriptor_sha256"=>descriptor_sha,"control"=>control,
                  "levels"=>rows,"pairwise_inf"=>differences,"empirical_uncertainty"=>U,
                  "claim_ceiling"=>"empirical full-vector endpoint comparison only")
    evidence_path=joinpath(outdir,"REFERENCE_EVIDENCE.json")
    emit(evidence_path,evidence)
    check(isfinite(U) && U<=1e-5,"empirical uncertainty >1e-5 or nonfinite")
    reference=Dict("schema"=>REF_SCHEMA,"model_descriptor"=>desc,
                   "model_descriptor_sha256"=>descriptor_sha,
                   "norm"=>"max_component_absolute","reference_state"=>states[3],
                   "uncertainty"=>U,
                   "provenance"=>Dict("source"=>c.id==20 ? "independent Julia fixed-bath hierarchy RHS from frozen equations" : "independent Julia Oregonator RHS from frozen equations",
                                      "method"=>c.id==20 ? "three Tsit5 tolerances and fine sparse analytic jac+tgrad Rodas5P crosscheck" : "three Tsit5 tolerances and fine Rodas5P crosscheck",
                                      "evidence_sha256"=>bytes2hex(sha256(read(evidence_path)))))
    emit(joinpath(outdir,"EXTERNAL_REFERENCE.json"),reference)
end

function main(args=ARGS)
    check(length(args)==3,"descriptor, new output directory, probe|solve required")
    descriptor_path,outdir,mode=args
    check(mode in ("probe","solve"),"mode")
    # Only a successful fresh mkdir establishes ownership of failure output.
    mkdir(outdir)
    try
        construct_reference(descriptor_path,outdir,mode)
    catch err
        preserve_failure(joinpath(outdir,"FAILED.json"),Dict("status"=>"FAILED_REFERENCE_CONSTRUCTION",
                         "exception"=>sprint(showerror,err,catch_backtrace())))
        rethrow()
    end
end

if abspath(PROGRAM_FILE)==@__FILE__
    main()
end
