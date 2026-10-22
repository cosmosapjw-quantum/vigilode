# Exact compressed Jacobian by forward-mode dual numbers (Base Julia only):
# columns are greedily coloured so that no two columns of one colour share a
# row of the structural pattern, and each chunk of CH colours is one RHS
# evaluation in dual arithmetic. For the linear P02 operator this is the
# analytic Jacobian to rounding; for the bordered and chemistry models it is
# forward-mode AD of the ported RHS. No external AD package is needed.

module ColorJac

export Dual, ColoredJacobian, colored_jacobian!, value

struct Dual{N} <: Real
    v::Float64
    p::NTuple{N,Float64}
end

Dual{N}(x::Real) where {N} = Dual{N}(Float64(x), ntuple(_ -> 0.0, N))
value(x::Dual) = x.v
value(x::Real) = Float64(x)

Base.convert(::Type{Dual{N}}, x::Real) where {N} = Dual{N}(x)
Base.convert(::Type{Dual{N}}, x::Dual{N}) where {N} = x
Base.promote_rule(::Type{Dual{N}}, ::Type{<:Real}) where {N} = Dual{N}
Base.zero(::Type{Dual{N}}) where {N} = Dual{N}(0.0)
Base.zero(::Dual{N}) where {N} = Dual{N}(0.0)
Base.one(::Type{Dual{N}}) where {N} = Dual{N}(1.0)
Base.float(x::Dual) = x

@inline _map(f, a::NTuple{N,Float64}, b::NTuple{N,Float64}) where {N} = ntuple(i -> f(a[i], b[i]), N)
@inline _scale(a::NTuple{N,Float64}, s::Float64) where {N} = ntuple(i -> a[i] * s, N)

Base.:+(a::Dual{N}, b::Dual{N}) where {N} = Dual{N}(a.v + b.v, _map(+, a.p, b.p))
Base.:-(a::Dual{N}, b::Dual{N}) where {N} = Dual{N}(a.v - b.v, _map(-, a.p, b.p))
Base.:-(a::Dual{N}) where {N} = Dual{N}(-a.v, _scale(a.p, -1.0))
Base.:*(a::Dual{N}, b::Dual{N}) where {N} = Dual{N}(a.v * b.v, ntuple(i -> a.p[i] * b.v + a.v * b.p[i], N))
Base.:/(a::Dual{N}, b::Dual{N}) where {N} =
    Dual{N}(a.v / b.v, ntuple(i -> (a.p[i] * b.v - a.v * b.p[i]) / (b.v * b.v), N))
Base.:+(a::Dual{N}, b::Real) where {N} = Dual{N}(a.v + b, a.p)
Base.:+(a::Real, b::Dual{N}) where {N} = Dual{N}(a + b.v, b.p)
Base.:-(a::Dual{N}, b::Real) where {N} = Dual{N}(a.v - b, a.p)
Base.:-(a::Real, b::Dual{N}) where {N} = Dual{N}(a - b.v, _scale(b.p, -1.0))
Base.:*(a::Dual{N}, b::Real) where {N} = Dual{N}(a.v * b, _scale(a.p, Float64(b)))
Base.:*(a::Real, b::Dual{N}) where {N} = Dual{N}(a * b.v, _scale(b.p, Float64(a)))
Base.:/(a::Dual{N}, b::Real) where {N} = Dual{N}(a.v / b, _scale(a.p, 1.0 / b))
Base.:/(a::Real, b::Dual{N}) where {N} = Dual{N}(a / b.v, _scale(b.p, -a / (b.v * b.v)))
Base.:^(a::Dual{N}, b::Real) where {N} = Dual{N}(a.v^b, _scale(a.p, b * a.v^(b - 1)))
Base.:^(a::Dual{N}, b::Integer) where {N} = Dual{N}(a.v^b, _scale(a.p, b * a.v^(b - 1)))
Base.exp(a::Dual{N}) where {N} = (e = exp(a.v); Dual{N}(e, _scale(a.p, e)))
Base.expm1(a::Dual{N}) where {N} = Dual{N}(expm1(a.v), _scale(a.p, exp(a.v)))
Base.log(a::Dual{N}) where {N} = Dual{N}(log(a.v), _scale(a.p, 1.0 / a.v))
Base.log1p(a::Dual{N}) where {N} = Dual{N}(log1p(a.v), _scale(a.p, 1.0 / (1.0 + a.v)))
Base.abs(a::Dual{N}) where {N} = a.v < 0 ? -a : a
for op in (:<, :<=, :>, :>=, :(==))
    @eval Base.$op(a::Dual, b::Dual) = $op(a.v, b.v)
    @eval Base.$op(a::Dual, b::Real) = $op(a.v, b)
    @eval Base.$op(a::Real, b::Dual) = $op(a, b.v)
