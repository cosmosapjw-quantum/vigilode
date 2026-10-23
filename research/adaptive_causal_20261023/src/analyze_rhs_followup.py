#!/usr/bin/env python3
"""Exact cross-route replay for the diagnosed RHS-only persistence correction."""
import argparse,gzip,json,pathlib,statistics

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('folder');a=p.parse_args();folder=pathlib.Path(a.folder)
 rows=[json.loads(l)for l in gzip.decompress((folder/'RAW_RUNS.jsonl.gz').read_bytes()).decode().splitlines()]
 parsed={}
 for row in rows:
  if row['status']!='COMPLETED' or row['exit_code']!=0:continue
  d=json.loads(row['stdout']);key=(row['case'],row['dimension'],row['rtol'],row['phase'],row['repeat']);parsed.setdefault(key,{})[row['mode']]=d
 pairs=[]
 for key,modes in sorted(parsed.items()):
  if not {'switch-rhs','dopri-whole'}<=modes.keys():continue
  a,b=modes['switch-rhs'],modes['dopri-whole'];pairs.append(dict(case=key[0],dimension=key[1],rtol=key[2],phase=key[3],repeat=key[4],same_times=a['times']==b['times'],same_states=a['states']==b['states'],same_work=a['work']==b['work'],same_success=a['success']==b['success'],success=a['success'],derivative_work_zero=all(a['work'][k]==0 for k in ['jacobian_builds','jvp_vectors','ft_calls','linear_solves'])))
 assert len(pairs)==108,len(pairs)
 assert all(all(r[k]for k in ['same_times','same_states','same_work','same_success','derivative_work_zero'])for r in pairs)
 summary=json.loads((folder/'SUMMARY.json').read_text());cfg=summary['configurations'];cost=[]
 for case in sorted({c['case']for c in cfg}):
  for tol in sorted({c['rtol']for c in cfg}):
   picked={c['mode']:c for c in cfg if c['case']==case and c['rtol']==tol}
   a,b,c=picked['switch-rhs'],picked['dopri-whole'],picked['dopri5'];ok=a['eligible']and b['eligible']and c['eligible']
   cost.append(dict(case=case,rtol=tol,eligible=ok,switch_rhs_seconds=a['median_seconds'],whole_dopri_seconds=b['median_seconds'],window_dopri_seconds=c['median_seconds'],window_over_switch_rhs=c['median_seconds']/a['median_seconds']if ok else None,whole_over_switch_rhs=b['median_seconds']/a['median_seconds']if ok else None,max_error=a['max_error']))
 valid_cost=[r for r in cost if r['eligible']]
 distribution={k:{'min':min(r[k]for r in valid_cost),'median':statistics.median(r[k]for r in valid_cost),'max':max(r[k]for r in valid_cost)}for k in ['whole_over_switch_rhs','window_over_switch_rhs']}
 result=dict(schema='vigilode.rhs-persistence-parity.v1',source_commit=summary['source_commit'],pairs=len(pairs),all_exact_parities_pass=True,includes_failed_partial_states=True,successful_pairs=sum(r['success']for r in pairs),checks=pairs,cost=cost,observed_ratio_distribution=distribution)
 (folder/'RHS_PARITY.json').write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps({k:v for k,v in result.items()if k not in ['checks','cost']}))
 out=folder.parents[1]/'analysis/RHS_PERSISTENCE_KO.md'
 lines=['# 측정에 따른 RHS-only workspace 유지 교정','',f"실행 소스 `{summary['source_commit']}`. 최초 864회 campaign은 그대로 보존했다. 그 결과에서 드러난 cheap nonstiff 재시작 비용에만 의존하는 새 실험이다. `DerivativeFallback::Forbidden`일 때 17개 관측 시각을 한 DOPRI 호출에 전달해 workspace와 FSAL을 유지했다. FD/JVP 후보의 과학 결과는 재실행하지 않았다.",'',f"9개 문제·3개 tolerance·3개 방법·4회 반복으로 **{summary['records']}회** 실행했다. {summary['valid_replays']}개 독립 재채점이 일치했고, {summary['solver_successes']}회 완주했다. 나머지는 Prothero RHS-only 경로의 attempt budget 종료이며 그대로 보존한다.",'',f"**{len(pairs)}개 대응 쌍에서 시간·상태·작업량·성공/부분 실패 상태가 whole-DOPRI와 정확히 일치했다.** Jacobian/JVP/time-derivative/linear-solve 작업은 모두0이다.",'','아래는 동일 rtol1e-6의 세 timed 반복 median이다. Persistent adapter와 whole DOPRI는 실제 상태도 같지만, window DOPRI는 더 작은 스텝을 쓰므로 실제 오차가 다르다. 마지막 열은 같은 명목 tolerance의 비용 비율이다. 동일 달성 오차 상한 비교는 SUMMARY.json의 comparisons를 사용한다. 실패한 완주 비교는 제외하며 전체 setup·출력 비용을 포함한다.','', '| 문제 | persistent adapter ms | whole DOPRI ms | window / adapter (같은 rtol) |', '|---|---:|---:|---:|']
 for r in cost:
  if r['rtol']==1e-6 and r['eligible']:lines.append(f"| {r['case']} | {r['switch_rhs_seconds']*1e3:.4f} | {r['whole_dopri_seconds']*1e3:.4f} | {r['window_over_switch_rhs']:.3f} |")
 lines+=['',f"동일한 상태를 반환한 whole-DOPRI/adapter 시간비의24개 성공 구성 min/median/max는 **{distribution['whole_over_switch_rhs']['min']:.3f}/{distribution['whole_over_switch_rhs']['median']:.3f}/{distribution['whole_over_switch_rhs']['max']:.3f}**다. 세 반복의 공유 호스트 시간 산포가 커서 이 수치를 일정한 가속률로 해석하지 않는다.",'','작업량 비교(rtol1e-6):','', '| 문제 | window RHS | persistent RHS | window accepted | persistent accepted |','|---|---:|---:|---:|---:|']
 for case in ['decay','logistic','nonnormal','rapid','transition','vector']:
  cc={c['mode']:c for c in cfg if c['case']==case and c['rtol']==1e-6};a,b=cc['dopri5']['work'],cc['switch-rhs']['work'];lines.append(f"| {case} | {a['rhs_calls']} | {b['rhs_calls']} | {a['accepted_steps']} | {b['accepted_steps']} |")
 lines+=['','이번 교정은 RHS-only default의 불필요한 재시작을 제거한다. 강성이 큰 경우의 stability 제한과 derivative fallback의 선택 성능은 기존 결과에 따라 별도로 다룬다. 실제 사건·mass matrix·후속 nonlinear root routing의 지원 범위는 확장하지 않는다.']
 lines+=['','![Persistent RHS의 실제 오차와 비용](../results/rhs_persistent_campaign/WORK_PRECISION.png)','','점은 세 반복의 median, 세로 막대는 min–max다. Prothero 패널은 모든 구성의 attempt budget 종료를 표시한다. 도표는 이미 계산한 SUMMARY를 읽어 그렸으며 solver/reference 계산을 반복하지 않았다.']
 out.write_text('\n'.join(lines)+'\n')
if __name__=='__main__':main()
