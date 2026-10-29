#!/usr/bin/env python3
"""Mechanical component/evidence reports. Scientific actions require input review."""
from __future__ import annotations

import argparse
import csv
import html
import json
import math
from pathlib import Path
import sys

if __package__ in (None,''):
    sys.path.insert(0,str(Path(__file__).resolve().parents[2]))

from tools.component_benchmark.confirmation import _receipt_valid as confirmation_receipt_valid
from tools.component_benchmark.protocol import digest, file_digest, strict_load
from tools.component_benchmark.registry import inventory
from tools.component_benchmark.runner import atomic_json, validate_receipt


def _runner_rows(directory, plan):
    rows = []
    for job in plan['jobs']:
        folder = directory/'jobs'/job['job_id']
        path = folder/'receipt.json'
        if not path.exists():
            rows.append({'job':job,'receipt':None,'receipt_path':None,
                         'evidence_status':'MISSING_RECEIPT'})
            continue
        receipt = strict_load(path.read_text())
        if not validate_receipt(folder,receipt,job,digest(plan['source_binding'])):
            raise RuntimeError('RUNNER_RECEIPT_IDENTITY_MISMATCH: '+str(path))
        rows.append({'job':job,'receipt':receipt,'receipt_path':str(path),
                     'evidence_status':receipt['status']})
    return rows


def _confirmation_rows(directory, plan):
    rows = []
    for round_index in range(17):
        for cohort_index, cohort in enumerate(plan['cohorts']):
            for phase in (0,1):
                for index in cohort:
                    item = plan['contrasts'][index]
                    arm_name = plan['orders'][f'{round_index}:{index}'][phase]
                    arm = item[arm_name]
                    ident = {'plan_sha256':digest(plan),'round':round_index,
                             'cohort':cohort_index,'phase':phase,
                             'contrast_index':index,'arm':arm_name,
                             'case_sha256':digest(item['case']),
                             'solver_spec_sha256':digest(arm['solver_spec'])}
                    name = f'r{round_index:02d}-c{cohort_index:02d}-p{phase}-{index:02d}'
                    folder = directory/'jobs'/name
                    path = folder/'receipt.json'
                    job = {'job_id':digest(ident),'case_id':item['case']['case_id'],
                           'solver_id':arm['solver_id'],'config_id':arm['config_id'],
                           'round':round_index,'measurement_mode':'timing',
                           'kind':'confirmation','request':{'case':item['case'],
                                                             'solver_id':arm['solver_id']}}
                    if not path.exists():
                        rows.append({'job':job,'receipt':None,'receipt_path':None,
                                     'evidence_status':'MISSING_RECEIPT'})
                        continue
                    receipt = strict_load(path.read_text())
                    if not confirmation_receipt_valid(path,ident):
                        raise RuntimeError('CONFIRMATION_RECEIPT_IDENTITY_MISMATCH: '+str(path))
                    rows.append({'job':job,'receipt':receipt,'receipt_path':str(path),
                                 'evidence_status':receipt['status']})
    return rows


def collect_runs(paths):
    rows, gates, plans = [], {}, []
    seen = {}
    for raw in paths:
        directory = Path(raw).resolve()
        runner_plan = directory/'plan.json'
        confirmation_plan = directory/'confirmation_plan.json'
        if runner_plan.exists() and confirmation_plan.exists():
            raise RuntimeError('AMBIGUOUS_RUN_PLAN: '+str(directory))
        if runner_plan.exists():
            plan = strict_load(runner_plan.read_text())
            current = _runner_rows(directory,plan)
            source_hash = digest(plan['source_binding'])
            kind = plan['kind']
            solvers = plan['solvers']
        elif confirmation_plan.exists():
            plan = strict_load(confirmation_plan.read_text())
            current = _confirmation_rows(directory,plan)
            source_hash = plan['source_binding_sha256']
            kind = 'confirmation'
            solvers = {arm['solver_id']:arm['solver_spec']
                       for item in plan['contrasts']
                       for arm in (item['baseline'],item['candidate'])}
        else:
            raise FileNotFoundError('no recognized frozen plan: '+str(directory))
        plans.append({'directory':str(directory),'kind':kind,'source_binding_sha256':source_hash,
                      'plan_sha256':file_digest(runner_plan if runner_plan.exists() else confirmation_plan),
                      'solver_components':{sid:spec.get('components',[]) for sid,spec in solvers.items()}})
        analysis_path = directory/'analysis.json'
        if analysis_path.exists():
            analysis = strict_load(analysis_path.read_text())
            if analysis.get('source_binding_valid') is not True:
                raise RuntimeError('ANALYSIS_SOURCE_BINDING_NOT_VALID: '+str(analysis_path))
            gates.update(analysis.get('scientific_gates',{}))
        for row in current:
            key = (source_hash,row['job']['job_id'])
            identity = digest(row['receipt']) if row['receipt'] is not None else None
            if key in seen:
                if seen[key] != identity:
                    raise RuntimeError('DUPLICATE_JOB_IDENTITY_CONFLICT: '+str(key))
                continue
            seen[key] = identity
            row['source_binding_sha256'] = source_hash
            row['run_directory'] = str(directory)
            rows.append(row)
    return rows,gates,plans


