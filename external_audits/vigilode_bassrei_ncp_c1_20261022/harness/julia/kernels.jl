# Julia ports of the campaign's RHS and f_t (Base Julia only): the P02 photon
# operator (`lrs_photon_fv_support/mod.rs`), the P03 bordered model
# (`border.rs`) and the P04 photon-gas chemistry (`chem.rs`), written in the
# Rust operation order. Generic in the element type (ForwardDiff duals).
# Parity against the VigilODE runner probe is a campaign gate (<= 1e-13).

module Kernels

export build_model, rhs!, tgrad!, nstate, structural_pattern

# ---------------------------------------------------------------------------
# P02 layout, geometry, absorber
# ---------------------------------------------------------------------------

struct Layout
    q_edges::Vector{Float64}
    mu_edges::Vector{Float64}
    dq::Vector{Float64}
    dm::Vector{Float64}
    nq::Int
    nm::Int
    c::Vector{Float64}
    p2::Vector{Float64}
    comoving::Bool
    closed::Bool
    n::Int
end

function Layout(qe::Vector{Float64}, me::Vector{Float64}, coordinate::String, boundary::String)
    dq = [qe[k+1] - qe[k] for k in 1:length(qe)-1]
    dm = [me[k+1] - me[k] for k in 1:length(me)-1]
    c = [3.0 * ((me[j+1]^3 - me[j]^3) / (3.0 * dm[j])) - 1.0 for j in eachindex(dm)]
    p2 = [((me[j+1]^3 - me[j+1]) - (me[j]^3 - me[j])) / (2.0 * dm[j]) for j in eachindex(dm)]
    nq, nm = length(dq), length(dm)
    Layout(qe, me, dq, dm, nq, nm, c, p2, coordinate == "comoving", boundary == "closed", 2 + nq * (nm + 1))
end

@inline cell(L::Layout, i, j) = 1 + (i - 1) * (L.nm + 1) + j
@inline absorbed(L::Layout, i) = 2 + (i - 1) * (L.nm + 1) + L.nm

struct Geometry
    constant::Bool
    h::Float64
    s::Float64
    hh::Float64
    s0::Float64
    p::Float64
end

function Geometry(d::Dict)
    if d["kind"] == "constant"
        Geometry(true, Float64(d["h"]), Float64(d["s"]), 0.0, 0.0, 0.0)
    else
        Geometry(false, 0.0, 0.0, Float64(d["hh"]), Float64(d["s0"]), Float64(d["p"]))
    end
end

@inline function at(g::Geometry, t)
    if g.constant
        return (g.h, g.s, 0.0, 0.0)
    end
    x = 1.0 + t
    s = g.s0 * x^(-g.p)
    return (g.hh / x, s, -g.hh / (x * x), -g.p * s / x)
end

@inline ln_a(g::Geometry, t) = g.constant ? g.h * t : g.hh * log1p(t)

struct Absorber
    k0::Float64
    q_chi::Float64
end

# ---------------------------------------------------------------------------
# P02 operator
# ---------------------------------------------------------------------------

struct P02Model
    L::Layout
    geom::Geometry
    gamma::Float64
    absorber::Union{Nothing,Absorber}
end

nstate(M::P02Model) = M.L.n

@inline function drift(M::P02Model, t)
    h, s, dh, ds = at(M.geom, t)
    return M.L.comoving ? (0.0, s, 0.0, ds) : (h, s, dh, ds)
end

@inline function kbar(L::Layout, geom::Geometry, a::Absorber, i, t)
    x_chi, rate = L.comoving ? (a.q_chi + ln_a(geom, t), at(geom, t)[1]) : (a.q_chi, 0.0)
    lo, hi = L.q_edges[i], L.q_edges[i+1]
    scale = a.k0 / (3.0 * L.dq[i])
    below = rate < 0.0 ? hi < x_chi : hi <= x_chi
    above = rate > 0.0 ? lo > x_chi : lo >= x_chi
    if below
        return (zero(scale * x_chi), zero(scale * x_chi))
    elseif above
        k = -scale * exp(-3.0 * (lo - x_chi)) * expm1(-3.0 * L.dq[i])
        return (k, 3.0 * rate * k)
    else
        tail = -3.0 * (hi - x_chi)
        return (-scale * expm1(tail), -3.0 * scale * rate * exp(tail))
    end
