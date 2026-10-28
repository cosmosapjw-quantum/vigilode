# Synthetic case20 derivative/CSC controls and one manufactured API path.
# ARGS: controls|smoke NEW_OUTPUT_DIR. Never integrates the case20 benchmark.
using Test,JSON3,SHA,SparseArrays,LinearAlgebra
include(joinpath(@__DIR__,"sparse_jacobian_support.jl"))
using .SparseJacobianSupport

function emit_new(path,row)
    payload=JSON3.write(row)*"\n"
    io=Base.Filesystem.open(path,Base.Filesystem.JL_O_WRONLY |
                           Base.Filesystem.JL_O_CREAT | Base.Filesystem.JL_O_EXCL,0o600)
    try write(io,payload) finally close(io) end
end

function independent_big_rhs(u,support,t)
    out=Vector{BigFloat}(undef,support.n)
    for k in 0:support.n-1
        l=floor(Int,sqrt(Float64(k)));m=k-l*l-l
        low=zero(BigFloat);high=zero(BigFloat)
        if l>0 && abs(m)<l
            low=sqrt(BigFloat(l*l-m*m))/sqrt(BigFloat(4*l*l-1))*BigFloat(u[l*l-l+m+1])
        end
        if l+1<support.levels
            z=l+1
            high=sqrt(BigFloat(z*z-m*m))/sqrt(BigFloat(4*z*z-1))*BigFloat(u[z*z+z+m+1])
        end
        d=l==0 ? zero(BigFloat) : l==2 ? BigFloat(9)/10 : one(BigFloat)
        out[k+1]=low-high-d*BigFloat(10000)/(1+t)^4*BigFloat(u[k+1])
    end
    out
end

function allocations(support,y,J,t)
    out=similar(y)
    for _ in 1:10
        case20_rhs!(out,y,support,t);case20_jacobian!(J,y,support,t);case20_tgrad!(out,y,support,t)
    end
    Dict("rhs"=>[@allocated case20_rhs!(out,y,support,t) for _ in 1:10],
         "jac"=>[@allocated case20_jacobian!(J,y,support,t) for _ in 1:10],
         "tgrad"=>[@allocated case20_tgrad!(out,y,support,t) for _ in 1:10])
end

function controls()
    rows=Any[]
    @testset "case20 analytic time derivative and fixed CSC storage" begin
        for levels in (4,32)
            support=build_case20_support(levels);n=support.n
            signed=[(-1.0)^k*k/(n+1) for k in 1:n]
            states=[signed,[-z/(k+1) for (k,z) in enumerate(signed)]]
            worst=0.0;count=0
            setprecision(BigFloat,256) do
                h=big"1e-20"
                for state in states,t in (0.0,0.125,0.73,2.0)
                    actual=similar(state);case20_tgrad!(actual,state,support,t)
                    plus=independent_big_rhs(state,support,BigFloat(t)+h)
                    minus=independent_big_rhs(state,support,BigFloat(t)-h)
                    fd=(plus.-minus)./(2h)
                    @test reinterpret(UInt64,actual[1])==UInt64(0)
                    @test fd[1]==0
                    for k in 1:n
                        scaled=Float64(abs(BigFloat(actual[k])-fd[k])/max(BigFloat(1),abs(fd[k])))
                        worst=max(worst,scaled);count+=1
                        @test scaled<=32eps(Float64)
                    end
                end
            end
            @test Set(support.damping)==Set([0.0,0.9,1.0])
            J=copy(support.prototype)
            colptr,rowval,nzval=J.colptr,J.rowval,J.nzval
            columns,indices=copy(colptr),copy(rowval)
            for t in (0.0,0.125,0.73,2.0)
                case20_jacobian!(J,signed,support,t)
                @test J.colptr===colptr && J.rowval===rowval && J.nzval===nzval
                @test J.colptr==columns && J.rowval==indices
                for k in 1:n
                    @test J.nzval[support.diagonal_slot[k]] == -support.damping[k]*(1e4/(1.0+t)^4)
                    low,high=support.lower_slot[k],support.upper_slot[k]
                    @test low==0 || J.nzval[low]==support.lower_coefficient[k]
                    @test high==0 || J.nzval[high]==-support.upper_coefficient[k]
                end
            end
            a=allocations(support,signed,J,0.73)
            @test all(maximum(samples)==0 for samples in values(a))
            push!(rows,Dict("levels"=>levels,"dimension"=>n,"fd_precision_bits"=>256,
                 "independent_time_fd_step"=>"1e-20","signed_component_checks"=>count,
                 "maximum_scaled_fd_residual"=>worst,"threshold"=>32eps(Float64),
                 "l0_is_exact_positive_zero"=>true,"same_csc_storage_and_fixed_streaming"=>true,
                 "allocated_bytes"=>a,"pass"=>true))
        end
    end
    Dict("schema"=>"vigilode.case20-tgrad-controls.v1","controls"=>rows,"pass"=>true,
         "scope"=>"independent high-precision time FD and callback/storage controls; no integration")