def _metric(row, gate):
    receipt = row['receipt']
    result = receipt.get('result') if receipt else None
    if not isinstance(result,dict):
        result = {}
    timing = result.get('timing') or {}
    counters = result.get('counters') or {}
    value = timing.get('timed_seconds')
    if type(value) not in (int,float) or not math.isfinite(value) or value<=0:
        value = None
    error = gate.get('max_error_budget_ratio') if isinstance(gate,dict) else None
    if type(error) not in (int,float) or not math.isfinite(error) or error<0:
        error = None
    return {'job_id':row['job']['job_id'],'case_id':row['job']['case_id'],
            'solver_id':row['job']['solver_id'],'config_id':row['job'].get('config_id'),
            'round':row['job'].get('round'),'measurement_mode':row['job'].get('measurement_mode'),
            'status':row['evidence_status'],'gate_status':gate.get('status') if isinstance(gate,dict) else None,
            'error_budget_ratio':error,'timed_seconds':value,
            'rhs_evaluations':counters.get('rhs_evaluations'),
            'jacobian_evaluations':counters.get('jacobian_evaluations'),
            'lu_factorizations':counters.get('lu_factorizations'),
            'peak_rss_bytes':receipt.get('peak_rss_bytes') if receipt else None,
            'receipt_path':row['receipt_path'],
            'source_binding_sha256':row['source_binding_sha256']}


def _write_csv(path, rows, fields):
    with Path(path).open('w',newline='') as stream:
        writer = csv.DictWriter(stream,fieldnames=fields,extrasaction='ignore')
        writer.writeheader()
        for row in rows:
            writer.writerow({field:row.get(field) for field in fields})


def _svg(points, output, xkey, xlabel):
    usable = [p for p in points if type(p.get(xkey)) in (int,float) and
              math.isfinite(p[xkey]) and p[xkey]>0 and
              p['error_budget_ratio'] is not None and p['error_budget_ratio']>0]
    width,height = 900,540
    left,right,top,bottom = 85,30,30,75
    if usable:
        xs = [math.log10(p[xkey]) for p in usable]
        ys = [math.log10(p['error_budget_ratio']) for p in usable]
        xmin,xmax = min(xs),max(xs)
        ymin,ymax = min(ys),max(ys)
        if xmin==xmax: xmin-=.5;xmax+=.5
        if ymin==ymax: ymin-=.5;ymax+=.5
    else:
        xmin,xmax,ymin,ymax = -1,1,-1,1
    def loc(p):
        x=left+(math.log10(p[xkey])-xmin)*(width-left-right)/(xmax-xmin)
        y=height-bottom-(math.log10(p['error_budget_ratio'])-ymin)*(height-top-bottom)/(ymax-ymin)
        return x,y
    lines = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
             '<rect width="100%" height="100%" fill="white"/>',
             f'<path d="M {left} {top} V {height-bottom} H {width-right}" stroke="black" fill="none"/>',
             f'<text x="{width//2}" y="{height-20}" text-anchor="middle">{html.escape(xlabel)} (log10, diagnostic only)</text>',
             f'<text x="20" y="{height//2}" transform="rotate(-90 20 {height//2})" text-anchor="middle">error / original budget (log10)</text>']
    for point in usable:
        x,y=loc(point)
        title=html.escape(f"{point['case_id']} / {point['solver_id']} / {point['config_id']}: {point['status']}")
        lines.append(f'<circle cx="{x:.2f}" cy="{y:.2f}" r="3" fill="#225ea8"><title>{title}</title></circle>')
    if not usable:
        lines.append('<text x="450" y="270" text-anchor="middle">No paired accuracy/cost points measured</text>')
    lines.append('</svg>')
    Path(output).write_text('\n'.join(lines)+'\n')