end
Base.isfinite(a::Dual) = isfinite(a.v)
Base.isnan(a::Dual) = isnan(a.v)
Base.getproperty(x::Dual, s::Symbol) = s === :value ? getfield(x, :v) : getfield(x, s)

const CH = 8

struct ColoredJacobian
    n::Int
    rows::Vector{Int}
    cols::Vector{Int}
    colors::Vector{Int}
    ncolors::Int
    nzidx::Vector{Int}       # position of (rows[k], cols[k]) in the CSC nonzeros
    colptr::Vector{Int}
    rowval::Vector{Int}
    yd::Vector{Dual{CH}}
    od::Vector{Dual{CH}}
end

"""Greedy colouring of the column-intersection graph of the pattern."""
function ColoredJacobian(n::Int, rows::Vector{Int}, cols::Vector{Int})
    # unique, CSC order
    keys = unique(sort([(c, r) for (r, c) in zip(rows, cols)]))
    cols_s = [k[1] for k in keys]
    rows_s = [k[2] for k in keys]
    colptr = ones(Int, n + 1)
    for c in cols_s
        colptr[c+1] += 1
    end
    for c in 1:n
        colptr[c+1] += colptr[c] - 1
    end
    rowval = rows_s
    # rows -> columns lists
    bycol = [Int[] for _ in 1:n]
    byrow = [Int[] for _ in 1:n]
    for (r, c) in zip(rows_s, cols_s)
        push!(bycol[c], r)
        push!(byrow[r], c)
    end
    colors = zeros(Int, n)
    forbidden = zeros(Int, n + 1)
    ncolors = 0
    for c in 1:n
        for r in bycol[c], c2 in byrow[r]
            col2 = colors[c2]
            col2 > 0 && (forbidden[col2] = c)
        end
        k = 1
        while forbidden[k] == c
            k += 1
        end
        colors[c] = k
        ncolors = max(ncolors, k)
    end
    nzidx = collect(1:length(rows_s))
    ColoredJacobian(n, rows_s, cols_s, colors, ncolors, nzidx, colptr, rowval,
                    Vector{Dual{CH}}(undef, n), Vector{Dual{CH}}(undef, n))
end

"""Fill `nzval` (CSC order of the pattern) with the Jacobian of `f!(out, y, t)`."""
function colored_jacobian!(nzval::AbstractVector{Float64}, f!, y::AbstractVector{Float64}, t, cj::ColoredJacobian)
    fill!(nzval, 0.0)
    for c0 in 1:CH:cj.ncolors
        @inbounds for j in 1:cj.n
            slot = cj.colors[j] - c0 + 1
            cj.yd[j] = (1 <= slot <= CH) ? Dual{CH}(y[j], ntuple(k -> k == slot ? 1.0 : 0.0, CH)) : Dual{CH}(y[j])
        end
        f!(cj.od, cj.yd, t)
        @inbounds for k in eachindex(cj.rows)
            slot = cj.colors[cj.cols[k]] - c0 + 1
            if 1 <= slot <= CH
                nzval[cj.nzidx[k]] = cj.od[cj.rows[k]].p[slot]
            end
        end
    end
    return nzval
end

end # module
