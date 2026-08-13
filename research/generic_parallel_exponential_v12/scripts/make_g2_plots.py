#!/usr/bin/env python3
import argparse, json
from pathlib import Path
import matplotlib.pyplot as plt


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('canonical',type=Path)
    ap.add_argument('plot_dir',type=Path)
    ns=ap.parse_args(); ns.plot_dir.mkdir(parents=True,exist_ok=True)
    d=json.loads(ns.canonical.read_text())

    fig=plt.figure(figsize=(7.2,4.8)); ax=fig.add_subplot(111)
    for method in ['exprb2','exprb43','pexprb54s4']:
        rows=sorted([r for r in d['order_screen'] if r['method']==method], key=lambda r:r['h'], reverse=True)
        ax.loglog([r['h'] for r in rows],[r['endpoint_error'] for r in rows],marker='o',label=method)
    ax.set_xlabel('step size h')
    ax.set_ylabel('endpoint absolute error')
    ax.set_title('Nonlinear order screen: y\' = y², T = 0.25')
    ax.grid(True,which='both',alpha=.3); ax.legend(); fig.tight_layout()
    fig.savefig(ns.plot_dir/'G2_ORDER_CONVERGENCE.png',dpi=180); plt.close(fig)

    fig=plt.figure(figsize=(7.2,4.8)); ax=fig.add_subplot(111)
    for case in sorted(set(r['case_id'] for r in d['phi_oracle'])):
        rows=[r for r in d['phi_oracle'] if r['case_id']==case and abs(r['scale']-0.7)<1e-15]
        ax.semilogy([r['phi_index'] for r in rows],[max(r['relative_error'],1e-18) for r in rows],marker='o',label=case)
    ax.axhline(2e-11,linestyle='--',label='foundation gate')
    ax.set_xlabel('φ-function index k'); ax.set_ylabel('relative action error')
    ax.set_title('Matrix-free Krylov action vs dense Padé oracle (scale 0.7)')
    ax.grid(True,which='both',alpha=.3); ax.legend(); fig.tight_layout()
    fig.savefig(ns.plot_dir/'G2_PHI_ORACLE_ERROR.png',dpi=180); plt.close(fig)

    methods=['exprb2','exprb43','pexprb54s4']
    depth={'exprb2':1,'exprb43':3,'pexprb54s4':3}
    phi={}
    jvp={}
    for m in methods:
        r=max([r for r in d['order_screen'] if r['method']==m],key=lambda r:r['h'])
        steps=round(.25/r['h'])
        phi[m]=r['phi_actions']/steps
        jvp[m]=r['jvp_vectors']/steps
    x=range(len(methods)); width=.25
    fig=plt.figure(figsize=(7.2,4.8)); ax=fig.add_subplot(111)
    ax.bar([i-width for i in x],[depth[m] for m in methods],width=width,label='logical critical depth')
    ax.bar(list(x),[phi[m] for m in methods],width=width,label='separate φ actions/step')
    ax.bar([i+width for i in x],[jvp[m] for m in methods],width=width,label='JVP vectors/step')
    ax.set_xticks(list(x),methods); ax.set_ylabel('count')
    ax.set_title('Dependency depth is not yet equal to executed φ/JVP work')
    ax.legend(); ax.grid(True,axis='y',alpha=.3); fig.tight_layout()
    fig.savefig(ns.plot_dir/'G2_DEPTH_VS_EXECUTED_WORK.png',dpi=180); plt.close(fig)

    fig=plt.figure(figsize=(7.2,4.8)); ax=fig.add_subplot(111)
    for method in methods:
        rows=sorted([r for r in d['stiff_linear_screen'] if r['method']==method],key=lambda r:abs(r['z']))
        ax.semilogy([abs(r['z']) for r in rows],[max(r['absolute_error'],1e-18) for r in rows],marker='o',label=method)
    ax.set_xlabel('|z| for y\' = z y over one unit step'); ax.set_ylabel('absolute error')
    ax.set_title('Stiff linear decay: exponential methods reproduce exp(z)')
    ax.grid(True,which='both',alpha=.3); ax.legend(); fig.tight_layout()
    fig.savefig(ns.plot_dir/'G2_STIFF_LINEAR_DECAY.png',dpi=180); plt.close(fig)

if __name__=='__main__': main()
