#!/usr/bin/env python3
"""Bind an existing native API configuration to a distinct benchmark arm ID."""
import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path


def main():
    p=argparse.ArgumentParser()
    p.add_argument('--adapter',required=True)
    p.add_argument('--solver-id',required=True)
    p.add_argument('--solver-config-json',required=True)
    p.add_argument('--request',type=Path,required=True)
    p.add_argument('--measurement-mode',choices=('timing','allocation'),default='timing')
    p.add_argument('--probe',action='store_true')
    args=p.parse_args()
    req=json.loads(args.request.read_text())
    config=json.loads(args.solver_config_json)
    if not isinstance(config,dict) or req.get('solver_config') not in (None,{},config):
        raise ValueError('conflicting frozen solver configuration')
    original_id=req['solver_id']
    req={**req,'solver_id':args.solver_id,'solver_config':config}
    with tempfile.TemporaryDirectory(prefix='vigilode-configured-arm-') as td:
        request=Path(td)/'request.json';request.write_text(json.dumps(req,allow_nan=False))
        command=[args.adapter,'--request',str(request)]
        command+=['--probe'] if args.probe else ['--measurement-mode',args.measurement_mode]
        proc=subprocess.run(command,capture_output=True,text=True,check=False)
        sys.stderr.write(proc.stderr)
        result=json.loads(proc.stdout)
        result['benchmark_arm_id']=original_id
        result['underlying_solver_id']=args.solver_id
        result['configuration_binding']=config
        if result.get('status')=='OK' and not args.probe and result.get('requested_solver_config')!=config:
            raise ValueError('native result did not confirm requested configuration')
        print(json.dumps(result,allow_nan=False,separators=(',',':')))
        raise SystemExit(proc.returncode)


if __name__=='__main__':main()
