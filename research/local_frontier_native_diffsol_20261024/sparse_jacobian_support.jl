module SparseJacobianSupport

using SparseArrays
export Case20SparseSupport, build_case20_support, case20_rhs!, case20_jvp!,
       case20_jacobian!, case20_tgrad!, case20_pattern_bytes, case20_row_sum

# Frozen case 20: 0 <= l < 32 standard (4 smoke), k=l^2+l+m,
# -l <= m <= l. The top moment is identically absent. The matter bath is fixed.
struct Case20SparseSupport
    n::Int
    levels::Int
    prototype::SparseMatrixCSC{Float64,Int}
    diagonal_slot::Vector{Int}
    lower_slot::Vector{Int}
    upper_slot::Vector{Int}
    lower_coefficient::Vector{Float64}
    upper_coefficient::Vector{Float64}
    damping::Vector{Float64}
end

@inline function row_quantum_numbers(k::Int)
    l=floor(Int,sqrt(Float64(k)))
    return l,k-l*l-l
end

function csc_slot(matrix::SparseMatrixCSC{Float64,Int},row::Int,column::Int)
    for position in nzrange(matrix,column)
        if matrix.rowval[position]==row
            return position
        end
    end
    error("missing sparse Jacobian structural entry at ($row,$column)")
end

function build_case20_support(levels::Int)
    levels in (4,32) || error("case20 frozen levels are 4 smoke or 32 standard")
    n=levels*levels
    rows=Int[];columns=Int[];values=Float64[]
    lowcol=zeros(Int,n);highcol=zeros(Int,n)
    lowcoef=zeros(n);highcoef=zeros(n);damping=zeros(n)
    for k in 0:n-1
        row=k+1
        l,m=row_quantum_numbers(k)
        l<levels || error("invalid l at row $row")
        push!(rows,row);push!(columns,row);push!(values,1.0)
        if l>0 && abs(m)<l
            col=l*l-l+m+1
            coefficient=sqrt(Float64(l*l-m*m))/sqrt(Float64(4*l*l-1))
            lowcol[row]=col;lowcoef[row]=coefficient
            push!(rows,row);push!(columns,col);push!(values,1.0)
        end
        if l+1<levels
            z=l+1
            col=z*z+z+m+1
            coefficient=sqrt(Float64(z*z-m*m))/sqrt(Float64(4*z*z-1))
            highcol[row]=col;highcoef[row]=coefficient
            push!(rows,row);push!(columns,col);push!(values,1.0)
        end
        damping[row]=l==0 ? 0.0 : l==2 ? 0.9 : 1.0
    end
    prototype=sparse(rows,columns,values,n,n)
    diagonal_slot=Vector{Int}(undef,n)
    lower_slot=zeros(Int,n);upper_slot=zeros(Int,n)
    for row in 1:n
        diagonal_slot[row]=csc_slot(prototype,row,row)
        if lowcol[row]!=0
            lower_slot[row]=csc_slot(prototype,row,lowcol[row])
        end
        if highcol[row]!=0
            upper_slot[row]=csc_slot(prototype,row,highcol[row])
        end
    end
    support=Case20SparseSupport(n,levels,prototype,diagonal_slot,lower_slot,
        upper_slot,lowcoef,highcoef,damping)
    case20_jacobian!(prototype,zeros(n),support,0.0)
    support
end

@inline case20_gamma(t::Float64)=1e4/(1.0+t)^4

function case20_rhs!(out::AbstractVector{Float64},state::AbstractVector{Float64},
                     support::Case20SparseSupport,t::Float64)
    g=case20_gamma(t)
    @inbounds for row in 1:support.n
        low=0.0; high=0.0
        l,m=row_quantum_numbers(row-1)
        if support.lower_slot[row]!=0
            low=support.lower_coefficient[row]*state[l*l-l+m+1]
        end
        if support.upper_slot[row]!=0
            z=l+1
            high=support.upper_coefficient[row]*state[z*z+z+m+1]
        end
        out[row]=low-high-support.damping[row]*g*state[row]
    end
    nothing
end

# At fixed t the frozen hierarchy RHS is exactly linear in its state.
function case20_jvp!(out::AbstractVector{Float64},direction::AbstractVector{Float64},
                     support::Case20SparseSupport,t::Float64)
    case20_rhs!(out,direction,support,t)
end

# Partial derivative with respect to t at fixed state. Streaming coefficients
# are constant; only -d_i*1e4/(1+t)^4 varies with time.
function case20_tgrad!(out::AbstractVector{Float64},state::AbstractVector{Float64},
                       support::Case20SparseSupport,t::Float64)
    denominator=(1.0+t)^5
    @inbounds for row in 1:support.n
        d=support.damping[row]
        out[row]=d==0.0 ? 0.0 : 4e4*d*state[row]/denominator
    end
    nothing
end

function case20_jacobian!(matrix::SparseMatrixCSC{Float64,Int},
                          state::AbstractVector{Float64},
                          support::Case20SparseSupport,t::Float64)
    size(matrix)==(support.n,support.n) || error("sparse Jacobian dimension mismatch")
    length(matrix.nzval)==length(support.prototype.nzval) || error("sparse Jacobian nnz mismatch")
    # The fixed prototype is passed through SciML. Verify CSC structure before
    # filling so a different pattern cannot receive these precomputed slots.
    matrix.colptr==support.prototype.colptr || error("sparse Jacobian columns changed")
    matrix.rowval==support.prototype.rowval || error("sparse Jacobian rows changed")
    g=case20_gamma(t)
    @inbounds for row in 1:support.n
        matrix.nzval[support.diagonal_slot[row]]=-support.damping[row]*g
        low=support.lower_slot[row]
        if low!=0
            matrix.nzval[low]=support.lower_coefficient[row]
        end
        high=support.upper_slot[row]
        if high!=0
            matrix.nzval[high]=-support.upper_coefficient[row]
        end
    end
    nothing
end

function case20_pattern_bytes(support::Case20SparseSupport)
    # Setup-only accounting, never invoked by RHS/Jacobian hot loops.
    Dict("nnz"=>nnz(support.prototype),
         "prototype_summarysize_bytes"=>Base.summarysize(support.prototype),
         "support_summarysize_bytes"=>Base.summarysize(support))
end

function case20_row_sum(support::Case20SparseSupport,row::Int,t::Float64)
    support.lower_coefficient[row]-support.upper_coefficient[row]-
        support.damping[row]*case20_gamma(t)
end

end # module
