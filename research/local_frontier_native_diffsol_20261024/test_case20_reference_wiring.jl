# Production wiring and synthetic endpoint admission only. No integration.
# ARGS: SOURCE.jl CASE20_STANDARD_DESCRIPTOR.json NEW_EVIDENCE_DIR
using Test,JSON3,SHA,SparseArrays
length(ARGS)==3 || error("source, standard descriptor, new evidence directory required")
source,descriptor_path,evidence_dir=ARGS
mkdir(evidence_dir)
include(abspath(source))
raw=read(descriptor_path);descriptor_sha=bytes2hex(sha256(raw))
desc=JSON3.read(String(raw));c=config(desc)
observations=Dict{String,Any}()

function allocation_samples(problem)
    f=problem.f;p=problem.p;y=problem.u0;out=similar(y);J=copy(f.jac_prototype)
    for _ in 1:10
        f(out,y,p,0.73);f.jac(J,y,p,0.73);f.tgrad(out,y,p,0.73)
    end
    Dict("rhs"=>[@allocated f(out,y,p,0.73) for _ in 1:10],
         "jac"=>[@allocated f.jac(J,y,p,0.73) for _ in 1:10],
         "tgrad"=>[@allocated f.tgrad(out,y,p,0.73) for _ in 1:10])
end

@testset "case20 actual sparse reference ODEFunction" begin
    @test c.id==20 && c.size=="standard" && c.n==1024 && c.m==32 && c.tend==2.0
    @test descriptor_sha=="fab044d7ed2e4fe6ae80abe3b308143401b2079ef90d612e7699478cddfbb41b"
    problem,counts=reference_problem(c)
    @test problem.tspan==(0.0,2.0) && problem.u0==c.initial
    f=problem.f;p=problem.p;J=copy(f.jac_prototype)
    @test J isa SparseMatrixCSC{Float64,Int}
    @test size(J)==(1024,1024) && nnz(J)==2946
    @test J!==p.support.prototype
    colptr=J.colptr;rowval=J.rowval;nzval=J.nzval
    y=[(-1.0)^k/(1+k) for k in 0:1023]
    out=zeros(1024);expected=similar(out);v=[sin(0.37*k) for k in 1:1024]
    for t in (0.0,0.73,2.0)
        f(out,y,p,t);rhs!(expected,y,c,t)
        @test out==expected
        f.jac(J,y,p,t);jvp!(expected,y,v,c,t)
        @test maximum(abs.(J*v.-expected))<=128eps(Float64)*max(1.0,maximum(abs,expected))
        f.tgrad(out,y,p,t)
        @test out[1]===0.0
        @test all(out[k]==4e4*p.support.damping[k]*y[k]/(1+t)^5 for k in 2:1024)
        @test J.colptr===colptr && J.rowval===rowval && J.nzval===nzval
    end
    @test counts.rhs[]==3 && counts.jac[]==3 && counts.tgrad[]==3
    allocations=allocation_samples(problem)
    @test all(maximum(samples)==0 for samples in values(allocations))
    levels=reference_levels(c)
    @test [(x[1],x[3],x[4]) for x in levels]==[
        ("tsit5_1e8",1e-8,1e-10),("tsit5_1e9",1e-9,1e-11),
        ("tsit5_1e10",1e-10,1e-12),("rodas5p_1e10",1e-10,1e-12)]
    @test levels[4][2] isa Rodas5P
    observations["callback_allocations"]=allocations
    observations["jacobian_nnz"]=nnz(J)
end

@testset "synthetic four endpoint reference admission" begin
    rows=[Dict{String,Any}("label"=>label,"rtol"=>rtol,"atol"=>atol,
              "success"=>true,"final_time"=>2.0,"full_endpoint_state"=>copy(c.initial))
          for (label,alg,rtol,atol) in reference_levels(c)]
    offsets=(0.0,4e-8,1e-8,-3e-8)
    for (i,row) in enumerate(rows)
        row["full_endpoint_state"][1]+=offsets[i]
    end
    good=joinpath(evidence_dir,"accepted");mkdir(good)
    complete_reference(desc,descriptor_sha,c,Dict(),rows,good)
    evidence=JSON3.read(read(joinpath(good,"REFERENCE_EVIDENCE.json"),String))
    reference=JSON3.read(read(joinpath(good,"EXTERNAL_REFERENCE.json"),String))
    @test length(evidence.pairwise_inf)==6
    @test reference.uncertainty==10*abs(rows[2]["full_endpoint_state"][1]-rows[4]["full_endpoint_state"][1])
    @test reference.reference_state==rows[3]["full_endpoint_state"]
    @test reference.reference_state!=rows[1]["full_endpoint_state"]
    floor_rows=deepcopy(rows)
    for row in floor_rows;row["full_endpoint_state"]=copy(c.initial);end
    floor_dir=joinpath(evidence_dir,"uncertainty_floor");mkdir(floor_dir)
    complete_reference(desc,descriptor_sha,c,Dict(),floor_rows,floor_dir)
    @test JSON3.read(read(joinpath(floor_dir,"EXTERNAL_REFERENCE.json"),String)).uncertainty==1e-12
    negative_count=0
    for bad in ("failed","empty","dimension","nonfinite","wrong_endpoint","uncertainty","missing_level","wrong_tolerance")
        changed=deepcopy(rows)
        if bad=="failed";changed[1]["success"]=false
        elseif bad=="empty";changed[1]["full_endpoint_state"]=nothing
        elseif bad=="dimension";pop!(changed[1]["full_endpoint_state"])
        elseif bad=="nonfinite";changed[1]["full_endpoint_state"][1]=Inf
        elseif bad=="wrong_endpoint";changed[1]["final_time"]=1.99
        elseif bad=="uncertainty";changed[1]["full_endpoint_state"][1]+=2e-6
        elseif bad=="missing_level";pop!(changed)
        else;changed[1]["rtol"]=1e-7
        end
        outdir=joinpath(evidence_dir,bad);mkdir(outdir)
        @test_throws Exception complete_reference(desc,descriptor_sha,c,Dict(),changed,outdir)
        @test !isfile(joinpath(outdir,"EXTERNAL_REFERENCE.json"))
        negative_count+=1
    end
    @test full_endpoint_success(SciMLBase.ReturnCode.Success,[2.0],[c.initial],1024,2.0)
    @test !full_endpoint_success(SciMLBase.ReturnCode.Success,Float64[],[c.initial],1024,2.0)
    @test !full_endpoint_success(SciMLBase.ReturnCode.Success,[1.99],[c.initial],1024,2.0)
    @test !full_endpoint_success(SciMLBase.ReturnCode.Success,[2.0],[c.initial[1:3]],1024,2.0)
    @test !full_endpoint_success(SciMLBase.ReturnCode.MaxIters,[2.0],[c.initial],1024,2.0)
    observations["endpoint_negative_controls"]=negative_count
end
observations["integration_performed"]=false
observations["pass"]=true
observations["source_sha256"]=bytes2hex(sha256(read(source)))
observations["descriptor_sha256"]=descriptor_sha
emit(joinpath(evidence_dir,"TEST_RECEIPT.json"),observations)
println(JSON3.write(observations))
