#!/usr/bin/env python3
"""Bounded hostile source/receipt/output checks for the real F05 Peer experiment.
Mutations are confined to a temporary directory, never the source or run records.
"""
import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def rejected(command, needle):
    process = subprocess.run(command, capture_output=True, text=True, timeout=45)
    return process.returncode != 0 and needle in (process.stderr + process.stdout)


def main(args):
    checks = {}
    auditor = Path(__file__).with_name('audit_full_ivp.py')
    with tempfile.TemporaryDirectory(prefix='peer_negative_', dir=args.out.parent) as temporary:
        temp = Path(temporary)
        source = bytearray((args.source/'SOURCE_POINT.raw').read_bytes())
        source[100] ^= 1
        (temp/'source.raw').write_bytes(source)
        checks['native_source_bitflip_rejected'] = rejected([
            str(args.binary), str(args.source/'SOURCE.json'), str(temp/'source.raw'),
            str(args.source/'OWNER_RHS_ORACLE.raw'), str(temp/'native_bad')], 'SOURCE_SHA_DRIFT')
        forcing = bytearray((args.source/'SOURCE.json').read_bytes())
        forcing[80] ^= 1
        (temp/'forcing.json').write_bytes(forcing)
        checks['native_forcing_bitflip_rejected'] = rejected([
            str(args.binary), str(temp/'forcing.json'), str(args.source/'SOURCE_POINT.raw'),
            str(args.source/'OWNER_RHS_ORACLE.raw'), str(temp/'forcing_bad')], 'SOURCE_SHA_DRIFT')
        original = json.loads((args.runs/'RESULT.json').read_text())
        broken = temp/'broken_runs'
        broken.mkdir()
        for path in args.runs.glob('*.raw'):
            (broken/path.name).symlink_to(path.resolve())
        victim = broken/'peer_n4_p1.raw'
        victim.unlink()
        shutil.copyfile(args.runs/victim.name, victim)
        with victim.open('r+b') as file:
            file.seek(104)
            before = file.read(1)
            file.seek(104)
            file.write(bytes([before[0] ^ 1]))
        (broken/'RESULT.json').write_text(json.dumps(original))
        audit_args = [str(auditor), '--source', str(args.source), '--runs', str(broken),
                      '--independent', str(args.independent), '--out', str(temp/'bad.json')]
        checks['audit_corrupt_output_rejected'] = rejected(
            [sys.executable] + audit_args, 'OUTPUT_SHA_MISMATCH')
        checks['optimized_audit_corrupt_output_rejected'] = rejected(
            [sys.executable, '-O'] + audit_args, 'OUTPUT_SHA_MISMATCH')
        victim.unlink()
        victim.symlink_to((args.runs/victim.name).resolve())
        mutations = [
            ('missing_case_rejected', lambda r: r['runs'].pop(), 'CASE_CENSUS_MISMATCH'),
            ('nonfinite_residual_rejected', lambda r: r['runs'][0].update(max_original_W_relative_residual=float('nan')), 'W_RESIDUAL_FAILED'),
            ('false_zero_solves_rejected', lambda r: r['runs'][0].update(W_solves=0), 'EXECUTED_WORK_COUNT_MISMATCH'),
            ('forged_official_acceptance_rejected', lambda r: r.update(official_tick161='ACCEPTED'), 'FORGED_OFFICIAL_ACCEPTANCE'),
        ]
        for name, mutate, expected in mutations:
            receipt = json.loads(json.dumps(original))
            mutate(receipt)
            (broken/'RESULT.json').write_text(json.dumps(receipt))
            checks[name] = rejected([sys.executable, '-O'] + audit_args, expected)
    result = {'schema': 'REI_FULL_POINT_PEER_HOSTILE_CONTROLS_V1', 'checks': checks,
              'passed': sum(checks.values()), 'total': len(checks),
              'all_expected_failures_rejected': all(checks.values()),
              'official_tick161': 'NOT_ACCEPTED', 'production': 'HOLD', 'canonical_delta': 0}
    args.out.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps(result, sort_keys=True))
    if not result['all_expected_failures_rejected']:
        raise RuntimeError('HOSTILE_CHECK_DID_NOT_REJECT_INVALID_EVIDENCE')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ('source', 'runs', 'independent', 'binary', 'out'):
        parser.add_argument('--' + name, type=Path, required=True)
    main(parser.parse_args())
