"""Paired, alternating-order optimized R5 timings, retaining all blocks."""
import argparse
import gzip
import json
import statistics
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("trajectories", type=Path)
    parser.add_argument("output", type=Path)
    args=parser.parse_args()
    args.executable=args.executable.resolve()
    def key(row):
        return row["source"]["name"], row["rtol"], row["arm"]
    with gzip.open(args.trajectories,"rt") as stream:
        seen={key(row):row["trajectory"] for row in map(json.loads, stream)}
    samples={k:[] for k in seen}
    # Warm both direction orders once before collecting eight measured blocks.
    for block in range(-2,8):
        flags=["--reverse"] if block%2 else []
        payload=subprocess.check_output([str(args.executable),*flags])
        rows=[json.loads(line) for line in payload.splitlines()]
        assert len(rows)==len(seen) and {key(r) for r in rows}==set(seen)
        for row in rows:
            k=key(row)
            assert row["trajectory"]==seen[k],k
            if block>=0:samples[k].append(row["elapsed_seconds"])
    records=[]
    for (source,rtol,arm),times in samples.items():
        ratios=[a/b for a,b in zip(times,samples[source,rtol,"Protected"],strict=True)]
        records.append(dict(source=source,rtol=rtol,arm=arm,seconds=times,
                            paired_candidate_over_protected=ratios,
                            median_paired_ratio=statistics.median(ratios)))
    summary=dict(schema="vigilode.r5.cooperative-cubic-timing.v1",blocks=8,warmup_blocks=2,
                 scope="Single CPU host; shared-system load, no affinity or frequency lock. Warm alternating order; each observed endpoint and every accepted/rejected stage charged. Times start before model/client construction, stop before JSON serialization. Outputs cross-checked against stored trajectories. Interpret comparisons using corresponding achieved-error classifications.",
                 arm_order_even=["Protected","Direct","Continued","Capped"],arm_order_odd=["Capped","Continued","Direct","Protected"],records=records)
    args.output.write_text(json.dumps(summary,indent=2,allow_nan=False)+"\n")


if __name__=="__main__":main()
