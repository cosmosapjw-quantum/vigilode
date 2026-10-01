#!/usr/bin/env python3
import argparse,json,math,pathlib
from fractions import Fraction as F
p=argparse.ArgumentParser();p.add_argument('--input',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);a=p.parse_args()
rows=[json.loads(s) for s in a.input.read_text().splitlines() if s.strip()];results=[]
for row in rows:
 d={'case':row['case'],'method':row.get('method'),'power':row.get('power')}
 if row.get('error'):d.update(status='explicit_error',error=row['error']);results.append(d);continue
 if row['case']=='doubling_geometry_kernel':
  a1=F(row['h1']);a2=F(row['h2']);h=a1+a2;coarse=F(2);fine=2*(a1*a1+a2*h)/(h*h);true_error=fine-1;difference=coarse-fine;q=(a1*a1+a2*a2)/(h*h)
  d.update(status='checked',exact_fine=str(fine),exact_true_error=str(true_error),exact_difference=str(difference),ratio_correction=str(q/(1-q)),native_fine=row['fine'],native_estimator_norm=row['error_norm'],candidate_abs_error=row['candidate_abs_error'],candidate_error_difference=abs(row['candidate_abs_error']-float(true_error)),true_scaled_error=float(true_error)/(row['atol']+row['rtol']*abs(row['fine'])))
  results.append(d);continue
 if 'times' not in row:d.update(status='admission_observation',admitted=row.get('admitted'));results.append(d);continue
 times=row['times'];ys=row['values'];v=row.get('velocity');t0=row['t0'];tf=row['tf']
 if v is not None:
  errors=[abs(F(y[0])-F(v)*(F(t)-F(t0))) for t,y in zip(times,ys)];d.update(max_exact_flow_error=float(max(errors)),endpoint_exact=float(F(v)*(F(tf)-F(t0))),endpoint_native=ys[-1][0],constant_flow_gate=all(e<=F(1e-12) for e in errors))
 else:
  unit=F(tf)-F(t0);errors=[abs(F(y[0])-((F(t)-F(t0))/unit)**2) for t,y in zip(times,ys)];budget=1e-10+128*math.ulp(1.0)*abs(t0)/float(unit);d.update(max_quadratic_error=float(max(errors)),conditioned_budget=budget,quadratic_gate=float(max(errors))<=budget)
  if row['case']=='doubling_geometry_adaptive':d.update(success=row['success'],native_estimator_norms=row['error_norms'],actual_endpoint_error=float(errors[-1]),actual_scaled_endpoint_error=float(errors[-1])/(row['atol']+row['rtol']*abs(ys[-1][0])))
 if row['case']=='bdf_unit_boundary':
  # Derived by differentiating the quadratic interpolant through the three
  # actual nodes: a0=(1+2r)/(1+r),a1=-(1+r),a2=r^2/(1+r).
  exact=[F(0)];mixed=[F(0)];previous_h=None;wrong_constant=0
  for index,(left,right) in enumerate(zip(times,times[1:])):
   h=F(right)-F(left)
   if index==0: exact.append(h*F(v));mixed.append(h*F(v))
   else:
    r=h/previous_h;a0=(1+2*r)/(1+r);a1=-(1+r);a2=r*r/(1+r)
    exact.append((h*F(v)-a1*exact[-1]-a2*exact[-2])/a0)
    same=abs(float(h)-float(previous_h))<=32*math.ulp(1.0)*max(abs(float(h)),abs(float(previous_h)),1.0)
    if same:
     mixed.append((4*mixed[-1]-mixed[-2]+2*h*F(v))/3);wrong_constant+=int(h!=previous_h)
    else:mixed.append((h*F(v)-a1*mixed[-1]-a2*mixed[-2])/a0)
   previous_h=h
  d.update(exact_variable_endpoint=float(exact[-1]),mixed_branch_endpoint=float(mixed[-1]),unequal_steps_classified_equal=wrong_constant,native_vs_mixed_endpoint=abs(ys[-1][0]-float(mixed[-1])),actual_steps=[float(F(b)-F(a)) for a,b in zip(times,times[1:])])
 if row['case']=='cap_boundary':
  cap=F(row['max_step']);hs=row['accepted_h'];d.update(success=row['success'],accepted_h=hs,strict_cap_gate=all(F(h)<=cap for h in hs),maximum_cap_ratio=max(hs,default=0.0)/float(cap))
 d['status']='checked';results.append(d)
output={'source':'1c54194123ee6abc6daa512e8574922f510b4e2c','oracle':'exact rational on binary64 inputs; independent interpolation coefficients','rows':results,'row_count':len(rows),'scientific_scope':'bounded preregistered grid; no global solver or speed certification'}
a.output.write_text(json.dumps(output,indent=2,allow_nan=False)+'\n');print(json.dumps({'rows':len(rows),'written':str(a.output)}))