end

# The oracle's transport loops: upwind direction from (h, s), flux velocity
# from (vh, vs); accumulates into out (zeroed by the caller).
function transport!(out, L::Layout, h, s, vh, vs, y)
    nq, nm = L.nq, L.nm
    hi = L.n
    @inbounds for j in 1:nm
        select = -h - s * L.c[j]
        velocity = -vh - vs * L.c[j]
        for k in 0:nq
            if L.closed && (k == 0 || k == nq)
                continue
            end
            up = select >= 0.0 ? k - 1 : k
            val = (0 <= up < nq) ? y[cell(L, up + 1, j)] / (L.dq[up+1] * L.dm[j]) : zero(eltype(y))
            flux = velocity * val * L.dm[j]
            if k > 0
                out[cell(L, k, j)] -= flux
            else
                out[1] -= flux
            end
            if k < nq
                out[cell(L, k + 1, j)] += flux
            else
                out[hi] += flux
            end
        end
    end
    @inbounds for i in 1:nq
        for k in 1:nm-1
            mu = L.mu_edges[k+1]
            select = -3.0 * s * mu * (1.0 - mu * mu)
            velocity = -3.0 * vs * mu * (1.0 - mu * mu)
            up = select >= 0.0 ? k - 1 : k
            flux = velocity * (y[cell(L, i, up + 1)] / (L.dq[i] * L.dm[up+1])) * L.dq[i]
            out[cell(L, i, k)] -= flux
            out[cell(L, i, k + 1)] += flux
        end
    end
    return out
end

function thomson!(out, L::Layout, gamma, y)
    @inbounds for i in 1:L.nq
        m0 = zero(eltype(y))
        m2 = zero(eltype(y))
        for j in 1:L.nm
            n = y[cell(L, i, j)]
            m0 += n
            m2 += L.p2[j] * n
        end
        for j in 1:L.nm
            n = y[cell(L, i, j)]
            out[cell(L, i, j)] += gamma * (L.dm[j] / 2.0 * (m0 + 0.5 * L.p2[j] * m2) - n)
        end
    end
    return out
end

function absorb!(out, L::Layout, geom::Geometry, a::Absorber, t, y, derivative::Bool)
    @inbounds for i in 1:L.nq
        k, dk = kbar(L, geom, a, i, t)
        kk = derivative ? dk : k
        acc = zero(eltype(out))
        for j in 1:L.nm
            rate = kk * y[cell(L, i, j)]
            out[cell(L, i, j)] -= rate
            acc += rate
        end
        out[absorbed(L, i)] += acc
    end
    return out
end

function rhs!(out, y, M::P02Model, t)
    fill!(out, zero(eltype(out)))
    h, s, _, _ = drift(M, t)
    transport!(out, M.L, h, s, h, s, y)
    M.gamma != 0.0 && thomson!(out, M.L, M.gamma, y)
    M.absorber === nothing || absorb!(out, M.L, M.geom, M.absorber, t, y, false)
    return out
end

function tgrad!(out, y, M::P02Model, t)
    fill!(out, zero(eltype(out)))
    h, s, dh, ds = drift(M, t)
    transport!(out, M.L, h, s, dh, ds, y)
    M.absorber === nothing || absorb!(out, M.L, M.geom, M.absorber, t, y, true)
    return out
end

# ---------------------------------------------------------------------------
# P03 bordered model
# ---------------------------------------------------------------------------

struct BorderedModel
    L::Layout
    geom::Geometry
    absorber::Union{Nothing,Absorber}
    k::Float64
    alpha::Float64
    beta::Float64
    kappa::Float64
    tau::Float64
    gamma0::Float64
    w::Vector{Float64}
end

nstate(M::BorderedModel) = M.L.n + 3

function d_gamma(M::BorderedModel, y)
    L = M.L
    total = zero(eltype(y))
    @inbounds for i in 1:L.nq
        m2 = zero(eltype(y))
        for j in 1:L.nm
            m2 += L.p2[j] * y[cell(L, i, j)]
        end
        total += M.w[i] * m2
    end
    return total
end

