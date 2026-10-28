"""New diagnostic image from saved exact JSON; imports no scientific modules."""
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

ROOT=Path(__file__).resolve().parents[1]


def number(value):
    if isinstance(value,dict) and set(value)=={'numerator','denominator'}:
        return float(Fraction(int(value['numerator']),int(value['denominator'])))
    return float(value)


def main():
    source=ROOT/'results/first_suite_data.json'
    data=json.loads(source.read_text())
    if data['tests_run']!=10 or data['errors'] or data['failures']:
        raise ValueError('PASSING_TEN_GROUP_SAVED_RESULT_REQUIRED')
    d=data['diagnostics']
    feedback=d['hydrogen_feedback_curve'];scaling=d['quadratic_scaling_curve']
    plt.rcParams.update({'font.size':10,'axes.spines.right':False,'axes.spines.top':False})
    fig,axes=plt.subplots(1,2,figsize=(12,4.8),layout='constrained')
    x=[number(row['HII']) for row in feedback]
    axes[0].plot(x,[number(row['correct_J_HH']) for row in feedback],'o-',color='#1e5d9f',label='Correct feedback: −1 − 2 HII')
    axes[0].plot(x,[number(row['omitted_feedback_J_HH']) for row in feedback],'s--',color='#b24d2b',label='Hostile omission: −1 − HII')
    axes[0].set(xlabel='Hydrogen charged fraction HII',ylabel='Chemical derivative J_HH',title='H-only feedback omission is detected')
    axes[0].legend(fontsize=8.5);axes[0].grid(alpha=.2)
    s=[number(row['increment_scale']) for row in scaling]
    axes[1].loglog(s,[number(row['reference_abs_R_H']) for row in scaling],color='#28745d',lw=2,label='Exact quadratic reference: s² |R(d)|')
    axes[1].loglog(s,[number(row['abs_R_H']) for row in scaling],'o',mfc='white',mec='#1e5d9f',ms=7,label='Independent saved identity controls')
    axes[1].set(xlabel='Increment scale s',ylabel='Absolute H chemical remainder',title='R(s d) = s² R(d), frozen coefficients')
    axes[1].set_xticks(s,labels=[str(int(v)) for v in s]);axes[1].legend(fontsize=8.5);axes[1].grid(alpha=.2,which='both')
    fig.suptitle('Source-restricted chemical polynomial • diagnostic rates and synthetic weights',fontsize=12)
    fig.supxlabel('Exact rational diagnostic values • T_H = 1 − 2⁻⁵³ • no native callback, owner scale observation or ODE trajectory',fontsize=9)
    for ext in ('png','svg'):
        p=ROOT/'results'/('chemical_feedback_remainder.'+ext)
        if p.exists():raise ValueError('FIRST_FIGURE_ALREADY_EXISTS')
        fig.savefig(p,dpi=170)
    plt.close(fig)
    record={'status':'SAVED_JSON_ONLY_DIAGNOSTIC_FIGURE','input':'results/first_suite_data.json','input_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'source_scope':'Source-restricted chemical polynomial with diagnostic frozen inputs','physical_trajectory':False,'scientific_modules_imported':False,'outputs':['results/chemical_feedback_remainder.png','results/chemical_feedback_remainder.svg']}
    with (ROOT/'results/figure_data_source.json').open('x') as f:json.dump(record,f,indent=2);f.write('\n')
    print(json.dumps(record))


if __name__=='__main__':main()
