#!/usr/bin/env python3
"""Exercise cancellation and stale-source failure behavior with owned toy children."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output",type=Path,required=True)
    args = parser.parse_args()
    node = Path(__file__).resolve().parent
    runner = node / "record_command.py"
    args.output.mkdir(parents=True,exist_ok=False)
    results = []

    def run(name,code,timeout=5,inputs=(),expected=0):
        command = [sys.executable,str(runner),"--output",str(args.output/name),"--timeout",str(timeout)]
        for item in inputs: command += ["--input",str(item)]
        command += ["--",sys.executable,"-c",code]
        result = subprocess.run(command,capture_output=True,text=True,timeout=15)
        receipt = json.loads((args.output/name/"RECEIPT.json").read_text())
        assert result.returncode == expected,(name,result.returncode,result.stderr)
        results.append({"name":name,"pass":True,"exit_code":result.returncode,"receipt":str(args.output/name/"RECEIPT.json")})
        return receipt

    with tempfile.TemporaryDirectory(prefix="vigilode-runner-contracts-") as temporary:
        directory = Path(temporary)
        receipt = run("success","print('owned command completed')")
        assert receipt["source_unchanged_during_command"]
        assert "RAYON_NUM_THREADS" in receipt["environment"] and "RUSTDOCFLAGS" in receipt["environment"]
        mutation = directory/"input.txt"; mutation.write_text("before")
        receipt = run("mutation",f"from pathlib import Path; Path({str(mutation)!r}).write_text('after')",inputs=[mutation],expected=2)
        assert receipt["actual_command_exit_code"] == 0 and receipt["status"] == "stale_source"
        receipt = run("child_exit","raise SystemExit(7)",expected=7)
        assert receipt["actual_command_exit_code"] == 7

        # A newly discovered configuration must invalidate source identity too.
        probe = node/"configs"/f"runner_contract_{os.getpid()}.json"
        try:
            receipt = run("new_discovered_input",f"from pathlib import Path; Path({str(probe)!r}).write_text('{{}}')",expected=2)
            assert str(probe) in receipt["source_changes"]
        finally:
            probe.unlink(missing_ok=True)

        for scenario in ("timeout","signal"):
            child_pid = directory/f"{scenario}.pid"
            child = "import signal,time,os; from pathlib import Path; signal.signal(signal.SIGTERM,signal.SIG_IGN); Path("+repr(str(child_pid))+").write_text(str(os.getpid())); time.sleep(60)"
            parent = "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c',"+repr(child)+"]); time.sleep(60)"
            output = args.output/scenario
            command = [sys.executable,str(runner),"--output",str(output),"--timeout","1" if scenario == "timeout" else "10","--",sys.executable,"-c",parent]
            process = subprocess.Popen(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
            deadline = time.monotonic()+5
            while not child_pid.exists() and process.poll() is None and time.monotonic()<deadline:
                time.sleep(0.02)
            assert child_pid.exists(),"owned child did not start"
            if scenario == "signal": process.send_signal(signal.SIGTERM)
            stdout,stderr = process.communicate(timeout=10)
            expected = 124 if scenario == "timeout" else 143
            assert process.returncode == expected,(scenario,process.returncode,stdout,stderr)
            pid = int(child_pid.read_text())
            stat = Path(f"/proc/{pid}/stat")
            deadline = time.monotonic()+2
            while stat.exists() and stat.read_text().split()[2] not in ("Z","X") and time.monotonic()<deadline:
                time.sleep(0.02)
            assert not stat.exists() or stat.read_text().split()[2] in ("Z","X"),f"owned descendant {pid} still executing"
            receipt = json.loads((output/"RECEIPT.json").read_text())
            assert receipt["process_group_termination"]["sigkill_sent"]
            results.append({"name":scenario,"pass":True,"exit_code":expected,"descendant_no_longer_executing":True,"receipt":str(output/"RECEIPT.json")})

        missing = args.output/"missing_executable"
        result = subprocess.run([sys.executable,str(runner),"--output",str(missing),"--timeout","5","--","/nonexistent/vigilode-owned-test-command"],capture_output=True,text=True,timeout=10)
        assert result.returncode == 127
        assert json.loads((missing/"RECEIPT.json").read_text())["launch_error"]
        results.append({"name":"missing_executable","pass":True,"exit_code":127})
    summary = {"schema":"vigilode.runner-contracts.v1","status":"pass","contracts":results,"runner_sha256":hashlib.sha256(runner.read_bytes()).hexdigest(),"scope":"owned toy processes; no solver benchmarks or model claims"}
    (args.output/"SUMMARY.json").write_text(json.dumps(summary,indent=2)+"\n")
    print(json.dumps({"status":"pass","contracts":len(results)}))


if __name__ == "__main__": main()