def generate(root, runs, output, decisions_path=None):
    root,output = Path(root).resolve(),Path(output).resolve()
    rows,gates,plans = collect_runs(runs)
    components = inventory(root)
    mapped = {x['component_id']:x for x in components}
    solver_components = {}
    for plan in plans:
        for sid, ids in plan['solver_components'].items():
            prior = solver_components.setdefault(sid,set())
            prior.update(ids)
            for component_id in ids:
                if component_id not in mapped:
                    raise RuntimeError('UNKNOWN_COMPONENT_MAPPING: '+component_id)
    metrics = [_metric(row,gates.get(row['job']['job_id'])) for row in rows]
    decision_data = None
    if decisions_path:
        decision_data = strict_load(Path(decisions_path).read_text())
        if decision_data.get('independent_review') is not True or not isinstance(decision_data.get('decisions'),dict):
            raise ValueError('independent decisions require explicit review flag and map')
        if decision_data.get('source_binding_sha256') not in {p['source_binding_sha256'] for p in plans}:
            raise ValueError('decision source binding absent from runs')
        if set(decision_data['decisions'])-set(mapped):
            raise ValueError('decision references unknown component')
    report_rows = []
    for component in components:
        component_id = component['component_id']
        solver_ids = sorted(sid for sid,ids in solver_components.items() if component_id in ids)
        linked = [m for m in metrics if m['solver_id'] in solver_ids]
        failed = [m for m in linked if m['status'] not in ('OK','NOT_APPLICABLE','MISSING_RECEIPT')]
        positive = [m for m in linked if m['gate_status']=='PASS']
        missing = [m for m in linked if m['status']=='MISSING_RECEIPT']
        explicit = decision_data['decisions'].get(component_id) if decision_data else None
        if explicit:
            action = explicit.get('action')
            if action not in ('promote','prune','specialize','need to optimize'):
                raise ValueError('invalid independent decision action')
            evidence_status = explicit.get('evidence_status')
            if not isinstance(evidence_status,str) or not explicit.get('reason'):
                raise ValueError('independent decision needs status and reason')
            reason = explicit['reason']
        else:
            action,evidence_status,reason = 'need to optimize','HOLD','UNREVIEWED_OR_EVIDENCE_GAP'
        report_rows.append({'component_id':component_id,'crate':component['crate'],
                            'source_path':component['source_path'],'kind':component['kind'],
                            'symbols':component['symbols'],'solver_ids':solver_ids,
                            'observed_jobs':len(linked)-len(missing),
                            'missing_jobs':len(missing),'passing_gates':len(positive),
                            'failed_or_unsupported_jobs':len(failed),
                            'failure_receipts':[m['receipt_path'] for m in failed if m['receipt_path']],
                            'recommended_action':action,'evidence_status':evidence_status,
                            'reason':reason,'independent_decision':bool(explicit)})
    output.mkdir(parents=True,exist_ok=True)
    atomic_json(output/'component_evidence.json',{'source_plans':plans,
                'decisions_input_sha256':file_digest(decisions_path) if decisions_path else None,
                'timing_authority':'diagnostic_only','production_admission':'HOLD',
                'components':report_rows,'jobs':metrics})
    component_fields = ('component_id','crate','source_path','kind','symbols','solver_ids',
                        'observed_jobs','missing_jobs','passing_gates',
                        'failed_or_unsupported_jobs','recommended_action','evidence_status','reason')
    _write_csv(output/'all_components.csv',[
        {**r,'symbols':';'.join(r['symbols']),'solver_ids':';'.join(r['solver_ids'])}
        for r in report_rows],component_fields)
    _write_csv(output/'work_precision.csv',metrics,tuple(metrics[0]) if metrics else
               ('job_id','case_id','solver_id','status','gate_status','error_budget_ratio','timed_seconds'))
    _svg(metrics,output/'work_precision.svg','rhs_evaluations','RHS evaluations')
    _svg(metrics,output/'time_precision.svg','timed_seconds','timed seconds')
    korean = ['# 전체 구성요소 증거 보고서','',
              '측정 시간은 진단 전용입니다. 독립 검토 입력이 없는 행은 최적화 필요 / HOLD입니다.','',
              '| 구성요소 | 솔버 | 관측 | 누락 | 통과 게이트 | 실패·미지원 | 조치 | 증거 |',
              '|---|---|---:|---:|---:|---:|---|---|']
    for row in report_rows:
        action_ko = {'need to optimize':'최적화 필요','promote':'승격',
                     'prune':'제거 검토','specialize':'특화'}[row['recommended_action']]
        korean.append('| '+ ' | '.join((row['component_id'],', '.join(row['solver_ids']),
            str(row['observed_jobs']),str(row['missing_jobs']),str(row['passing_gates']),
            str(row['failed_or_unsupported_jobs']),action_ko,
            row['evidence_status']))+' |')
    (output/'all_components_ko.md').write_text('\n'.join(korean)+'\n')
    return {'components':len(report_rows),'jobs':len(metrics),
            'missing_jobs':sum(m['status']=='MISSING_RECEIPT' for m in metrics),
            'independent_decisions':sum(r['independent_decision'] for r in report_rows)}


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[2])
    parser.add_argument('--runs',nargs='+',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--decisions',type=Path)
    args=parser.parse_args()
    print(json.dumps(generate(args.root,args.runs,args.output,args.decisions)))


if __name__=='__main__':
    main()
