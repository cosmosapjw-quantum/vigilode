# Minimal JSON reader/writer in Base Julia (no packages), for the campaign's
# own request/response files. Numbers parse with Base.parse(Float64) (exact
# for the shortest round-trip literals Python writes); integers without a
# fraction or exponent become Int. Writing uses repr(::Float64), the shortest
# round-trip form; non-finite floats are written as null.

module MiniJSON

export jparse, jwrite, jwrite_string

mutable struct Cursor
    s::String
    i::Int
end

@inline peek(c::Cursor) = c.i <= ncodeunits(c.s) ? codeunit(c.s, c.i) : 0x00

function skipws!(c::Cursor)
    while c.i <= ncodeunits(c.s)
        b = codeunit(c.s, c.i)
        (b == 0x20 || b == 0x0a || b == 0x0d || b == 0x09) || break
        c.i += 1
    end
end

function expect!(c::Cursor, lit::String)
    n = ncodeunits(lit)
    c.i + n - 1 <= ncodeunits(c.s) && view(codeunits(c.s), c.i:c.i+n-1) == codeunits(lit) ||
        error("JSON: expected $lit at byte $(c.i)")
    c.i += n
end

function parse_value(c::Cursor)
    skipws!(c)
    b = peek(c)
    if b == UInt8('{')
        c.i += 1
        d = Dict{String,Any}()
        skipws!(c)
        if peek(c) == UInt8('}')
            c.i += 1
            return d
        end
        while true
            skipws!(c)
            k = parse_string(c)
            skipws!(c)
            peek(c) == UInt8(':') || error("JSON: ':' expected at byte $(c.i)")
            c.i += 1
            d[k] = parse_value(c)
            skipws!(c)
            b = peek(c)
            c.i += 1
            b == UInt8('}') && return d
            b == UInt8(',') || error("JSON: ',' or '}' expected at byte $(c.i - 1)")
        end
    elseif b == UInt8('[')
        c.i += 1
        a = Any[]
        skipws!(c)
        if peek(c) == UInt8(']')
            c.i += 1
            return a
        end
        while true
            push!(a, parse_value(c))
            skipws!(c)
            b = peek(c)
            c.i += 1
            b == UInt8(']') && return a
            b == UInt8(',') || error("JSON: ',' or ']' expected at byte $(c.i - 1)")
        end
    elseif b == UInt8('"')
        return parse_string(c)
    elseif b == UInt8('t')
        expect!(c, "true"); return true
    elseif b == UInt8('f')
        expect!(c, "false"); return false
    elseif b == UInt8('n')
        expect!(c, "null"); return nothing
    else
        return parse_number(c)
    end
end

function parse_string(c::Cursor)
    peek(c) == UInt8('"') || error("JSON: string expected at byte $(c.i)")
    c.i += 1
    io = IOBuffer()
    while true
        c.i <= ncodeunits(c.s) || error("JSON: unterminated string")
        b = codeunit(c.s, c.i)
        if b == UInt8('"')
            c.i += 1
            return String(take!(io))
        elseif b == UInt8('\\')
            e = codeunit(c.s, c.i + 1)
            if e == UInt8('u')
                cp = parse(UInt16, c.s[c.i+2:c.i+5]; base=16)
                print(io, Char(cp))
                c.i += 6
            else
                m = Dict(UInt8('"') => '"', UInt8('\\') => '\\', UInt8('/') => '/', UInt8('b') => '\b',
                         UInt8('f') => '\f', UInt8('n') => '\n', UInt8('r') => '\r', UInt8('t') => '\t')
                print(io, m[e])
                c.i += 2
            end
        else
            write(io, b)
            c.i += 1
        end
    end
end

function parse_number(c::Cursor)
    j = c.i
    isfloat = false
    while c.i <= ncodeunits(c.s)
        b = codeunit(c.s, c.i)
        if b == UInt8('.') || b == UInt8('e') || b == UInt8('E')
            isfloat = true
        elseif !(UInt8('0') <= b <= UInt8('9') || b == UInt8('-') || b == UInt8('+'))
            break
        end
        c.i += 1
    end
    t = c.s[j:c.i-1]
    isempty(t) && error("JSON: value expected at byte $j")
    return isfloat ? parse(Float64, t) : (length(t) > 18 ? parse(Float64, t) : parse(Int, t))
end

jparse(s::AbstractString) = parse_value(Cursor(String(s), 1))

function jwrite(io::IO, x)
    if x === nothing
        print(io, "null")
    elseif x isa Bool
        print(io, x ? "true" : "false")
    elseif x isa Integer
        print(io, x)
    elseif x isa AbstractFloat
        v = Float64(x)
        isfinite(v) ? print(io, repr(v)) : print(io, "null")
    elseif x isa AbstractString || x isa Symbol
        print(io, '"')
        for ch in string(x)
            if ch == '"'
                print(io, "\\\"")
            elseif ch == '\\'
                print(io, "\\\\")
            elseif ch == '\n'
                print(io, "\\n")
            elseif ch < ' '
                print(io, "\\u", string(UInt16(ch); base=16, pad=4))
            else
                print(io, ch)
            end
        end
        print(io, '"')
    elseif x isa AbstractDict
        print(io, '{')
        first = true
        for (k, v) in x
            first || print(io, ',')
            first = false
            jwrite(io, string(k))
            print(io, ':')
            jwrite(io, v)
        end
        print(io, '}')
    elseif x isa AbstractVector || x isa Tuple
        print(io, '[')
        for (i, v) in enumerate(x)
            i > 1 && print(io, ',')
            jwrite(io, v)
        end
        print(io, ']')
    else
        jwrite(io, string(x))
    end
end

function jwrite_string(x)
    io = IOBuffer()
    jwrite(io, x)
    return String(take!(io))
end

end # module
