include(joinpath(@__DIR__,"adapter.jl"))
rows=[]
for name in sort(collect(keys(ENDS))), n in (name=="large-vector" ? [4096,16384] : name in ("mixed-scales","moderate-stiff","stiff-control") ? [12] : [8])
    y,p=setup(name,n);u=y .+ .02;du=similar(y);rhs!(du,u,p,.37)
    push!(rows,Dict("case"=>name,"dimension"=>n,"initial"=>y,"u"=>u,"rhs"=>du,"time"=>.37,"reference"=>exact(name,n,.37)))
end
JSON3.write(stdout,rows);println()