end

function prepare!(u,p,t)
    amplitude=exp(-t)
    for k in eachindex(u)
        p.exact[k]=amplitude*p.target[k]
        p.delta[k]=u[k]-p.exact[k]
    end
end
function manufactured_rhs!(out,u,p,t)
    p.rhs_calls[]+=1;prepare!(u,p,t)
    case20_rhs!(out,p.delta,p.support,t)
    for k in eachindex(out)
        out[k]-=p.exact[k]
    end
    nothing
end
function manufactured_jac!(J,u,p,t)
    p.jac_calls[]+=1
    J isa SparseMatrixCSC{Float64,Int} || error("analytic Jacobian was not sparse CSC")
    case20_jacobian!(J,u,p.support,t)
end
function manufactured_tgrad!(out,u,p,t)
    p.tgrad_calls[]+=1;prepare!(u,p,t)
    case20_tgrad!(out,p.delta,p.support,t)
    case20_rhs!(p.scratch,p.exact,p.support,t)
    for k in eachindex(out)
        out[k]+=p.scratch[k]+p.exact[k]
    end
    nothing
end

function smoke()
    support=build_case20_support(4);n=support.n
    target=[(-1.0)^k/(k+1) for k in 1:n]
    p=(support=support,target=target,exact=zeros(n),delta=zeros(n),scratch=zeros(n),
       rhs_calls=Ref(0),jac_calls=Ref(0),tgrad_calls=Ref(0))
    object=ODEFunction(manufactured_rhs!;jac=manufactured_jac!,tgrad=manufactured_tgrad!,
                       jac_prototype=copy(support.prototype))
    problem=ODEProblem(object,copy(target),(0.0,0.001),p)
    solution=solve(problem,Rodas5P();reltol=1e-8,abstol=1e-10,maxiters=100,
                   save_everystep=false,save_start=false,save_end=true,dense=false)
    endpoint=Float64.(solution.u[end]);exact=exp(-0.001).*target
    pass=SciMLBase.successful_retcode(solution.retcode) && solution.t[end]==0.001 &&
         all(isfinite,endpoint) && p.rhs_calls[]>0 && p.jac_calls[]>0 && p.tgrad_calls[]>0
    Dict("schema"=>"vigilode.case20-manufactured-tgrad-api.v1","pass"=>pass,
         "dimension"=>n,"nonautonomous"=>true,"retcode"=>string(solution.retcode),
         "final_time"=>solution.t[end],"final_state"=>endpoint,
         "maximum_absolute_endpoint_error_diagnostic"=>maximum(abs.(endpoint.-exact)),
         "rhs_calls"=>p.rhs_calls[],"jac_calls"=>p.jac_calls[],"tgrad_calls"=>p.tgrad_calls[],
         "jacobian_nnz"=>nnz(support.prototype),
         "naccept"=>solution.stats.naccept,"nreject"=>solution.stats.nreject,
         "rtol"=>1e-8,"atol"=>1e-10,"maxiters"=>100,
         "manufactured_equation"=>"A(t)*(u-exp(-t)*v)-exp(-t)*v; exact y=exp(-t)*v",
         "scope"=>"sparse analytic jac+tgrad API compatibility only; not a case20 benchmark solve")
end

function main()
    length(ARGS)==2 || error("controls|smoke and new output directory required")
    mode,outdir=ARGS
    mode in ("controls","smoke") || error("mode")
    mkdir(outdir)
    try
        if mode=="smoke"
            @eval using SciMLBase,OrdinaryDiffEqRosenbrock
            BLAS.set_num_threads(1)
        end
        result=mode=="controls" ? controls() : Base.invokelatest(smoke)
        result["support_source_sha256"]=bytes2hex(sha256(read(joinpath(@__DIR__,"sparse_jacobian_support.jl"))))
        result["julia_version"]=string(VERSION)
        emit_new(joinpath(outdir,"RESULT.json"),result)
        println(JSON3.write(result))
        result["pass"] || error("case20 tgrad validation failed")
    catch err
        emit_new(joinpath(outdir,"FAILED.json"),Dict("exception"=>sprint(showerror,err,catch_backtrace())))
        rethrow()
    end
end
main()
