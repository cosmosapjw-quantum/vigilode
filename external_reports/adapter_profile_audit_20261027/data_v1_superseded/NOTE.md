# Superseded v1 diagnostic data

Produced 17:47-17:53 KST with binaries built from 381a9cc2 plus an uncommitted
harness diff (bench_bin sha 8fdaa109…, prof_bin sha recorded in PROF_BIN_SHA256.txt).
That harness timed a per-segment JSON summary inside the loop for switching arms
only and reported allocations from a cold first call (workflow finding R02/SC-01).
The workflow analyses in ../workflow/ were made on this data; their corrected
numbers were re-checked against ../data_v2/ (harness v2, committed source
46939bbe). Do not cite v1 ratios. Kept unchanged as the audit trail.
