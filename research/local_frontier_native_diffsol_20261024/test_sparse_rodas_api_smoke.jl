#!/usr/bin/env julia
# Bounded manufactured 3-state API check only; never invokes a benchmark model.
using JSON3, SparseArrays, LinearAlgebra, SciMLBase, OrdinaryDiffEqRosenbrock
length(ARGS)==1 || error("new output directory required")
outdir=ARGS[1];mkdir(outdir)
A=sparse([1,2,2,3,3],[1,1,2,2,3],[-2.0,0.3,-3.0,0.2,-4.0],3,3)
external_jac_calls=Ref(0)
function rhs!(du,u,p,t)
    mul!(du,p,u)
    nothing
end
function external_jac!(J,u,p,t)
    external_jac_calls[]+=1
    J isa SparseMatrixCSC{Float64,Int} || error("Rodas Jacobian was not sparse CSC")
    J.colptr==p.colptr && J.rowval==p.rowval || error("Rodas Jacobian pattern changed")
    copyto!(J.nzval,p.nzval)
    nothing
end
function run()
    function_object=ODEFunction(rhs!;jac=external_jac!,jac_prototype=copy(A))
    problem=ODEProblem(function_object,[1.0,0.5,-0.2],(0.0,0.001),A)
    solution=solve(problem,Rodas5P();reltol=1e-8,abstol=1e-10,maxiters=100,
                   save_everystep=false,save_start=false,save_end=true,dense=false)
    pass=SciMLBase.successful_retcode(solution.retcode) &&
         solution.t[end]==0.001 && external_jac_calls[]>0 &&
         all(isfinite,solution.u[end])
    Dict("schema"=>"vigilode.manufactured-sparse-rodas-api-smoke.v1",
         "dimension"=>3,"jacobian_nnz"=>nnz(A),
         "external_jac_calls"=>external_jac_calls[],
         "retcode"=>string(solution.retcode),"final_time"=>solution.t[end],
         "final_state"=>Float64.(solution.u[end]),"pass"=>pass,
         "scope"=>"API compatibility only; manufactured 3-state linear ODE, not benchmark case20")
end
try
    result=run()
    open(joinpath(outdir,"RESULT.json"),"w") do io
        write(io,JSON3.write(result));write(io,'\n')
    end
    result["pass"] || error("manufactured sparse Rodas API check failed")
catch err
    open(joinpath(outdir,"FAILED.json"),"w") do io
        write(io,JSON3.write(Dict("status"=>"FAILED_API_SMOKE",
                                 "exception"=>sprint(showerror,err,catch_backtrace()))))
        write(io,'\n')
    end
    rethrow()
end
