from pathlib import Path
from fractions import Fraction as Q
from itertools import product
import json,collections,hashlib
import numpy as np
from scipy.optimize import brentq
from reference import solve,evaluate
from structural import native,ev,br,deriv,add,sub
from exact_native import exact_cells,counts
def encl(box,a,b):
 assert Q(box[0])<=a<=b<=Q(box[1]),(box,a,b)
def check_native(slabs,cert,rad,c0,rate):
 prev=Q(rad);last=None;out={}
 for s in slabs:
  a,b=map(Q,s['times']);assert a<b
  pp=native(s)[:2];counts['coefficient_checks']+=5*len(s['native']['y0'])
  c=s['certificate']
  if not c or not c['global_qualified'] or not c.get('conservation'):
   counts['unqualified_slabs']+=1;break
  assert Q(c['incoming'])==prev
  if last is not None:assert a==last[0]and [ev(x,Q(0))for x in pp]==last[1]
  else:assert [ev(x,Q(0))for x in pp]==list(map(Q,c0))
  prev=exact_cells(s,pp,rate,cert)
  last=(b,[ev(x,Q(1))for x in pp])
  out[tuple(s['times'])]=(pp,c)
 return out
def query_check(a,p,e,certs):
 span=list(map(Q,a['phase']));clock=list(map(Q,e['clock']));t=Q(a['t'])
 assert Q(e['clock'][1])<=t<=Q(e['absolute_safe'])
 encl(a['phase'],t-clock[1],t-clock[0]);assert span[0]>=0 and span[1]<=Q(e['relative_safe'])
 assert a['cells']
 covered=span[0];hull=[[float('inf'),-float('inf')]for _ in range(3)]
 for cell in a['cells']:
  key=tuple(cell['native_times']);ps,c=certs[key]
  l,h=map(Q,key);x,y=map(Q,cell['phase'])
  assert l<=x<=y<=h and x<=covered
  assert x==max(l,span[0])and y==min(h,span[1])
  covered=max(covered,y)
  u0,u1=(x-l)/(h-l),(y-l)/(h-l)
  pol=[*ps,sub([Q(1)],add(*ps))]
  err=Q(cell['error_l1']);assert err==Q(c['whole_x_error'])
  for i,pp in enumerate(pol):
   b=cell['represented'][i];d=br(deriv(pp),u0,u1-u0)
   if min(d)>=0 or max(d)<=0:
    v=ev(pp,u0),ev(pp,u1);encl(b,min(v),max(v));counts['exact_monotone_ranges']+=1
   else:
    bb=[w for j in range(16)for w in br(pp,u0+(u1-u0)*Q(j,16),(u1-u0)/16)]
    if Q(b[0])<=min(bb)<=max(bb)<=Q(b[1]):counts['exact_bernstein_ranges']+=1
    else:counts['sample_only_ranges']+=1
   for j in range(5):
    z=ev(pp,u0+(u1-u0)*Q(j,4));encl(b,z,z);counts['represented_samples']+=1
   # Directed physical error inflation followed by valid invariant clipping.
   expect=max(Q(0),Q(b[0])-err),min(Q(1),Q(b[1])+err)
   encl(cell['enclosure'][i],*expect)
   hull[i][0]=min(hull[i][0],cell['enclosure'][i][0]);hull[i][1]=max(hull[i][1],cell['enclosure'][i][1])
   counts['inflation_checks']+=1
 assert covered==span[1] and hull==a['enclosure']
 counts['phase_queries']+=1
