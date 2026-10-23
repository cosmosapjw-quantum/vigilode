#!/usr/bin/env python3
"""Algebraic method illustrations; no benchmark or runtime measurements."""
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import FancyBboxPatch, FancyArrowPatch
P=Path(__file__).resolve().parent
plt.rcParams.update({'font.family':'DejaVu Sans','font.size':10,'axes.spines.top':False,'axes.spines.right':False,'pdf.fonttype':42,'svg.fonttype':'none'})
def R(z): return 1+z+z*z/2+z**3/6+z**4/24+z**5/120+z**6/600
fig,axs=plt.subplots(1,2,figsize=(10.5,3.65),layout='constrained')
x=np.linspace(-4.3,1.2,550);y=np.linspace(-4,4,640);X,Y=np.meshgrid(x,y);Z=X+1j*Y
axs[0].contourf(X,Y,np.abs(R(Z)),levels=[0,1],colors=['#d9e9f4'])
axs[0].contour(X,Y,np.abs(R(Z)),levels=[1],colors=['#1f567d'],linewidths=1.4)
axs[0].axhline(0,c='#999',lw=.6);axs[0].axvline(0,c='#999',lw=.6)
axs[0].set(xlabel=r'$\mathrm{Re}(h\lambda)$',ylabel=r'$\mathrm{Im}(h\lambda)$',title='A. DOPRI5: |R(z)| <= 1');axs[0].set_aspect('equal')
t=np.linspace(.06,.9,220);q=R(1j*t)
axs[1].loglog(t,np.abs(np.abs(q)-1),label='Amplitude defect',color='#1f567d')
axs[1].loglog(t,np.abs(np.angle(q)-t),label='Phase defect [rad]',color='#ab5c2f')
axs[1].loglog(t,t**6*(1/3600+.9*np.exp(.9)/5040),'--',label='Complex-error upper bound',color='#59676e')
axs[1].set(xlabel=r'Phase increment $|\omega h|$',ylabel='One-step absolute error',title='B. Normalized linear oscillation');axs[1].legend(fontsize=8,loc='upper left');axs[1].grid(alpha=.2,which='both')
for ext in ['pdf','svg','png']: fig.savefig(P/'figures'/f'dopri5_stability.{ext}',dpi=180)
plt.close(fig)

# Exact-arithmetic dependency statement, not measured convergence or timing.
fig,ax=plt.subplots(figsize=(7.6,3.7),layout='constrained')
known=np.array([[int(stage<=sweep) for stage in range(8)] for sweep in range(8)])
from matplotlib.colors import ListedColormap
ax.imshow(known,cmap=ListedColormap(['#eef0f2','#285d7d']),vmin=0,vmax=1,aspect='auto')
for sweep in range(8):
 for stage in range(8):
  ax.text(stage,sweep,'exact' if known[sweep,stage] else '',ha='center',va='center',color='white',fontsize=8)
ax.set(xticks=range(8),xticklabels=range(1,9),yticks=range(8),xlabel='Stage index',ylabel='Simultaneous updates',title='Causal propagation from an exact first stage')
ax.set_xticks(np.arange(-.5,8,1),minor=True);ax.set_yticks(np.arange(-.5,8,1),minor=True)
ax.grid(which='minor',color='white',linewidth=1.5);ax.tick_params(which='minor',bottom=False,left=False)
for ext in ['pdf','svg','png']:fig.savefig(P/'figures'/f'causal_finite_propagation.{ext}',dpi=180)
plt.close(fig)

fig,ax=plt.subplots(figsize=(7.6,3.6),layout='constrained')
k=np.arange(0,17)
for eta,color in [(.1,'#285d7d'),(.5,'#ae5b32'),(.9,'#747b48')]:
 ax.semilogy(k,eta**k,marker='o',markersize=3,color=color,label=rf'$\eta={eta}$')
ax.set(xlabel='Correction count k',ylabel='Theoretical residual ratio bound',xticks=range(0,17,2),title=r'Approximate quotient: $\|r_k\|/\|r_0\|\leq\eta^k$')
ax.grid(alpha=.2,which='both');ax.legend(frameon=False)
for ext in ['pdf','svg','png']:fig.savefig(P/'figures'/f'quotient_defect_contraction.{ext}',dpi=180)
plt.close(fig)
fig,ax=plt.subplots(figsize=(10.5,4.6));ax.set(xlim=(0,10.5),ylim=(-.15,4.8));ax.axis('off')
def box(x,y,w,h,text,color='#e5eff6'):
 ax.add_patch(FancyBboxPatch((x,y),w,h,boxstyle='round,pad=0.07',facecolor=color,edgecolor='#517185',linewidth=1))
 ax.text(x+w/2,y+h/2,text,ha='center',va='center',fontsize=9)
def arrow(a,b,color='#517185',style='-'):
 ax.add_patch(FancyArrowPatch(a,b,arrowstyle='-|>',mutation_scale=11,linewidth=1,color=color,linestyle=style))
box(3.6,3.9,3.1,.65,'Accepted state (t, y)\nSelected method and step proposal h')
box(.3,2.65,4.1,.7,'DOPRI5: 7 explicit stages\nReuse accepted endpoint RHS')
box(6.1,2.65,4.1,.7,'RODAS5P: build / reuse J\nFactor W once; 8 serial stage solves')
box(3.45,1.35,3.6,.75,'Finite candidate + embedded error\nWeighted RMS acceptance test')
box(.3,.05,2.7,.7,'Reject: retain (t, y)\nReduce requested step','#f7ecdf')
box(3.9,.05,2.7,.7,'Accept: publish new (t, y)\nUpdate controller','#e4efe5')
box(7.5,.05,2.7,.7,'Fatal failure: stop\nReturn accepted partial state','#f2e5e5')
arrow((4.3,3.9),(2.9,3.35));arrow((6.,3.9),(7.5,3.35))
arrow((2.9,2.65),(4.3,2.1));arrow((7.5,2.65),(6.,2.1))
arrow((3.8,1.35),(2.1,.75));arrow((5.25,1.35),(5.25,.75));arrow((6.6,1.35),(8.4,.75))
ax.text(.35,4.32,'One selected method per integration',fontsize=9,color='#294c61')
for ext in ['pdf','svg','png']:fig.savefig(P/'figures'/f'accepted_state_flow.{ext}',dpi=180,bbox_inches='tight')
plt.close(fig)
for path in (P/'figures').glob('*.svg'):
 path.write_text('\n'.join(line.rstrip() for line in path.read_text().splitlines())+'\n')
