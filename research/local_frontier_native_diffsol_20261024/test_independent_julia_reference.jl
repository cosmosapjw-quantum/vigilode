# Synthetic preservation/resource controls only; this test never integrates.
# Invocation: julia --project=PINNED_PROJECT THIS.jl SOURCE.jl NEW_EVIDENCE_DIR
using Test, JSON3, SHA
length(ARGS) == 2 || error("source and new evidence directory required")
source, evidence_dir = ARGS
mkdir(evidence_dir)
include(abspath(source))

snapshot(dir) = Dict(relpath(joinpath(path,name),dir) => bytes2hex(sha256(read(joinpath(path,name))))
                     for (path,_,names) in walkdir(dir) for name in names)
observations = Dict{String,Any}()

@testset "independent reference evidence preservation" begin
    @testset "exclusive emit retains the first bytes and refuses symlinks" begin
        file = joinpath(evidence_dir,"exclusive.json")
        emit(file, Dict("first"=>1))
        first_bytes = read(file)
        @test_throws Exception emit(file, Dict("second"=>2))
        @test read(file) == first_bytes
        alias = joinpath(evidence_dir,"alias.json")
        symlink(file,alias)
        @test_throws Exception emit(alias, Dict("second"=>3))
        @test read(file) == first_bytes
        observations["exclusive_first_sha256"] = bytes2hex(sha256(first_bytes))
    end

    @testset "an existing output directory is never owned by this invocation" begin
        dir = joinpath(evidence_dir,"historical")
        mkdir(dir)
        write(joinpath(dir,"FAILED.json"), "{\"historical_failure\":true}\n")
        write(joinpath(dir,"PROBE_CONTROL.json"), "{\"historical_probe\":true}\n")
        write(joinpath(dir,"sentinel.bin"), UInt8[0x00,0x0a,0xff])
        before = snapshot(dir)
        @test_throws Exception main(["missing_descriptor",dir,"probe"])
        @test snapshot(dir) == before
        observations["existing_directory_sha256_before"] = before
        observations["existing_directory_sha256_after"] = snapshot(dir)
        alias = joinpath(evidence_dir,"historical_alias")
        symlink(dir,alias)
        @test_throws Exception main(["missing_descriptor",alias,"probe"])
        @test snapshot(dir) == before
        file = joinpath(evidence_dir,"existing_file")
        write(file,"existing output file\n")
        @test_throws Exception main(["missing_descriptor",file,"probe"])
        @test read(file,String) == "existing output file\n"
    end

    @testset "only fresh directories receive failure receipts" begin
        dir = joinpath(evidence_dir,"owned_failure")
        missing = joinpath(evidence_dir,"no_descriptor.json")
        @test_throws Exception main([missing,dir,"probe"])
        receipt = JSON3.read(read(joinpath(dir,"FAILED.json"),String))
        @test receipt.status == "FAILED_REFERENCE_CONSTRUCTION"
        @test occursin("no_descriptor.json",String(receipt.exception))
        before = snapshot(dir)
        @test_throws Exception main([missing,dir,"probe"])
        @test snapshot(dir) == before
        preserve_failure(joinpath(dir,"FAILED.json"),Dict("replacement"=>true))
        @test snapshot(dir) == before
        observations["owned_first_failure_sha256"] = before
        never_owned = joinpath(evidence_dir,"invalid_mode")
        @test_throws Exception main([missing,never_owned,"invalid"])
        @test !ispath(never_owned)
    end

    @testset "dense case4 is refused before integration" begin
        dir = joinpath(evidence_dir,"resource_refusal")
        mkdir(dir)
        @test_throws ResourceRefusal enforce_resource_gate((id=4,n=4096),dir)
        receipt = JSON3.read(read(joinpath(dir,"RESOURCE_REFUSAL.json"),String))
        @test receipt.status == "HOLD_RESOURCE_REFUSAL"
        @test receipt.started_solve_levels == 0
        @test receipt.reference_eligible == false
        @test receipt.cross_method_check_status == "NOT_RUN_RESOURCE_REFUSAL"
        @test Set(readdir(dir)) == Set(["RESOURCE_REFUSAL.json"])
        @test enforce_resource_gate((id=9,n=512),dir) === nothing
        @test enforce_resource_gate((id=11,n=396),dir) === nothing
        @test_throws Exception enforce_resource_gate((id=12,n=1024),dir)
        observations["case4_resource_refusal"] = receipt
    end

    @testset "all three primary levels and the distinct method determine uncertainty" begin
        @test [(r[1],r[3],r[4]) for r in LEVELS] == [
            ("tsit5_coarse",1e-8,1e-10), ("tsit5_middle",1e-9,1e-11),
            ("tsit5_tight",1e-10,1e-12), ("rodas5p_tight",1e-10,1e-12)]
        states = [[0.0,0.0],[2e-8,0.0],[3e-8,0.0],[4e-8,8e-8]]
        rows = [Dict("label"=>LEVELS[i][1],"full_endpoint_state"=>states[i]) for i in eachindex(LEVELS)]
        r = endpoint_refinement(rows)
        @test length(r.pairwise) == 6
        @test r.U == 10*8e-8
        @test r.reference_state == states[3]
        @test_throws Exception endpoint_refinement(rows[1:3])
        rows[2]["full_endpoint_state"] = [1e-6,0.0]
        @test endpoint_refinement(rows).U == 10*1e-6
        observations["synthetic_pairwise_count"] = length(r.pairwise)
    end
end

observations["status"] = "SYNTHETIC_CONTROLS_PASS"
observations["source_sha256"] = bytes2hex(sha256(read(source)))
observations["integration_performed"] = false
emit(joinpath(evidence_dir,"TEST_RECEIPT.json"),observations)
println(JSON3.write(observations))
