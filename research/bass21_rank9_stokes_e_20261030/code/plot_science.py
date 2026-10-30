"""Physical figures from frozen same-event BASS21 rank9 and screen-fixed E_l0."""
import pathlib, numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
ROOT=pathlib.Path(__file__).resolve().parents[1]
with np.load(ROOT/'results/BASS21_RANK9_EHARM_50x193.npz') as z:
    cases=z['cases'];x=z['x'];L=z['l'];E=z['E_l0_plus'];D=z['M_anisotropy'];frac=z['anisotropy_frac'];
    E20=E[:,:,0];E12=E[:,:,5];SJ=z['j2_collision'];SB=z['BQ_collision']
mask00=np.array([c.startswith('s00') for c in cases]);mask04=~mask00
fig,ax=plt.subplots(figsize=(7.5,4.8))
for mask,label in [(mask00,'s00 (25 source histories)'),(mask04,'s04 (25 source histories)')]:
    av=E20[mask].mean(axis=0);mn=E20[mask].min(axis=0);mx=E20[mask].max(axis=0)
    color=ax.plot(x,av,label=label)[0].get_color();ax.fill_between(x,mn,mx,color=color,alpha=.15)
ax.axhline(0,color='gray',linewidth=.8);ax.set(xlabel='x = ln(a/a_ref)',ylabel='E_{20}^{(+)} [source-normalized brightness]',title='Fixed-screen E-mode quadrupole, original BASS21')
ax.legend(frameon=False);fig.tight_layout();fig.savefig(ROOT/'figures/E20_HISTORY.png',dpi=170);fig.savefig(ROOT/'figures/E20_HISTORY.svg');plt.close(fig)
fig,ax=plt.subplots(figsize=(7.5,4.8))
for mask,label in [(mask00,'s00 max |E_l0|'),(mask04,'s04 max |E_l0|')]:
    Y=np.max(np.abs(E[mask]),axis=(0,1));ax.semilogy(L,Y,'o-',label=label)
ax.set(xlabel='even multipole ell',ylabel='max |E_{ell,0}^{(+)}|',title='Source-conditional high-spin harmonic spectrum')
ax.grid(alpha=.2);ax.legend(frameon=False);fig.tight_layout();fig.savefig(ROOT/'figures/E_MODES.png',dpi=170);fig.savefig(ROOT/'figures/E_MODES.svg');plt.close(fig)
fig,ax=plt.subplots(figsize=(7.5,4.8))
for mask,label in [(mask00,'s00 mean anisotropy'),(mask04,'s04 mean anisotropy')]:
    ax.plot(x,frac[mask].mean(axis=0),label=label)
ax.axhline(0,color='gray',linewidth=.8)
ax.set(xlabel='x = ln(a/a_ref)',ylabel='(M_parallel-M_perp)/(4pi J0)',title='Rank-nine coherency tensor anisotropy, 50 sources')
ax.legend(frameon=False);fig.tight_layout();fig.savefig(ROOT/'figures/RANK9_ANISOTROPY.png',dpi=170);fig.savefig(ROOT/'figures/RANK9_ANISOTROPY.svg');plt.close(fig)
fig,ax=plt.subplots(figsize=(7.5,4.8))
ax.scatter(SJ[mask00,-1],SB[mask00,-1],label='s00')
ax.scatter(SJ[mask04,-1],SB[mask04,-1],label='s04')
ax.set(xlabel='Thomson C_{J2} at final epoch',ylabel='Thomson C_{BQ} at final epoch',title='Rank-nine collision channel coupling')
ax.legend(frameon=False);fig.tight_layout();fig.savefig(ROOT/'figures/RANK9_COLLISION_CHANNELS.png',dpi=170);fig.savefig(ROOT/'figures/RANK9_COLLISION_CHANNELS.svg');plt.close(fig)
print('FIGURES 4 PNG + 4 SVG from actual source NPZ')
