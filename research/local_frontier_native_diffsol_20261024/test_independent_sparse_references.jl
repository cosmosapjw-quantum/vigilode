# Synthetic evidence controls plus one probe-only identity check; no integration.
# ARGS: SOURCE.jl NEW_EVIDENCE_DIR CASE8_DESCRIPTOR.json
using Test,JSON3,SHA
length(ARGS)==3 || error("source, new evidence directory, case8 descriptor required")
source,evidence_dir,descriptor_path=ARGS
mkdir(evidence_dir)
include(abspath(source))
snapshot(dir)=Dict(name=>bytes2hex(sha256(read(joinpath(dir,name)))) for name in readdir(dir))
observations=Dict{String,Any}()

@testset "sparse reference provenance and output ownership" begin
    file=joinpath(evidence_dir,"exclusive.json")
    emit(file,Dict("first"=>true))
    original=read(file)
    @test_throws Exception emit(file,Dict("replacement"=>true))
    @test read(file)==original
    alias=joinpath(evidence_dir,"exclusive_alias.json")
    symlink(file,alias)
    @test_throws Exception emit(alias,Dict("replacement"=>true))
    @test read(file)==original

    historical=joinpath(evidence_dir,"historical")
    mkdir(historical)
    write(joinpath(historical,"FAILED.json"),"{\"historical_failure\":true}\n")
    write(joinpath(historical,"PROBE_CONTROL.json"),"{\"historical_probe\":true}\n")
    write(joinpath(historical,"raw.bin"),UInt8[0x00,0xff,0x0a])
    before=snapshot(historical)
    @test_throws Exception main(["missing_descriptor",historical,"probe"])
    @test snapshot(historical)==before
    historical_alias=joinpath(evidence_dir,"historical_alias")
    symlink(historical,historical_alias)
    @test_throws Exception main(["missing_descriptor",historical_alias,"probe"])
    @test snapshot(historical)==before
    observations["historical_before"]=before
    observations["historical_after"]=snapshot(historical)

    owned=joinpath(evidence_dir,"owned_failure")
    @test_throws Exception main([joinpath(evidence_dir,"missing_descriptor.json"),owned,"probe"])
    first=snapshot(owned)
    failure=JSON3.read(read(joinpath(owned,"FAILED.json"),String))
    @test failure.status=="FAILED_REFERENCE_CONSTRUCTION"
    @test occursin("missing_descriptor.json",String(failure.exception))
    preserve_failure(joinpath(owned,"FAILED.json"),Dict("second_failure"=>true))
    @test snapshot(owned)==first
    @test_throws Exception main(["missing_descriptor",owned,"probe"])
    @test snapshot(owned)==first
    invalid=joinpath(evidence_dir,"invalid_mode")
    @test_throws Exception main(["missing_descriptor",invalid,"invalid"])
    @test !ispath(invalid)

    probe=joinpath(evidence_dir,"identity_probe")
    expected_sha=bytes2hex(sha256(read(descriptor_path)))
    main([descriptor_path,probe,"probe"])
    control=JSON3.read(read(joinpath(probe,"PROBE_CONTROL.json"),String))
    @test control.descriptor_sha256==expected_sha
    @test control.descriptor_sha256!=bytes2hex(sha256(UInt8[]))
    @test all(row.pass for row in control.rhs_probes)
    @test control.jacobian_action.pass
    @test control.callback_allocations.pass
    @test control.sparse_support_sha256==bytes2hex(sha256(read(joinpath(dirname(abspath(source)),"sparse_jacobian_support.jl"))))
    @test control.reference_callback_allocations.status=="NOT_APPLICABLE"
    @test Set(readdir(probe))==Set(["PROBE_CONTROL.json"])
    observations["descriptor_sha256"]=expected_sha
end
observations["status"]="SYNTHETIC_CONTROLS_PASS"
observations["integration_performed"]=false
observations["source_sha256"]=bytes2hex(sha256(read(source)))
emit(joinpath(evidence_dir,"TEST_RECEIPT.json"),observations)
println(JSON3.write(observations))