function rhs!(out, y, M::BorderedModel, t)
    fill!(out, zero(eltype(out)))
    np = M.L.n
    is, ix, idm = np + 1, np + 2, np + 3
    h, _, _, _ = at(M.geom, t)
    s, xe, dm = y[is], y[ix], y[idm]
    transport!(out, M.L, h, s, h, s, y)
    gamma = M.gamma0 * xe
    gamma != 0.0 && thomson!(out, M.L, gamma, y)
    M.absorber === nothing || absorb!(out, M.L, M.geom, M.absorber, t, y, false)
    out[is] = -3.0 * h * s + M.k * (d_gamma(M, y) + dm)
    out[ix] = M.beta * (1.0 - xe) - M.alpha * xe * xe
    out[idm] = -(dm + M.kappa * s) / M.tau
    return out
end

function tgrad!(out, y, M::BorderedModel, t)
    fill!(out, zero(eltype(out)))
    np = M.L.n
    h, _, dh, _ = at(M.geom, t)
    s = y[np+1]
    transport!(out, M.L, h, s, dh, 0.0, y)
    out[np+1] = -3.0 * dh * s
    return out
end

# ---------------------------------------------------------------------------
# P04 photon-gas chemistry
# ---------------------------------------------------------------------------

const W_GAMMA, HI_, HII_, HEI_, HEII_, HEIII_, U_, DM_, WM_, EESC_, RE_, S_ = 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11
const REACTANT = (HI_, HEI_, HEII_)
const PRODUCT = (HII_, HEII_, HEIII_)

struct CaseA
    T::Vector{Float64}
    x::Vector{Float64}
    coef::Array{Float64,3}   # (interval, column, 4): [c0, c1, c2, c3]
end

_sign(v) = v > 0.0 ? 1.0 : (v < 0.0 ? -1.0 : 0.0)

function _edge(h0, h1, m0, m1)
    d = ((2.0 * h0 + h1) * m0 - h0 * m1) / (h0 + h1)
    if _sign(d) != _sign(m0)
        return 0.0
    elseif _sign(m0) != _sign(m1) && abs(d) > 3.0 * abs(m0)
        return 3.0 * m0
    end
    return d
end

function CaseA(path::String)
    lines = readlines(path)
    rows = [parse.(Float64, split(l, ',')) for l in lines[2:end] if !isempty(strip(l))]
    m = length(rows)
    T = [r[1] for r in rows]
    x = log.(T)
    ncol = length(rows[1]) - 1
    y = [log(rows[k][c+1]) for k in 1:m, c in 1:ncol]
    h = [x[k+1] - x[k] for k in 1:m-1]
    coef = zeros(m - 1, ncol, 4)
    for c in 1:ncol
        slopes = [(y[k+1, c] - y[k, c]) / h[k] for k in 1:m-1]
        d = zeros(m)
        for k in 2:m-1
            mp, mk = slopes[k-1], slopes[k]
            if _sign(mk) != _sign(mp) || mk == 0.0 || mp == 0.0
                d[k] = 0.0
            else
                w1 = 2.0 * h[k] + h[k-1]
                w2 = h[k] + 2.0 * h[k-1]
                whmean = (w1 / mp + w2 / mk) / (w1 + w2)
                d[k] = 1.0 / whmean
            end
        end
        d[1] = _edge(h[1], h[2], slopes[1], slopes[2])
        d[m] = _edge(h[m-1], h[m-2], slopes[m-1], slopes[m-2])
        for k in 1:m-1
            slope = (y[k+1, c] - y[k, c]) / h[k]
            tt = (d[k] + d[k+1] - 2.0 * slope) / h[k]
            coef[k, c, 1] = tt / h[k]
            coef[k, c, 2] = (slope - d[k]) / h[k] - tt
            coef[k, c, 3] = d[k]
            coef[k, c, 4] = y[k, c]
        end
    end
    CaseA(T, x, coef)
end

struct DomainError_ <: Exception
    T::Float64
end

# exp(P(ln T)) per column (8); throws DomainError_ outside [T_min, T_max].
function casea_eval(tab::CaseA, temperature)
    tv = _value(temperature)
    (tv >= tab.T[1] && tv <= tab.T[end]) || throw(DomainError_(tv))
    x = log(temperature)
    m = length(tab.x)
    k = clamp(searchsortedlast(tab.x, _value(x)), 1, m - 1)
    s = x - tab.x[k]
    ss, sss = s * s, s * s * s
    return ntuple(c -> exp(tab.coef[k, c, 4] + tab.coef[k, c, 3] * s + tab.coef[k, c, 2] * ss +
                           tab.coef[k, c, 1] * sss), 8)
