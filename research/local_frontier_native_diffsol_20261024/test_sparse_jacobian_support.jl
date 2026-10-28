#!/usr/bin/env julia
# Probe-only contract tests for the frozen case20 sparse Jacobian support.
using JSON3, SHA, SparseArrays, LinearAlgebra
include(joinpath(@__DIR__,"sparse_jacobian_support.jl"))
using .SparseJacobianSupport

function compare_rhs(actual,expected)
    maximum(abs.(actual.-expected)), all(abs(actual[k]-expected[k]) <=
        128eps(Float64)*max(1.0,abs(actual[k]),abs(expected[k])) for k in eachindex(actual))
end

function run_probe(descriptor_path,outdir)
    mkdir(outdir)
    raw=read(descriptor_path)
    descriptor_sha=bytes2hex(sha256(raw))
    descriptor=JSON3.read(String(copy(raw)))
    Int(descriptor.case_id)==20 || error("case20 descriptor required")
    size=String(descriptor.size)
    levels=size=="standard" ? 32 : size=="smoke" ? 4 : error("size")
    n=levels^2
    meta=descriptor.model_contract
    Int(meta.dimension)==n && Int(meta.grid[1])==levels && Int(meta.grid[2])==0 ||
        error("frozen hierarchy dimension/grid mismatch")
    p=descriptor.parameters
    Float64(p.gamma0)==1e4 && Int(p.gamma_exponent)==4 && Float64(p.k)==1.0 &&
        Float64(p.baryon_dipole)==0.0 || error("frozen hierarchy parameters")
    support=build_case20_support(levels)
    checks=Any[]
    samples=Any[(label="initial",t=0.0,state=descriptor.initial_state,rhs=descriptor.rhs_initial)]
    for (j,probe) in enumerate(descriptor.rhs_probes)
        push!(samples,(label="perturbed_$j",t=Float64(probe.t),state=probe.state,rhs=probe.rhs))
    end
    for sample in samples
        y=Float64.(sample.state);target=Float64.(sample.rhs)
        length(y)==n && length(target)==n || error("probe dimension")
        actual=similar(y);case20_rhs!(actual,y,support,sample.t)
        rhs_residual,rhs_pass=compare_rhs(actual,target)
        J=copy(support.prototype)
        case20_jacobian!(J,y,support,sample.t)
        v=[sin(0.37*k) for k in 1:n]
        jvp=similar(y);case20_jvp!(jvp,v,support,sample.t)
        matrix_action=J*v
        action_residual,action_pass=compare_rhs(matrix_action,jvp)
        h=1e-6
        plus=similar(y);minus=similar(y)
        yp=y.+h.*v;ym=y.-h.*v
        case20_rhs!(plus,yp,support,sample.t)
        case20_rhs!(minus,ym,support,sample.t)
        fd_residual=maximum(abs.(jvp.-(plus.-minus)./(2h)))
        fd_scaled=fd_residual/max(1.0,maximum(abs,jvp))
        fd_pass=fd_scaled<=1e-6
        row_sums=J*ones(n)
        row_residual=maximum(abs(row_sums[k]-case20_row_sum(support,k,sample.t))
                             for k in 1:n)
        row_pass=all(abs(row_sums[k]-case20_row_sum(support,k,sample.t)) <=
                     128eps(Float64)*max(1.0,abs(row_sums[k])) for k in 1:n)
        push!(checks,Dict("label"=>sample.label,"time"=>sample.t,
            "rhs_max_abs_residual"=>rhs_residual,"rhs_pass"=>rhs_pass,
            "matrix_action_max_abs_residual"=>action_residual,
            "matrix_action_pass"=>action_pass,
            "independent_directional_difference_max_abs_residual"=>fd_residual,
            "independent_directional_difference_scaled_residual"=>fd_scaled,
            "independent_directional_difference_pass"=>fd_pass,
            "row_sum_max_abs_residual"=>row_residual,"row_sum_pass"=>row_pass))
    end
    top_boundary=all(support.upper_slot[k]==0 for k in 1:n if
                     SparseJacobianSupport.row_quantum_numbers(k-1)[1]==levels-1)
    J=copy(support.prototype);y=Float64.(descriptor.initial_state)
    for _ in 1:10
        case20_jacobian!(J,y,support,0.0)
    end
    allocations=[@allocated case20_jacobian!(J,y,support,0.0) for _ in 1:10]
    pass=all(x["rhs_pass"] && x["matrix_action_pass"] &&
             x["independent_directional_difference_pass"] && x["row_sum_pass"]
             for x in checks) && top_boundary && maximum(allocations)==0
    result=Dict("schema"=>"vigilode.case20-sparse-jacobian-probe.v1",
                "descriptor_sha256"=>descriptor_sha,
                "support_source_sha256"=>bytes2hex(sha256(read(joinpath(@__DIR__,"sparse_jacobian_support.jl")))),
                "size"=>size,"dimension"=>n,"checks"=>checks,
                "top_boundary_has_no_high_neighbor"=>top_boundary,
                "warmed_jacobian_allocated_bytes"=>allocations,
                "memory"=>case20_pattern_bytes(support),"pass"=>pass)
    open(joinpath(outdir,"PROBE_RESULT.json"),"w") do io
        write(io,JSON3.write(result));write(io,'\n')
    end
    pass || error("case20 sparse Jacobian probe failed")
end

length(ARGS)==2 || error("descriptor path and new output dir required")
run_probe(ARGS[1],ARGS[2])