end

_value(x::AbstractFloat) = Float64(x)
_value(x) = _value(x.value)   # a dual number's value (ColorJac.Dual, ForwardDiff.Dual)

struct ChemModel
    L::Layout
    tab::CaseA
    nph::Int
    ebar::Vector{Float64}
    sbar::Matrix{Float64}     # (3, nq)
    C::Float64
    KB::Float64
    EV_ERG::Float64
    BIND::NTuple{3,Float64}
    SIGMA_T::Float64
    k_d_amp::Float64
    age0::Float64
    tau_m::Float64
end

nstate(M::ChemModel) = M.nph + 12

@inline function background(M::ChemModel, t)
    h = (2.0 / 3.0) / (M.age0 + t)
    a_inv3 = (1.0 + t / M.age0)^(-2.0)
    return (h, -1.5 * h * h, a_inv3, -3.0 * h * a_inv3)
end

function chem_eval!(out, y, M::ChemModel, t, RHS::Bool)
    L = M.L
    b = M.nph
    sl(k) = b + k + 1
    n_hi, n_hii, n_hei, n_heii, n_heiii, u = y[sl(HI_)], y[sl(HII_)], y[sl(HEI_)], y[sl(HEII_)], y[sl(HEIII_)], y[sl(U_)]
    ne = n_hii + n_heii + 2.0 * n_heiii
    particles = (n_hi + n_hii) + (n_hei + n_heii + n_heiii) + ne
    temperature = u / (1.5 * M.KB * particles)
    v = casea_eval(M.tab, temperature)
    fill!(out, zero(eltype(out)))
    bh, bdh, ba3, bda3 = background(M, t)
    s = y[sl(S_)]
    vh, vs, rate, hw = RHS ? (bh, s, ba3, bh) : (bdh, zero(s), bda3, bdh)
    nq, nm = L.nq, L.nm
    # §2 transport, then the discrete photon work.
    transport!(out, L, bh, s, vh, vs, y)
    work = zero(eltype(out))
    @inbounds for i in 1:nq, j in 1:nm
        work += M.ebar[i] * out[cell(L, i, j)]
    end
    work += M.ebar[1] * out[1]
    work += M.ebar[nq] * out[L.n]
    out[sl(W_GAMMA)] = -work
    # §3 Thomson.
    gam = M.C * M.SIGMA_T * (ne * rate)
    thomson!(out, L, gam, y)
    # §4 photoionization, s outer, then i, then j.
    @inbounds for sp in 1:3
        reactant, product = sl(REACTANT[sp]), sl(PRODUCT[sp])
        ns = y[reactant]
        for i in 1:nq
            k = M.C * M.sbar[sp, i] * (ns * rate)
            heat = M.ebar[i] - M.BIND[sp]
            ab = absorbed(L, i)
            for j in 1:nm
                c = cell(L, i, j)
                r = k * y[c]
                out[c] -= r
                out[ab] += r
                out[reactant] -= r
                out[product] += r
                out[sl(U_)] += heat * r
            end
        end
    end
    # §5 recombination, cooling, escape and electron ledgers.
    rr_hii, rr_heii, rr_heiii, dr_heii, c_hii, c_heii, c_heiii, cdr_heii = v
    ne_p = ne * rate
    r_h = rr_hii * ne_p * n_hii
    r2 = (rr_heii + dr_heii) * ne_p * n_heii
    r3 = rr_heiii * ne_p * n_heiii
    out[sl(HII_)] -= r_h
    out[sl(HI_)] += r_h
    out[sl(HEII_)] -= r2
    out[sl(HEI_)] += r2
    out[sl(HEIII_)] -= r3
    out[sl(HEII_)] += r3
    lam = ne_p * (n_hii * c_hii + n_heii * c_heii + n_heiii * c_heiii + n_heii * cdr_heii) / M.EV_ERG
    out[sl(U_)] -= lam
    out[sl(EESC_)] += M.BIND[1] * r_h + M.BIND[2] * r2 + M.BIND[3] * r3 + lam
    out[sl(RE_)] += r_h + r2 + r3
    # §6 matter stress, gas energy work and shear.
    d_m = y[sl(DM_)]
    out[sl(DM_)] += RHS ? (-2.0 * hw * d_m - d_m / M.tau_m - 4.0 * s * u) : (-2.0 * hw * d_m)
    a1 = 2.0 * hw * u
    a2 = RHS ? 2.0 * s * d_m : zero(s * d_m)
    out[sl(U_)] += -a1 - a2
    out[sl(WM_)] += a1 + a2
    dg = zero(eltype(y))
    @inbounds for i in 1:nq
        quad = zero(eltype(y))
        for j in 1:nm
            quad += L.p2[j] * y[cell(L, i, j)]
        end
        dg += M.ebar[i] * quad
    end
    out[sl(S_)] = -3.0 * hw * s + M.k_d_amp * (dg + d_m) * rate
    return out
end

function _guarded!(out, y, M::ChemModel, t, RHS)
    try
        chem_eval!(out, y, M, t, RHS)
    catch e
        e isa DomainError_ || rethrow()
        fill!(out, NaN)   # the runner's typed CASE_A_T_DOMAIN refusal: a rejected trial
    end
    return out
end

rhs!(out, y, M::ChemModel, t) = _guarded!(out, y, M, t, true)
tgrad!(out, y, M::ChemModel, t) = _guarded!(out, y, M, t, false)

# ---------------------------------------------------------------------------
# Construction from the campaign's model description
# ---------------------------------------------------------------------------

fvec(a) = Float64[Float64(x) for x in a]

function build_model(d::Dict)
    fam = d["family"]
    if fam == "p02" || fam == "bordered"
        lay = d["layout"]
        L = Layout(fvec(lay["q_edges"]), fvec(lay["mu_edges"]), String(lay["coordinate"]), String(lay["boundary"]))
        geom = Geometry(d["geometry"])
        ab = d["absorber"] === nothing ? nothing : Absorber(Float64(d["absorber"]["k0"]), Float64(d["absorber"]["q_chi"]))
        if fam == "p02"
            return P02Model(L, geom, Float64(d["gamma"]), ab)
        end
        bd = d["border"]
        w = [exp(L.q_edges[i]) * expm1(L.dq[i]) / L.dq[i] for i in 1:L.nq]
        return BorderedModel(L, geom, ab, Float64(bd["k"]), Float64(bd["alpha"]), Float64(bd["beta"]),
                             Float64(bd["kappa"]), Float64(bd["tau"]), Float64(bd["gamma0"]), w)
    elseif fam == "chem"
        st = d["chem_static"]
        L0 = Layout(fvec(st["q_edges"]), fvec(st["mu_edges"]), "physical", "outflow")
        # dq, dm, p2 exactly as the runner's (its powf(3.0) cell averages)
        L = Layout(L0.q_edges, L0.mu_edges, fvec(st["dq"]), fvec(st["dm"]), L0.nq, L0.nm, L0.c, fvec(st["p2"]),
                   false, false, L0.n)
        cst = st["constants"]
        sb = st["sbar"]
        sbar = [Float64(sb[s][i]) for s in 1:3, i in 1:L.nq]
        return ChemModel(L, CaseA(String(d["table_path"])), Int(st["n_ph"]), fvec(st["Ebar"]), sbar,
                         Float64(cst["C"]), Float64(cst["KB"]), Float64(cst["EV_ERG"]),
                         Tuple(fvec(cst["BIND"])), Float64(cst["SIGMA_T"]), Float64(cst["K_D_amp"]),
                         Float64(cst["t0"]), Float64(d["params"]["tau_m"]))
    end
    error("unknown family $fam")
end

# A structural superset of the Jacobian's nonzeros (1-based (row, col) lists):
# the photon band |r - c| <= nm + 1, plus full border/matter rows and columns.
function structural_pattern(M)
    L = M.L
    np, n = L.n, nstate(M)
    bw = L.nm + 1
    rows, cols = Int[], Int[]
    for c in 1:np, r in max(1, c - bw):min(np, c + bw)
        push!(rows, r); push!(cols, c)
    end
    for e in np+1:n
        for c in 1:n
            push!(rows, e); push!(cols, c)
        end
        for r in 1:np
            push!(rows, r); push!(cols, e)
        end
    end
    return rows, cols
end

end # module
